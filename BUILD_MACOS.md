# Building FluxTube on macOS (11 Big Sur & Newer)

## Target Support
- Minimum deployment target: **macOS 11.0 Big Sur** (`.cargo/config.toml` sets `MACOSX_DEPLOYMENT_TARGET=11.0` and `-mmacosx-version-min=11.0`)
- Architectures: `x86_64-apple-darwin` (Intel Macs) and `aarch64-apple-darwin` (Apple Silicon)

## Prerequisites
```bash
xcode-select --install
rustup target add x86_64-apple-darwin aarch64-apple-darwin
```

## Build & Verification
```bash
export MACOSX_DEPLOYMENT_TARGET=11.0

# Test workspace
cargo test --workspace

# Build Intel and Apple Silicon release binaries
cargo build --release --target x86_64-apple-darwin -p fluxtube-desktop
cargo build --release --target aarch64-apple-darwin -p fluxtube-desktop

# Create Universal Binary & .app / .dmg
./scripts/package_macos.sh
```

## macOS 11 Compatibility Notes
- **Iced 0.13 (`wgpu` Metal / `tiny-skia`)**: Uses Metal APIs available on macOS 10.14+ and falls back to `tiny-skia` software rendering on unsupported GPUs.
- **WKWebView (`wry`)**: Built into macOS 11 Big Sur with no external runtime dependencies.
- **Keychain (`keyring`)**: Uses standard Security.framework APIs compatible with macOS 11+.
- **Code Signing & Notarization**: For public distribution outside your local machine, sign the `.app` bundle with `codesign --deep --options runtime` using an Apple Developer ID certificate and submit the `.dmg` to `xcrun notarytool`.
- *Verification status*: macOS builds are configured in GitHub Actions (`macos-latest`); physical hardware execution on macOS 11.x Big Sur must be validated on an actual Big Sur host/VM before final public release.
