use serde::de::DeserializeOwned;
use tauri::{
    AppHandle, Runtime,
    plugin::{PluginApi, PluginHandle},
};

use crate::models::*;

#[cfg(target_os = "ios")]
tauri::ios_plugin_binding!(init_plugin_kanto_device);

// initializes the Kotlin or Swift plugin classes
pub fn init<R: Runtime, C: DeserializeOwned>(
    _app: &AppHandle<R>,
    api: PluginApi<R, C>,
) -> crate::Result<KantoDevice<R>> {
    #[cfg(target_os = "android")]
    let handle = api.register_android_plugin("ac.kanto.launcher.device", "KantoDevicePlugin")?;
    #[cfg(target_os = "ios")]
    let handle = api.register_ios_plugin(init_plugin_kanto_device)?;
    Ok(KantoDevice(handle))
}

/// Access to the kanto-device APIs.
pub struct KantoDevice<R: Runtime>(PluginHandle<R>);

impl<R: Runtime> KantoDevice<R> {
    pub fn capabilities(&self, request: GameRequest) -> crate::Result<DeviceCapabilities> {
        self.0
            .run_mobile_plugin("capabilities", request)
            .map_err(Into::into)
    }

    pub fn install(&self, request: InstallRequest) -> crate::Result<InstallResponse> {
        self.0
            .run_mobile_plugin("install", request)
            .map_err(Into::into)
    }

    pub fn open_game(&self, request: GameRequest) -> crate::Result<ActionResponse> {
        self.0
            .run_mobile_plugin("openGame", request)
            .map_err(Into::into)
    }

    pub fn open_install_settings(&self) -> crate::Result<ActionResponse> {
        self.0
            .run_mobile_plugin("openInstallSettings", ())
            .map_err(Into::into)
    }
}
