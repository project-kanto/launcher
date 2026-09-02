use std::{io::Read, path::PathBuf, time::Duration};

use kanto_release::{GameReleaseManifest, sha256};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use tauri::Manager;
use tauri_plugin_kanto_device::{DeviceCapabilities, GameRequest, InstalledGame, KantoDeviceExt};

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
    cached_android_original: bool,
    cached_ios_original: bool,
    supports_32_bit_apps: bool,
    can_install_apps: bool,
    installed_android: Option<InstalledGame>,
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
    let builder = reqwest::Client::builder().timeout(Duration::from_secs(timeout_seconds));
    #[cfg(target_os = "android")]
    let builder = {
        let mut roots = rustls::RootCertStore::empty();
        roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        builder.tls_backend_preconfigured(
            rustls::ClientConfig::builder()
                .with_root_certificates(roots)
                .with_no_client_auth(),
        )
    };
    builder
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

fn cached_original_path(app: &tauri::AppHandle, platform: &str) -> Result<PathBuf, String> {
    let extension = if platform == "android" { "apk" } else { "ipa" };
    Ok(app
        .path()
        .app_cache_dir()
        .map_err(|_| "Kanto couldn't open its private working folder.".to_owned())?
        .join(format!("kanto-original.{extension}")))
}

async fn cached_original(
    app: &tauri::AppHandle,
    platform: &str,
    manifest: &GameReleaseManifest,
) -> Option<Vec<u8>> {
    let bytes = read_local(
        cached_original_path(app, platform).ok()?,
        manifest.input_bytes,
    )
    .await
    .ok()??;
    (sha256(&bytes) == manifest.input_sha256).then_some(bytes)
}

fn android_package() -> &'static str {
    if API_BASE.contains("dev.kanto.ac") {
        "ac.kanto.client.dev"
    } else {
        "ac.kanto.client"
    }
}

fn android_state(app: &tauri::AppHandle) -> DeviceCapabilities {
    app.kanto_device()
        .capabilities(GameRequest {
            package_name: android_package().to_owned(),
        })
        .unwrap_or_default()
}

#[tauri::command]
fn load_android_state(app: tauri::AppHandle) -> DeviceCapabilities {
    android_state(&app)
}

#[tauri::command]
fn open_android_game(app: tauri::AppHandle) -> Result<(), String> {
    let response = app
        .kanto_device()
        .open_game(GameRequest {
            package_name: android_package().to_owned(),
        })
        .map_err(|_| "Kanto isn't installed yet.".to_owned())?;
    response
        .opened
        .then_some(())
        .ok_or_else(|| "Kanto couldn't open the game.".to_owned())
}

#[tauri::command]
fn open_android_install_settings(app: tauri::AppHandle) -> Result<(), String> {
    let response = app
        .kanto_device()
        .open_install_settings()
        .map_err(|_| "Kanto couldn't open Android's install permission.".to_owned())?;
    response
        .opened
        .then_some(())
        .ok_or_else(|| "Kanto couldn't open Android's install permission.".to_owned())
}

#[tauri::command]
async fn load_dashboard(app: tauri::AppHandle) -> Dashboard {
    let client = http_client(10);
    let android = release(&client, "android").await;
    let ios = release(&client, "ios").await;
    let cached_android_original = if let Some(manifest) = android.as_ref() {
        cached_original(&app, "android", manifest).await.is_some()
    } else {
        false
    };
    let cached_ios_original = if let Some(manifest) = ios.as_ref() {
        cached_original(&app, "ios", manifest).await.is_some()
    } else {
        false
    };
    let android_state = android_state(&app);
    Dashboard {
        host: std::env::consts::OS,
        environment: if API_BASE.contains("dev.kanto.ac") {
            "development"
        } else {
            "production"
        },
        server: get(&client, "/api/launcher/v1/status").await,
        android,
        ios,
        cached_android_original,
        cached_ios_original,
        supports_32_bit_apps: android_state.supports_32_bit_apps,
        can_install_apps: android_state.can_install_apps,
        installed_android: android_state.installed_game,
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
            "That’s the right file. You can continue."
        } else {
            "That file won’t work with Kanto. Choose the supported Pokémon GO file."
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
    let extension = if platform == "android" { "apk" } else { "ipa" };
    let directory = app
        .path()
        .app_cache_dir()
        .map_err(|_| "Kanto couldn't open its private working folder.".to_owned())?;
    let cached_path = cached_original_path(&app, platform)?;
    let source = if let Some(path) = source_path {
        read_local(path, manifest.input_bytes)
            .await?
            .ok_or_else(|| {
                "That file won’t work with Kanto. Choose the supported Pokémon GO file.".to_owned()
            })?
    } else if let Some(bytes) = cached_original(&app, platform, &manifest).await {
        bytes
    } else {
        let source_url = manifest.source_url.as_deref().ok_or_else(|| {
            "Download isn’t available right now. Choose your Pokémon GO file instead.".to_owned()
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
    let destination = directory.join(format!("kanto-prepared.{extension}"));
    let write_path = destination.clone();
    // ponytail: this is a rebuildable cache artifact; a retry safely replaces a partial write.
    tauri::async_runtime::spawn_blocking(move || {
        std::fs::create_dir_all(&directory)?;
        std::fs::write(cached_path, source)?;
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
            load_android_state,
            open_android_game,
            open_android_install_settings,
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
        load_android_state,
        open_android_game,
        open_android_install_settings,
        verify_original,
        prepare_release,
        install_prepared
    ]);
    builder
        .run(tauri::generate_context!())
        .expect("error while running Kanto Launcher");
}
