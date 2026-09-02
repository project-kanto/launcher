use serde::de::DeserializeOwned;
use tauri::{AppHandle, Runtime, plugin::PluginApi};

use crate::models::*;

pub fn init<R: Runtime, C: DeserializeOwned>(
    app: &AppHandle<R>,
    _api: PluginApi<R, C>,
) -> crate::Result<KantoDevice<R>> {
    Ok(KantoDevice(app.clone()))
}

/// Access to the kanto-device APIs.
pub struct KantoDevice<R: Runtime>(AppHandle<R>);

impl<R: Runtime> KantoDevice<R> {
    pub fn capabilities(&self, _request: GameRequest) -> crate::Result<DeviceCapabilities> {
        Ok(DeviceCapabilities {
            supports_32_bit_apps: true,
            can_install_apps: true,
            installed_game: None,
        })
    }

    pub fn install(&self, _request: InstallRequest) -> crate::Result<InstallResponse> {
        Ok(InstallResponse::default())
    }

    pub fn open_game(&self, _request: GameRequest) -> crate::Result<ActionResponse> {
        Ok(ActionResponse::default())
    }

    pub fn open_install_settings(&self) -> crate::Result<ActionResponse> {
        Ok(ActionResponse::default())
    }
}
