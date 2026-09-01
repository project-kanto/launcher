# Kanto Launcher

Kanto Launcher is the guided Android and desktop installer for Kanto. It will download or accept
the exact supported original client, verify it, apply Kanto's published changes locally, verify the
result, and guide the ordinary signed installation onto the player's phone.

This repository is under active development. The shared Tauri application builds as a macOS app and
an Android APK, reads Kanto's release/status contract, downloads or verifies an exact original APK
or IPA, and applies the verified Kanto delta locally. The Android build opens the system installer.
The desktop build detects connected iPhones, signs through Apple, and installs over USB. Windows and
Linux use the same desktop shell but still need physical-device release testing.

Kanto Launcher does not contain Pokémon GO, signing credentials, certificates, provisioning
profiles, device identifiers, or completed patched clients. Keep all such material out of Git.

## Development

```sh
cargo test --workspace
cd apps/launcher
npm install
npm run build
npm run tauri build
```

The desktop signing/device integration pins narrowly scoped crates from Impactor rather than forking
its unrelated UI. Apple sessions and private signing keys are kept in the operating system credential
store; passwords are held only for the active login attempt. See [THIRD_PARTY.md](THIRD_PARTY.md).

## Licence

Kanto Launcher is licensed under GPL-3.0-only. Third-party components retain their own licences.
