use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Mutex, mpsc},
    time::Duration,
};

use futures::future::join_all;
use idevice::usbmuxd::UsbmuxdConnection;
use keyring::Entry;
use plume_core::{
    AnisetteConfiguration, CertificateIdentity,
    auth::{Account, TwoFactorAction, TwoFactorMethod, TwoFactorRequest},
    developer::DeveloperSession,
};
use plume_store::{GsaAccount, account_from_session};
use plume_utils::{Device, Package, Signer, SignerMode, SignerOptions};
use serde::Serialize;
use tauri::{Emitter, Manager};

const SESSION_SERVICE: &str = "ac.kanto.launcher.apple-session";
const CERTIFICATE_SERVICE: &str = "ac.kanto.launcher.apple-certificate";
const SELECTED_ACCOUNT: &str = "selected";

pub(crate) struct IosState(pub Mutex<Option<mpsc::Sender<TwoFactorAction>>>);

#[derive(Serialize)]
pub(crate) struct IosDevice {
    name: String,
    udid: String,
}

#[derive(Serialize)]
pub(crate) struct IosSetup {
    devices: Vec<IosDevice>,
    signed_in_as: Option<String>,
}

#[derive(Clone, Serialize)]
struct TrustedPhone {
    id: u32,
    last_two_digits: String,
}

#[derive(Clone, Serialize)]
struct TwoFactorPrompt {
    method: &'static str,
    phones: Vec<TrustedPhone>,
}

#[derive(Clone, Serialize)]
struct InstallFinished {
    success: bool,
    message: String,
}

fn credential(service: &str, username: &str) -> Result<Entry, String> {
    Entry::new(service, username)
        .map_err(|_| "Kanto couldn't open your system credential store.".to_owned())
}

fn load_account() -> Result<GsaAccount, String> {
    let bytes = credential(SESSION_SERVICE, SELECTED_ACCOUNT)?
        .get_secret()
        .map_err(|_| "Sign in with your Apple ID first.".to_owned())?;
    serde_json::from_slice(&bytes)
        .map_err(|_| "Your saved Apple session is invalid. Sign in again.".to_owned())
}

fn save_account(account: &GsaAccount) -> Result<(), String> {
    let bytes = serde_json::to_vec(account)
        .map_err(|_| "Kanto couldn't secure your Apple session.".to_owned())?;
    credential(SESSION_SERVICE, SELECTED_ACCOUNT)?
        .set_secret(&bytes)
        .map_err(|_| "Kanto couldn't save your Apple session securely.".to_owned())
}

fn anisette(app: &tauri::AppHandle) -> Result<AnisetteConfiguration, String> {
    let path = app
        .path()
        .app_data_dir()
        .map_err(|_| "Kanto couldn't open its private settings folder.".to_owned())?
        .join("apple-services");
    fs::create_dir_all(&path).map_err(|_| "Kanto couldn't prepare Apple sign-in.".to_owned())?;
    Ok(AnisetteConfiguration::default().set_configuration_path(path))
}

async fn devices() -> Result<Vec<Device>, String> {
    let mut muxer = UsbmuxdConnection::default().await.map_err(|_| {
        "Kanto couldn't access Apple USB devices. Unlock your phone and reconnect it.".to_owned()
    })?;
    let found = muxer.get_devices().await.map_err(|_| {
        "Kanto couldn't read connected Apple devices. Reconnect the USB cable and retry.".to_owned()
    })?;
    Ok(join_all(found.into_iter().map(Device::new)).await)
}

fn prepared_ipa(app: &tauri::AppHandle, path: &Path) -> Result<(), String> {
    let expected = app
        .path()
        .app_cache_dir()
        .map_err(|_| "Kanto couldn't open its private working folder.".to_owned())?
        .join("kanto-prepared.ipa");
    if path != expected || !path.is_file() {
        return Err("The prepared iPhone build is missing. Prepare it again.".to_owned());
    }
    Ok(())
}

fn bundle_identifier(team_id: &str) -> Result<String, String> {
    let suffix: String = team_id
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect();
    if suffix.is_empty() {
        return Err("Apple didn't return a signing team for this account.".to_owned());
    }
    Ok(format!("ac.kanto.game.{suffix}"))
}

fn certificate_key_path(root: &Path, team_id: &str) -> PathBuf {
    root.join("keys").join(team_id).join("key.pem")
}

