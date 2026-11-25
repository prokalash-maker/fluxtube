# FluxTube Feature Matrix

This document explicitly classifies every feature by its technical implementation and official YouTube Data API v3 / Terms of Service boundaries. FluxTube never uses private Google endpoints, credential scraping, or DRM/ad bypasses.

## Legend
- **Official API**: Implemented directly via YouTube Data API v3 (`crates/youtube-api`).
- **Embed / WebView**: Handled exclusively inside the official YouTube IFrame Embed (`crates/player-embed`).
- **Local Device**: Implemented locally in SQLite (`crates/storage`) and clearly marked in the UI as device-only.
- **Unsupported**: Not available to third-party desktop clients via official APIs.

---

| Feature | Category | Implementation & Constraints |
|---|---|---|
| **Video / Channel / Playlist Search** | Official API | `search.list` (100 quota units/call). Debounced + filtered by type, duration, and order. |
| **Trending / Popular Feed** | Official API | `videos.list?chart=mostPopular` (1 quota unit/call). |
| **Video Metadata & Chapters** | Official API | Batched `videos.list` (up to 50 IDs/call). Chapters parsed locally from timestamps in description. |
| **Channel Details & Uploads** | Official API | `channels.list` (ID or `@handle`) + `playlistItems.list` on derived `UU...` uploads playlist. |
| **User Subscriptions** | Official API | `subscriptions.list?mine=true`, `subscriptions.insert`, `subscriptions.delete` (OAuth required). |
| **Subscribe / Unsubscribe** | Official API | Requires Google OAuth 2.0 PKCE session; local channel groups also supported offline. |
| **Video Ratings (Like / Dislike)** | Official API | `videos.rate` & `videos.getRating` (OAuth required). |
| **Read & Post Comments** | Official API | `commentThreads.list` (public) and `commentThreads.insert` (OAuth). Collapsible to save RAM/CPU. |
| **YouTube Playlists** | Official API | `playlists.list?mine=true`, `playlists.insert`, `playlistItems.insert/delete`. |
| **Video Playback & Adaptive Quality** | Embed / WebView | Official YouTube IFrame Player API inside isolated `wry` WebView with `h264ify` codec preference. |
| **Subtitles / Captions (Playback)** | Embed / WebView | Controlled natively by the official YouTube Embed player. |
| **Personalized Home Feed** | Local Device | YouTube Data API v3 does **not** expose the website's personalized Home feed; FluxTube synthesizes local picks from subscriptions, trending, and device history, labeled *Device-only*. |
| **Watch History & Resume Position** | Local Device | YouTube API v3 does **not** expose personal watch history; stored locally in SQLite (`local_history`, `video_positions`). |
| **Watch Later** | Local Device | `WL` special playlist is restricted in API v3; FluxTube provides a local Watch Later list. |
| **Timestamp Bookmarks & Video Notes** | Local Device | Stored in SQLite `bookmarks` and settings; supports copying `youtu.be/<id>?t=<sec>` links. |
| **Local Playlists & Channel Groups** | Local Device | Stored in SQLite (`playlists`, `channel_groups`); JSON export supported. |
| **Shorts Discovery** | Local Device + API | Official API lacks a dedicated Shorts endpoint; FluxTube filters locally by `duration <= 60s` and notes this limitation in UI. |
| **Live Stream Chat** | Official API (Partial) | `liveChatMessages.list` requires active liveChatId and quota; toggleable off in UI to save resources. |
| **Offline Video Stream Caching / Ad-blocking** | Unsupported | Prohibited for third-party clients; FluxTube does not download or strip YouTube video streams. |
