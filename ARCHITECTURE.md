# FluxTube Architecture

## 1. Design Philosophy

FluxTube is a native desktop YouTube client engineered for resource-constrained hardware:
- Older Intel MacBooks & iMacs (macOS 11 Big Sur+, x86_64)
- Apple Silicon Macs (ARM64)
- Windows 10/11 x64 laptops with 4 GB RAM, dual-core CPUs, integrated Intel HD Graphics, and HDDs

To keep idle memory in the 60–120 MB range and avoid continuous background redraws:
1. **No Electron / CEF**: The desktop shell and UI are compiled natively in Rust using **Iced 0.13** (`wgpu` GPU rendering with `tiny-skia` CPU software fallback).
2. **Strict Separation of Concerns**: Account auth/metadata (`YouTube Data API v3` + `OAuth 2.0 PKCE`) and video playback (`PlayerBackend` trait) are decoupled.
3. **Isolated Embed Playback (`wry`)**: Official YouTube playback runs inside an isolated child system WebView (`WKWebView` on macOS, `WebView2` on Windows) only when watching videos, with an injected `h264ify` shim that prioritizes hardware-decodable H.264 streams over CPU-heavy VP9/AV1. Under **Ultra Lite**, the WebView lifecycle policy destroys the child WebView when navigating away.
4. **Non-blocking Async Pipeline**: Network I/O (`tokio` + `reqwest`), SQLite persistence (`rusqlite` WAL mode), and image decoding/downscaling (`rayon` + `image`) run completely off the UI thread.

---

## 2. Cargo Workspace Layout

| Crate / App | Responsibility |
|---|---|
| `apps/desktop` | Main binary (`fluxtube`), application bootstrap, event loop, keyboard shortcuts, `--smoke-test` |
| `crates/core` | Pure domain models (`Video`, `Channel`, `Playlist`, `Comment`, `HistoryEntry`, `PerformanceSettings`) |
| `crates/design-system` | Theme tokens (`GraphiteDark`, `PureBlack`, `ArcticLight`, `System`, `Custom`), typography, spacing, reusable widgets |
| `crates/ui` | Iced views (`Home`, `Watch`, `Search`, `Channel`, `Subscriptions`, `Library`, `Shorts`, `Live`, `Settings`, `Diagnostics`) |
| `crates/youtube-api` | `YouTubeDataApi` trait, `LiveYouTubeApiClient` (REST v3 + quota), `MockYouTubeApiClient` (offline demo) |
| `crates/auth` | Google OAuth 2.0 Authorization Code + PKCE S256, loopback HTTP listener, token refresh coordinator |
| `crates/security` | `CredentialVault` wrapping macOS Keychain & Windows Credential Manager (`keyring`) + `zeroize` |
| `crates/network` | `ResilientHttpClient`, exponential backoff, circuit breaker, in-flight GET deduplication, `QuotaTracker` |
| `crates/player` | Abstract `PlayerBackend` trait, `MediaSource`, `PlaybackCapabilities`, `PlayerEventBus` |
| `crates/player-embed` | Official YouTube IFrame embed via `wry`, IPC bridge, `h264ify` JS shim, profile lifecycle policies |
| `crates/player-native` | `NativePlayerBackend` for permitted local files and direct streams |
| `crates/storage` | SQLite WAL manager, versioned migrations, history, watch positions, bookmarks, local playlists |
| `crates/cache` | Two-tier L1 RAM + L2 disk LRU thumbnail cache with singleflight deduplication & off-thread resizing |
| `crates/settings` | `AppSettings`, TOML import/export, hardware profile auto-recommendation |
| `crates/diagnostics` | Live CPU/RAM/RSS telemetry (`sysinfo`), anonymized JSON/Markdown report exporter |
| `crates/localization` | Runtime translation catalog (`en`, `uk`, `ru`) with argument interpolation and fallback chain |
| `crates/platform` | Hardware & decoder detection, XDG/OS user paths, YouTube URL parser, external link validator |
| `crates/updater` | SemVer release checker, channel filtering (`stable`/`beta`), SHA-256 + Ed25519 signature verification & rollback |

---

## 3. GUI Framework Research Summary

| Criterion | **Iced 0.13 (Chosen)** | **Slint** | **egui** | **Tauri (WebView)** |
|---|---|---|---|---|
| **Idle RAM (no WebView)** | ~40–75 MB | ~35–65 MB | ~30–55 MB | ~140–250 MB |
| **Idle CPU** | < 0.5% (event-driven) | < 0.5% | Higher if continuous repaint | Moderate (JS runtime) |
| **macOS 11 Big Sur** | Supported (`wgpu` Metal / `tiny-skia`) | Supported | Supported | Supported (WKWebView) |
| **Old GPU / Software Fallback** | Built-in `tiny-skia` + `softbuffer` | Skia / FemtoVG / software | Glow/wgpu (limited software) | System WebView |
| **Licensing** | MIT | GPLv3 or paid commercial | MIT / Apache-2.0 | MIT / Apache-2.0 |
| **Pure Rust UI** | Yes (Elm architecture) | `.slint` DSL | Immediate mode | HTML/CSS/JS |

**Conclusion**: Iced 0.13 is selected as the primary framework due to MIT licensing, zero JS runtime overhead for UI chrome, native event-driven rendering (`<1%` idle CPU), and built-in `tiny-skia` software fallback for legacy GPUs.
