use std::{path::PathBuf, time::Duration};

use kanto_release::{GameReleaseManifest, sha256};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

const API_BASE: &str = match option_env!("KANTO_LAUNCHER_API_BASE") {
    Some(base) => base,
    None => "https://kanto.ac",
};

#[derive(Debug, Deserialize, Serialize)]
struct ServerStatus {
    status: String,
    version: String,
    updated_at: String,
}

#[derive(Serialize)]
struct Dashboard {
    host: &'static str,
    server: Option<ServerStatus>,
    android: Option<GameReleaseManifest>,
    ios: Option<GameReleaseManifest>,
}

#[derive(Serialize)]
struct SourceCheck {
    valid: bool,
    message: &'static str,
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

fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
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

#[tauri::command]
async fn load_dashboard() -> Dashboard {
    let client = http_client();
    Dashboard {
        host: std::env::consts::OS,
        server: get(&client, "/api/launcher/v1/status").await,
        android: release(&client, "android").await,
        ios: release(&client, "ios").await,
    }
}

#[tauri::command]
async fn verify_original(path: PathBuf, platform: &str) -> Result<SourceCheck, String> {
    let client = http_client();
    let manifest = release(&client, platform)
        .await
        .ok_or_else(|| "No Kanto build is currently available for that device.".to_owned())?;
    let bytes = tauri::async_runtime::spawn_blocking(move || std::fs::read(path))
        .await
        .map_err(|_| "Kanto couldn't check that file. Retry in a moment.".to_owned())?
        .map_err(|_| "Kanto couldn't read that file. Choose it again and retry.".to_owned())?;
    let valid =
        bytes.len() as u64 == manifest.input_bytes && sha256(&bytes) == manifest.input_sha256;
    Ok(SourceCheck {
        valid,
        message: if valid {
            "Original game verified."
        } else {
            "That isn't the supported Pokémon GO 0.35.0 file."
        },
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![load_dashboard, verify_original])
        .run(tauri::generate_context!())
        .expect("error while running Kanto Launcher");
}
