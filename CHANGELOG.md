# Changelog

## [0.1.0] - 2026-11-25
### Added
- Full 18-crate modular Cargo workspace + `apps/desktop` (`fluxtube`) binary.
- Native Iced 0.13 UI with 5 themes (`GraphiteDark`, `PureBlack`, `ArcticLight`, `System`, `Custom`), 3 density modes, and responsive column grid.
- Localization catalogs for English (`en`), Ukrainian (`uk`), and Russian (`ru`).
- SQLite WAL local persistence for watch history, resume positions, timestamp bookmarks, local playlists, channel groups, and settings.
- Google OAuth 2.0 PKCE S256 authentication with macOS Keychain / Windows Credential Manager integration.
- YouTube Data API v3 live client + offline `MockYouTubeApiClient` demo mode + Pacific Time `QuotaTracker`.
- Isolated `wry` embed player abstraction with `h264ify` H.264 preference shim and profile-based lifecycle policies.
- Two-tier L1/L2 LRU thumbnail cache with singleflight deduplication and off-thread decoding.
- Diagnostics Center with live RAM/CPU telemetry and anonymized report export.
- Cryptographically verified updater (`SHA-256` + `Ed25519`) with atomic staging and rollback.
- Multi-tier E2E test suite (`tier1`..`tier4`) and GitHub Actions CI workflow.
