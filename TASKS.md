# FluxTube Task Tracker

## Completed
- [x] Cargo Workspace & macOS 11.0 deployment target configuration
- [x] `crates/core`, `crates/storage`, `crates/security`, `crates/settings`, `crates/localization`, `crates/diagnostics`
- [x] `crates/network`, `crates/auth`, `crates/youtube-api` (Live + Mock)
- [x] `crates/player`, `crates/player-embed` (`h264ify`), `crates/player-native`, `crates/cache`
- [x] `crates/platform`, `crates/updater` (Ed25519 + SHA-256)
- [x] `crates/design-system`, `crates/ui`, `apps/desktop`
- [x] E2E 4-tier test suite (`tests/e2e/tier1..tier4`)
- [x] Repository documentation & GitHub Actions CI

## Remaining Hardware / Release Tasks
- [ ] Attach native `wry` child WebView HWND/NSView bounds sync inside active Watch view
- [ ] Benchmark RAM/CPU on physical macOS 11 Big Sur (Intel MacBook Air) and 4 GB RAM Windows 10 hardware
- [ ] Code signing & Apple notarization with Apple Developer ID
