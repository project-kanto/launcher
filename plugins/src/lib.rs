use tauri::{
    Manager, Runtime,
    plugin::{Builder, TauriPlugin},
};

pub use models::*;

#[cfg(desktop)]
mod desktop;
#[cfg(mobile)]
mod mobile;

mod error;
mod models;

pub use error::{Error, Result};

#[cfg(desktop)]
use desktop::KantoDevice;
#[cfg(mobile)]
use mobile::KantoDevice;

/// Extensions to [`tauri::App`], [`tauri::AppHandle`] and [`tauri::Window`] to access the kanto-device APIs.
pub trait KantoDeviceExt<R: Runtime> {
    fn kanto_device(&self) -> &KantoDevice<R>;
}

impl<R: Runtime, T: Manager<R>> crate::KantoDeviceExt<R> for T {
    fn kanto_device(&self) -> &KantoDevice<R> {
        self.state::<KantoDevice<R>>().inner()
    }
}

/// Initializes the plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("kanto-device")
        .setup(|app, api| {
            #[cfg(mobile)]
            let kanto_device = mobile::init(app, api)?;
            #[cfg(desktop)]
            let kanto_device = desktop::init(app, api)?;
            app.manage(kanto_device);
            Ok(())
        })
        .build()
}
