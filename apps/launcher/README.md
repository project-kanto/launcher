# Kanto Launcher app

One responsive Tauri shell builds the desktop iOS installer and the Android launcher. Platform-only
device installation and signing code stays behind the shared Rust commands.

```sh
npm install
npm run build
npm run tauri dev
```

Set `KANTO_LAUNCHER_API_BASE` while compiling to point a test build at another Kanto deployment.
