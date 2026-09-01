use std::{io::Read, path::PathBuf, time::Duration};

use kanto_release::{GameReleaseManifest, sha256};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use tauri::Manager;
use tauri_plugin_kanto_device::KantoDeviceExt;

#[cfg(desktop)]
mod ios;

const API_BASE: &str = match option_env!("KANTO_LAUNCHER_API_BASE") {
    Some(base) => base,
    None => "https://kanto.ac",
};
const MAX_ARTIFACT_BYTES: u64 = 256 * 1024 * 1024;

#[derive(Debug, Deserialize, Serialize)]
struct ServerStatus {
    status: String,
    version: String,
    updated_at: String,
}

#[derive(Serialize)]
struct Dashboard {
    host: &'static str,
    environment: &'static str,
    server: Option<ServerStatus>,
    android: Option<GameReleaseManifest>,
    ios: Option<GameReleaseManifest>,
    supports_32_bit_apps: bool,
}

#[derive(Serialize)]
struct SourceCheck {
    valid: bool,
    message: &'static str,
}

#[derive(Serialize)]
struct PreparedBuild {
    path: PathBuf,
    release_version: String,
}

async fn get<T: DeserializeOwned>(client: &reqwest::Client, path: &str) -> Option<T> {
    client
        .get(format!("{}{}", API_BASE.trim_end_matches('/'), path))
        .send()
        .await
        .ok()?
        .error_for_status()
        .ok()?
        .json()
        .await
        .ok()
}

fn http_client(timeout_seconds: u64) -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_seconds))
        .build()
        .expect("the static HTTP client configuration is valid")
}

async fn release(client: &reqwest::Client, platform: &str) -> Option<GameReleaseManifest> {
    if platform != "android" && platform != "ios" {
        return None;
    }
    let manifest: GameReleaseManifest = get(
        client,
        &format!("/api/launcher/v1/game/{platform}/manifest.json"),
    )
    .await?;
    manifest.validate().ok()?;
    Some(manifest)
}

async fn download(client: &reqwest::Client, url: &str, expected: u64) -> Result<Vec<u8>, String> {
    // ponytail: exact 0.35 clients are about 60 MB; raise this only for a verified larger profile.
    if expected > MAX_ARTIFACT_BYTES {
        return Err("Kanto refused an unexpectedly large download.".to_owned());
    }
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|_| "The download failed. Check your connection and retry.".to_owned())?
        .error_for_status()
        .map_err(|_| "Kanto couldn't download that file right now.".to_owned())?;
    if response
        .content_length()
        .is_some_and(|size| size != expected)
    {
        return Err("The downloaded file was the wrong size. Nothing was installed.".to_owned());
    }
    let bytes = response.bytes().await.map_err(|_| {
        "The download stopped early. Retry when your connection is stable.".to_owned()
    })?;
    if bytes.len() as u64 != expected {
        return Err("The downloaded file was incomplete. Nothing was installed.".to_owned());
    }
    Ok(bytes.to_vec())
}

async fn read_local(path: PathBuf, expected: u64) -> Result<Option<Vec<u8>>, String> {
    tauri::async_runtime::spawn_blocking(move || -> std::io::Result<Option<Vec<u8>>> {
        if expected > MAX_ARTIFACT_BYTES {
            return Ok(None);
        }
        let file = std::fs::File::open(path)?;
        if file.metadata()?.len() != expected {
            return Ok(None);
        }
        let mut bytes = Vec::with_capacity(expected as usize);
        file.take(expected + 1).read_to_end(&mut bytes)?;
        Ok((bytes.len() as u64 == expected).then_some(bytes))
    })
    .await
    .map_err(|_| "Kanto couldn't check that file. Retry in a moment.".to_owned())?
    .map_err(|_| "Kanto couldn't read that file. Choose it again and retry.".to_owned())
}

#[tauri::command]
async fn load_dashboard(app: tauri::AppHandle) -> Dashboard {
    let client = http_client(10);
    Dashboard {
        host: std::env::consts::OS,
        environment: if API_BASE.contains("dev.kanto.ac") {
            "development"
        } else {
            "production"
        },
        server: get(&client, "/api/launcher/v1/status").await,
        android: release(&client, "android").await,
        ios: release(&client, "ios").await,
        supports_32_bit_apps: app
            .kanto_device()
            .capabilities()
            .map(|capabilities| capabilities.supports_32_bit_apps)
            .unwrap_or(false),
    }
}

