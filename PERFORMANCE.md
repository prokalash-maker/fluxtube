# FluxTube Performance Engineering & Targets

## Target Resource Budgets
*(Design targets; verify with real measurements on target hardware before final release tagging)*

| Mode | Idle RAM (no active WebView) | Navigation RAM | Idle CPU | Startup Target |
|---|---|---|---|---|
| **Ultra Lite** | 60–120 MB | <= 150 MB | < 1% | <= 2–3s |
| **Balanced** | 80–150 MB | <= 200 MB | < 1% | <= 2–3s |
| **High Quality** | 120–200 MB | <= 300 MB | < 1% | <= 3s |

## Key Optimizations Implemented
1. **Event-Driven UI Loop**: Iced only re-renders on user interaction, resize, or explicit async completion—no continuous 60fps spin loops when idle.
2. **H.264 Hardware Acceleration (`h264ify` shim)**: `crates/player-embed/src/shim.rs` overrides `canPlayType` and `MediaSource.isTypeSupported` to decline VP8/VP9/AV1/WebM and advertise H.264 (`avc1`), enabling VideoToolbox (macOS) and D3D11VA/DXVA2 (Windows) on legacy Intel HD Graphics.
3. **Profile-Driven WebView Lifecycle**: `ProfileLifecyclePolicy::DestroyOnNavigateAway` destroys the child WebView immediately when leaving the Watch page in Ultra Lite mode, reclaiming all WebView memory.
4. **Two-Tier LRU Thumbnail Cache**: `crates/cache` uses an L1 RGBA memory LRU + L2 disk cache with `singleflight` deduplication and off-thread `rayon` decoding/downscaling to exact widget dimensions.
5. **SQLite WAL Mode**: `crates/storage` configures `PRAGMA journal_mode = WAL`, `synchronous = NORMAL`, and `temp_store = MEMORY` to avoid UI stalls on slow HDDs.
6. **Hardware Auto-Recommendation**: `fluxtube-settings::recommend_profile` automatically selects **Ultra Lite** for systems with <= 2 CPU cores or < 4 GB RAM.