fn restore_certificate_key(root: &Path, team_id: &str) -> Result<(), String> {
    let Ok(secret) = credential(CERTIFICATE_SERVICE, team_id)?.get_secret() else {
        return Ok(());
    };
    let path = certificate_key_path(root, team_id);
    fs::create_dir_all(path.parent().expect("the certificate path has a parent"))
        .map_err(|_| "Kanto couldn't prepare its temporary signing key.".to_owned())?;
    fs::write(&path, secret)
        .map_err(|_| "Kanto couldn't restore its temporary signing key.".to_owned())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))
            .map_err(|_| "Kanto couldn't protect its temporary signing key.".to_owned())?;
    }
    Ok(())
}

fn save_certificate_key(root: &Path, team_id: &str) -> Result<(), String> {
    let secret = fs::read(certificate_key_path(root, team_id))
        .map_err(|_| "Kanto couldn't secure the new signing key.".to_owned())?;
    credential(CERTIFICATE_SERVICE, team_id)?
        .set_secret(&secret)
        .map_err(|_| "Kanto couldn't save the signing key securely.".to_owned())
}

#[tauri::command]
pub(crate) async fn load_ios_setup() -> Result<IosSetup, String> {
    let signed_in_as = load_account().ok().map(|account| account.email().clone());
    let devices = devices()
        .await?
        .into_iter()
        .map(|device| IosDevice {
            name: if device.name.is_empty() {
                "Connected iPhone or iPad".to_owned()
            } else {
                device.name
            },
            udid: device.udid,
        })
        .collect();
    Ok(IosSetup {
        devices,
        signed_in_as,
    })
}

#[tauri::command]
pub(crate) async fn apple_login(
    app: tauri::AppHandle,
    state: tauri::State<'_, IosState>,
    email: String,
    password: String,
) -> Result<String, String> {
    if email.trim().is_empty() || password.is_empty() {
        return Err("Enter your Apple ID and password.".to_owned());
    }
    let email = email.trim().to_owned();
    let login_email = email.clone();
    let login_password = password;
    let prompt_app = app.clone();
    let state_handle = state.inner();
    let account = Account::login(
        move || Ok((login_email.clone(), login_password.clone())),
        move |request: TwoFactorRequest| {
            let (sender, receiver) = mpsc::channel();
            *state_handle
                .0
                .lock()
                .map_err(|_| "Apple sign-in was interrupted.".to_owned())? = Some(sender);
            let prompt = TwoFactorPrompt {
                method: match request.method {
                    TwoFactorMethod::Device => "device",
                    TwoFactorMethod::Sms => "sms",
                },
                phones: request
                    .trusted_phone_numbers
                    .into_iter()
                    .map(|phone| TrustedPhone {
                        id: phone.id,
                        last_two_digits: phone.last_two_digits,
                    })
                    .collect(),
            };
            prompt_app
                .emit("apple-2fa-required", prompt)
                .map_err(|_| "Kanto couldn't show the verification prompt.".to_owned())?;
            receiver
                .recv_timeout(Duration::from_secs(300))
                .map_err(|_| "Apple verification timed out. Sign in again.".to_owned())
        },
        anisette(&app)?,
    )
    .await
    .map_err(|_| "Apple sign-in failed. Check your details and verification code.".to_owned())?;
    let account = account_from_session(email.clone(), account)
        .await
        .map_err(|_| {
            "Apple signed in, but Kanto couldn't open the developer account.".to_owned()
        })?;
    save_account(&account)?;
    if let Ok(mut pending) = state.0.lock() {
        *pending = None;
    }
    Ok(email)
}

#[tauri::command]
pub(crate) fn respond_apple_2fa(
    state: tauri::State<'_, IosState>,
    code: Option<String>,
    phone_id: Option<u32>,
) -> Result<(), String> {
    let action = match (code, phone_id) {
        (Some(code), None) if !code.trim().is_empty() => {
            TwoFactorAction::SubmitCode(code.trim().to_owned())
        }
        (None, Some(id)) => TwoFactorAction::SendSms(id),
        _ => return Err("Enter the verification code Apple sent you.".to_owned()),
    };
    let sender = state
        .0
        .lock()
        .map_err(|_| "Apple verification was interrupted.".to_owned())?
        .take()
        .ok_or_else(|| "Apple isn't waiting for a verification code.".to_owned())?;
    sender
        .send(action)
        .map_err(|_| "Apple verification timed out. Sign in again.".to_owned())
}

