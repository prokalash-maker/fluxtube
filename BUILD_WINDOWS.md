# Building FluxTube on Windows (10 / 11 x64)

## Prerequisites
- Rust toolchain 1.80+ (`rustup-init.exe`)
- **MSVC target (Recommended)**: Visual Studio Build Tools with C++ desktop workload
- **GNU target**: MinGW-w64 / WinLibs (`x86_64-pc-windows-gnu`)
- Microsoft Edge WebView2 Runtime (pre-installed on Windows 11 and modern Windows 10)

## Build & Run
```powershell
# Run workspace tests
cargo test --workspace

# Run E2E offline test suites
cargo test --manifest-path tests/Cargo.toml

# Run desktop smoke verification
cargo run -p fluxtube-desktop -- --smoke-test

# Launch GUI
cargo run -p fluxtube-desktop

# Release build (LTO, single codegen unit, stripped)
cargo build --release -p fluxtube-desktop
```

## Packaging (Portable & Installer)
Run `scripts/package_windows.ps1` to generate a portable `.zip` bundle containing `fluxtube.exe`, `.env.example`, and documentation without requiring administrator rights.