#[tauri::command]
fn install_prepared(
    app: tauri::AppHandle,
    path: PathBuf,
) -> Result<tauri_plugin_kanto_device::InstallResponse, String> {
    let expected = app
        .path()
        .app_cache_dir()
        .map_err(|_| "Kanto couldn't open its private working folder.".to_owned())?
        .join("kanto-prepared.apk");
    if path != expected || !path.is_file() {
        return Err("The prepared Android build is missing. Prepare it again.".to_owned());
    }
    app.kanto_device()
        .install(tauri_plugin_kanto_device::InstallRequest { path })
        .map_err(|_| "Kanto couldn't open the Android installer.".to_owned())
}

#[tauri::command]
async fn verify_original(path: PathBuf, platform: &str) -> Result<SourceCheck, String> {
    let client = http_client(10);
    let manifest = release(&client, platform)
        .await
        .ok_or_else(|| "No Kanto build is currently available for that device.".to_owned())?;
    let bytes = read_local(path, manifest.input_bytes).await?;
    let valid = bytes.is_some_and(|bytes| sha256(&bytes) == manifest.input_sha256);
    Ok(SourceCheck {
        valid,
        message: if valid {
            "Original game verified."
        } else {
            "That isn't the supported Pokémon GO 0.35.0 file."
        },
    })
}

#[tauri::command]
async fn prepare_release(
    app: tauri::AppHandle,
    platform: &str,
    source_path: Option<PathBuf>,
) -> Result<PreparedBuild, String> {
    let client = http_client(300);
    let manifest = release(&client, platform)
        .await
        .ok_or_else(|| "No Kanto build is currently available for that device.".to_owned())?;
    let source = if let Some(path) = source_path {
        read_local(path, manifest.input_bytes)
            .await?
            .ok_or_else(|| "That isn't the supported Pokémon GO 0.35.0 file.".to_owned())?
    } else {
        let source_url = manifest.source_url.as_deref().ok_or_else(|| {
            "Automatic download isn't available for this build. Choose the original file instead."
                .to_owned()
        })?;
        download(&client, source_url, manifest.input_bytes).await?
    };
    let delta_path = manifest
        .delta_url
        .as_deref()
        .filter(|path| path.starts_with('/'))
        .ok_or_else(|| "Kanto received an invalid update address.".to_owned())?;
    let delta_url = format!("{}{}", API_BASE.trim_end_matches('/'), delta_path);
    let delta = download(&client, &delta_url, manifest.delta_bytes).await?;
    let output = kanto_release::apply_verified(&source, &delta, &manifest)
        .map_err(|error| format!("Kanto refused the files: {error}"))?;
    let extension = if platform == "android" { "apk" } else { "ipa" };
    let directory = app
        .path()
        .app_cache_dir()
        .map_err(|_| "Kanto couldn't open its private working folder.".to_owned())?;
    let destination = directory.join(format!("kanto-prepared.{extension}"));
    let write_path = destination.clone();
    // ponytail: this is a rebuildable cache artifact; a retry safely replaces a partial write.
    tauri::async_runtime::spawn_blocking(move || {
        std::fs::create_dir_all(&directory)?;
        std::fs::write(write_path, output)
    })
    .await
    .map_err(|_| "Kanto couldn't finish saving the game. Retry in a moment.".to_owned())?
    .map_err(|_| "Kanto couldn't save the prepared game.".to_owned())?;
    Ok(PreparedBuild {
        path: destination,
        release_version: manifest
            .release_version
            .unwrap_or_else(|| "unknown".to_owned()),
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(desktop)]
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("the TLS provider is only installed once at startup");
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_kanto_device::init())
        .plugin(tauri_plugin_dialog::init());
    #[cfg(desktop)]
    let builder = builder
        .manage(ios::IosState(std::sync::Mutex::new(None)))
        .invoke_handler(tauri::generate_handler![
            load_dashboard,
            verify_original,
            prepare_release,
            install_prepared,
            ios::load_ios_setup,
            ios::apple_login,
            ios::respond_apple_2fa,
            ios::sign_and_install_ios
        ]);
    #[cfg(mobile)]
    let builder = builder.invoke_handler(tauri::generate_handler![
        load_dashboard,
        verify_original,
        prepare_release,
        install_prepared
    ]);
    builder
        .run(tauri::generate_context!())
        .expect("error while running Kanto Launcher");
}