async fn sign_and_install(
    app: tauri::AppHandle,
    path: PathBuf,
    udid: String,
) -> Result<(), String> {
    prepared_ipa(&app, &path)?;
    let account = load_account()?;
    let team_id = account.team_id().clone();
    let bundle_id = bundle_identifier(&team_id)?;
    let device = devices()
        .await?
        .into_iter()
        .find(|device| device.udid == udid)
        .ok_or_else(|| {
            "That iPhone is no longer connected. Reconnect it and refresh.".to_owned()
        })?;
    let session = DeveloperSession::new(
        account.adsid().clone(),
        account.xcode_gs_token().clone(),
        anisette(&app)?,
    )
    .await
    .map_err(|_| "Your Apple session expired. Sign in again.".to_owned())?;
    let signing_root = tempfile::tempdir()
        .map_err(|_| "Kanto couldn't prepare temporary signing files.".to_owned())?;
    restore_certificate_key(signing_root.path(), &team_id)?;

    app.emit("ios-install-progress", "Preparing signing certificate…")
        .ok();
    let mut refuse_certificate_reset = || false;
    let identity = CertificateIdentity::new_with_session(
        &session,
        signing_root.path().to_path_buf(),
        Some("Kanto Launcher".to_owned()),
        &team_id,
        false,
        Some(&mut refuse_certificate_reset),
    )
    .await
    .map_err(|_| {
        "Apple couldn't create a signing certificate. Your account may be at its certificate limit."
            .to_owned()
    })?;
    save_certificate_key(signing_root.path(), &team_id)?;

    let package = Package::new(path)
        .map_err(|_| "The prepared iPhone build couldn't be opened.".to_owned())?;
    let result = async {
        let bundle = package
            .get_package_bundle()
            .map_err(|_| "The prepared iPhone build is invalid.".to_owned())?;
        let options = SignerOptions {
            custom_identifier: Some(bundle_id),
            custom_name: Some("Kanto".to_owned()),
            mode: SignerMode::Pem,
            ..Default::default()
        };
        let mut signer = Signer::new(Some(identity), options);
        app.emit("ios-install-progress", "Registering your iPhone…")
            .ok();
        signer
            .modify_bundle(&bundle, &Some(team_id.clone()))
            .await
            .map_err(|_| "Kanto couldn't prepare the app for your Apple account.".to_owned())?;
        session
            .qh_ensure_device(&team_id, &device.name, &device.udid)
            .await
            .map_err(|_| {
                "Apple wouldn't register this iPhone. Check your developer account.".to_owned()
            })?;
        signer
            .register_bundle(&bundle, &session, &team_id, false)
            .await
            .map_err(|_| "Apple wouldn't create Kanto's provisioning profile.".to_owned())?;
        app.emit("ios-install-progress", "Signing Kanto…").ok();
        signer
            .sign_bundle(&bundle)
            .await
            .map_err(|_| "Kanto couldn't sign the iPhone app.".to_owned())?;
        app.emit("ios-install-progress", "Installing on your iPhone…")
            .ok();
        let progress_app = app.clone();
        device
            .install_app(bundle.bundle_dir(), move |progress| {
                let progress_app = progress_app.clone();
                async move {
                    progress_app
                        .emit(
                            "ios-install-progress",
                            format!("Installing on your iPhone… {progress}%"),
                        )
                        .ok();
                }
            })
            .await
            .map_err(|_| {
                "Installation failed. Keep the phone unlocked, tap Trust, and retry.".to_owned()
            })?;
        Ok(())
    }
    .await;
    package.remove_package_stage();
    result
}

#[tauri::command]
pub(crate) fn sign_and_install_ios(
    app: tauri::AppHandle,
    path: PathBuf,
    udid: String,
) -> Result<(), String> {
    prepared_ipa(&app, &path)?;
    let event_app = app.clone();
    std::thread::Builder::new()
        .name("kanto-ios-installer".to_owned())
        .spawn(move || {
            let result = tauri::async_runtime::block_on(sign_and_install(app, path, udid));
            let finished = match result {
                Ok(()) => InstallFinished {
                    success: true,
                    message: "Kanto is installed. If iOS asks, enable Developer Mode or trust your Apple ID under Settings > General > VPN & Device Management, then open Kanto."
                        .to_owned(),
                },
                Err(message) => InstallFinished {
                    success: false,
                    message,
                },
            };
            event_app.emit("ios-install-finished", finished).ok();
        })
        .map_err(|_| "Kanto couldn't start the iPhone installer.".to_owned())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::bundle_identifier;

    #[test]
    fn signing_bundle_id_only_uses_apple_safe_team_characters() {
        assert_eq!(
            bundle_identifier("AB-12_CD").unwrap(),
            "ac.kanto.game.ab12cd"
        );
        assert!(bundle_identifier("---").is_err());
    }
}
