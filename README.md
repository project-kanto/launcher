# Kanto Launcher

Kanto Launcher is the guided Android and desktop installer for Kanto. It will download or accept
the exact supported original client, verify it, apply Kanto's published changes locally, verify the
result, and guide the ordinary signed installation onto the player's phone.

This repository is under active development. It currently contains the shared, tested release and
delta contract used by the desktop application. The Android application will implement the same
fixture so both platforms reject the same bad inputs and produce the same verified output.

Kanto Launcher does not contain Pokémon GO, signing credentials, certificates, provisioning
profiles, device identifiers, or completed patched clients. Keep all such material out of Git.

## Development

```sh
cargo test --workspace
```

The desktop signing/device integration will reuse narrowly scoped crates from Impactor rather than
forking its unrelated UI. See [THIRD_PARTY.md](THIRD_PARTY.md).

## Licence

Kanto Launcher is licensed under GPL-3.0-only. Third-party components retain their own licences.

