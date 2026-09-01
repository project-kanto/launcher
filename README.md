# Kanto Launcher

Kanto Launcher is the guided Android and desktop installer for Kanto. It will download or accept
the exact supported original client, verify it, apply Kanto's published changes locally, verify the
result, and guide the ordinary signed installation onto the player's phone.

This repository is under active development. The shared Tauri application already builds as a
macOS app and an Android APK, reads Kanto's release/status contract, and verifies a selected original
APK or IPA locally. Windows and Linux use the same desktop shell.

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

The desktop signing/device integration will reuse narrowly scoped crates from Impactor rather than
forking its unrelated UI. See [THIRD_PARTY.md](THIRD_PARTY.md).

## Licence

Kanto Launcher is licensed under GPL-3.0-only. Third-party components retain their own licences.
