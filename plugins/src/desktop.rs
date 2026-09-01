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
    pub fn capabilities(&self) -> crate::Result<DeviceCapabilities> {
        Ok(DeviceCapabilities {
            supports_32_bit_apps: true,
        })
    }

    pub fn install(&self, _request: InstallRequest) -> crate::Result<InstallResponse> {
        Ok(InstallResponse::default())
    }
}
