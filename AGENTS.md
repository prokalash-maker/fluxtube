# AGENTS.md — Contributor & AI Agent Guidelines

- **Target Platforms**: Maintain strict compatibility with `MACOSX_DEPLOYMENT_TARGET=11.0` (Intel x86_64 & Apple Silicon ARM64) and Windows 10/11 x64.
- **No Browser Bloat**: Never introduce Electron, CEF, or full-page YouTube web scraping into the UI layer.
- **Compliance**: Use only official YouTube Data API v3 endpoints, Google OAuth 2.0 PKCE via system browser, and official embed playback.
- **Verification**: Always compile and run `cargo test --workspace` and `cargo test --manifest-path tests/Cargo.toml` after changes.
