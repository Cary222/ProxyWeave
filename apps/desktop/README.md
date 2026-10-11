# ProxyWeave desktop baseline

Direct extraction from Quotio Desktop v0.7.7 (`21d75d08b38a23f97fbd534a1768a829cb147f2c`, MIT).
Internal Rust crate names remain upstream names; application ID is `com.proxyweave.desktop`.
See `../../docs/verification/PHASE0.md` for provenance, adaptations and verification limits.

## Windows development

Node.js 22+, npm and a Rust/MSVC toolchain with WebView2 are required.
From the repository root:

```cmd
set "RUSTUP_HOME=D:\build-tools\rustup"
set "CARGO_HOME=D:\build-tools\cargo"
set "PATH=D:\build-tools\cargo\bin;%PATH%"
npm run desktop:install
npm run desktop:dev
```

## Embedded standalone executable (no installer)

```cmd
set "CARGO_BUILD_JOBS=1"
npm --prefix apps/desktop run tauri -- build --debug --no-bundle
```

The executable is `target/debug/quotio-desktop.exe` unless `CARGO_TARGET_DIR` is set.
Use the **Tauri build command** to embed `apps/desktop/dist`. Plain `cargo build` in debug mode uses `devUrl` and needs the matching Vite server; it is not standalone runtime acceptance.
Release executable: `npm run desktop:build` (installer/signing not part of Phase 0).

```cmd
cargo check --workspace --locked -j 1
cargo test --workspace --locked -j 1 -- --test-threads=1
npm run test:frontend
```

Native close/Alt+F4 hides the window. The extracted custom close dialog offers cancel, tray or explicit exit. Right-click tray menu offers restore and exit; left-click toggles the small shell panel. A second launch restores the existing main window.
No provider, updater, gateway, marketplace, protocol conversion or IDE configuration runs in this baseline. macOS/Linux are not validated.
