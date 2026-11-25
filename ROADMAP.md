# FluxTube Roadmap

- **Phase 0 – Technical Research (Done)**: Evaluated Iced vs Slint vs egui vs Tauri, macOS 11 Big Sur deployment constraints, YouTube Data API v3 compliance, and isolated WebView + `h264ify` playback.
- **Phase 1 – Core Foundation (Done)**: `core`, `storage` (SQLite WAL + migrations), `security` (OS Keychain/Credential Manager + zeroize), `settings`, `localization` (en/uk/ru), `diagnostics`.
- **Phase 2 – Network, Auth & API (Done)**: `network` (ResilientHttpClient, circuit breaker, deduplication, Pacific Time `QuotaTracker`), `auth` (OAuth 2.0 PKCE S256 + loopback), `youtube-api` (live + mock clients).
- **Phase 3 – Media & Cache (Done)**: `player`, `player-embed` (`wry` + `h264ify` + lifecycle policies), `player-native`, `cache` (L1/L2 LRU + singleflight).
- **Phase 4 – UI, Platform, Updater & Desktop App (Done)**: `platform`, `updater` (Ed25519 + SHA-256), `design-system` (5 themes, responsive grid), `ui` (15 routes), `apps/desktop` (`fluxtube`).
- **Phase 5 – Hardware Validation & Public Packaging (Next)**: Native `wry` child WebView window attachment inside Iced window handle, real physical macOS 11 Big Sur & 4 GB Windows 10 laptop benchmarks, notarization & signed GitHub Releases.
