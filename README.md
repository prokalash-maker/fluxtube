# FluxTube 📺⚡

> Lightweight, native desktop YouTube client in Rust, built specifically for older MacBooks, iMacs, and low-resource Windows PCs.

[![Rust](https://img.shields.io/badge/rust-1.80%2B-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)
[![Target](https://img.shields.io/badge/macOS-11.0%2B%20Big%20Sur-lightgrey.svg)]()
[![Target](https://img.shields.io/badge/Windows-10%2F11%20x64-lightgrey.svg)]()

---

## 🎯 The Mission

Modern web browsers have become resource-intensive runtimes. Opening YouTube in Chrome or Safari on an older Mac or PC (Intel Core 2 Duo / early Core i3/i5, 4GB RAM, integrated Intel HD Graphics) often results in heavy thermal throttling, high fan noise, dropped frames, and memory consumption exceeding 850 MB to 1.5 GB.

**FluxTube** is designed to eliminate browser overhead:
- **Native Rust Performance**: Zero Electron, zero CEF, zero bundled Chromium for the GUI.
- **Ultra-low Footprint**: Target RAM of 60–120 MB in idle, ~200 MB during navigation.
- **Strict Hardware Acceleration**: Force H.264 video decoding (`h264ify` approach) to leverage hardware VPUs on legacy Intel HD Graphics, avoiding heavy software VP9/AV1 decoding.
- **macOS 11 Big Sur & Windows 10/11 Support**: Explicit target support for older Intel MacBooks (2013-2020), Apple Silicon, and Windows PCs with HDD/slow SSDs.
- **Official Compliance & Safety**: Google OAuth 2.0 with PKCE S256, secure OS credential storage (Keychain & Windows Credential Manager), YouTube Data API v3 quota manager.

---

## 🏗️ Architecture (Cargo Workspace)

FluxTube is structured as a modular Cargo workspace with strictly decoupled concerns:

```
apps/
  desktop/                   # Main desktop application UI & event loop
crates/
  core/                      # Pure domain models (Video, Channel, Playlist, Comments)
  storage/                   # SQLite database (WAL mode, migrations, indexing, history)
  security/                  # OS Keychain & Credential Manager with zeroize memory protection
  settings/                  # Performance profiles (Ultra Lite, Balanced, High Quality, Custom)
  localization/              # Internationalization (English, Ukrainian, Russian)
  diagnostics/               # Real-time hardware telemetry (RAM, CPU, API quotas, decoders)
  network/                   # Tokio/Reqwest HTTP client with in-flight deduplication & TLS
  auth/                      # Google OAuth 2.0 PKCE S256, loopback redirect, token rotation
  youtube-api/               # YouTube Data API v3 client with quota management
  player/                    # PlayerBackend trait abstraction & playback state machine
  player-embed/              # Isolated system WebView player (wry) with h264ify injection
  player-native/             # Extensible native video backend
  cache/                     # Two-tier LRU memory & disk thumbnail cache with off-thread decoding
tests/
  e2e/                       # 4-tier opaque-box test harness & mock client
  fixtures/                  # YouTube v3 canonical JSON response fixtures
```

---

## ⚡ Performance Profiles

FluxTube provides 4 configurable performance profiles:

| Profile | Target RAM | Video Quality | Thumbnail Cache | Animations | Comments | List Virtualization |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: |
| **Ultra Lite** (2–4 GB RAM) | ~60–120 MB | 360p–480p (H.264) | 64–128 MB | Off | Off (lazy) | Enabled |
| **Balanced** (Default) | ~150–200 MB | 720p (H.264) | 256 MB | Minimal | Lazy load | Enabled |
| **High Quality** (8+ GB RAM) | ~300 MB | 1080p | 512 MB | Smooth | Enabled | Enabled |
| **Custom** | User-defined | User-defined | User-defined | Configurable | Configurable | Configurable |

---

## 🔒 Security & Privacy

- **OAuth 2.0 PKCE S256**: No webview-based Google password input. The official system browser is used with a loopback redirect on `127.0.0.1`.
- **System Credential Storage**: Refresh tokens are never stored in plain text, JSON, or logs. Tokens reside in **macOS Keychain** or **Windows Credential Manager / DPAPI**.
- **Memory Scrubbing**: Sensitive keys and token buffers are wiped on drop using the `zeroize` crate.
- **Zero Telemetry**: No third-party trackers, no usage analytics, no advertising SDKs.
- **Private Mode**: Support for private viewing without persisting history to the local SQLite database.

---

## 🛠️ Building & Testing

### Prerequisites
- **Rust 1.80+**: Install via [rustup.rs](https://rustup.rs)
- **Windows**: Visual Studio C++ Build Tools or MinGW
- **macOS**: Xcode Command Line Tools (`xcode-select --install`)

### Build Workspace
```bash
# Check workspace
cargo check --workspace

# Run all unit and crate tests
cargo test --workspace

# Run end-to-end and mock test suite
cargo test --test tier1_feature_coverage
```

### Setup Google Cloud Credentials (Optional for Live API)
1. Copy `.env.example` to `.env`:
   ```bash
   cp .env.example .env
   ```
2. Fill in your `GOOGLE_CLIENT_ID` and `YOUTUBE_API_KEY`.
3. If no credentials are provided, FluxTube automatically uses the built-in offline mock fixtures for testing and development.

---

## 📜 License

Licensed under either of:
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
