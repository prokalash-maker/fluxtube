# FluxTube Security & Threat Model

## 1. Security Architecture Principles
Security is enforced at crate boundaries:
- **Zero Plaintext Refresh Tokens**: OAuth refresh tokens are stored exclusively in OS-protected vaults via `crates/security`:
  - **macOS**: Keychain Services (`Security.framework`)
  - **Windows**: Windows Credential Manager / DPAPI
  - Sensitive buffers use `zeroize::Zeroizing` to scrub memory on drop.
  - Refresh tokens are **never** written to SQLite, JSON, `.env`, or diagnostic logs.
- **Official Desktop OAuth 2.0 Flow**:
  - Uses Authorization Code Flow + **PKCE S256** + cryptographic `state` validation (`crates/auth`).
  - Opens the user's **system browser** and listens on an ephemeral `127.0.0.1:<port>` loopback socket.
  - FluxTube never asks for or renders Google login forms inside an embedded WebView.
- **WebView Isolation (`crates/player-embed`)**:
  - Loads only `youtube-nocookie.com/embed/...` via an isolated `wry` child WebView.
  - Does not expose arbitrary filesystem access or privileged Rust IPC commands to external origins.
- **SQL Injection & URL Validation**:
  - All `rusqlite` queries use parameterized bindings (`params![...]`).
  - `fluxtube-platform::open_external_url` and `parse_youtube_url` strictly validate schemes (`http`/`https`) and allowed domains before opening external links.
- **Cryptographically Verified Updates (`crates/updater`)**:
  - Updates require both a valid **SHA-256** digest and an **Ed25519** signature before staging.
  - Unsigned or tampered payloads are rejected (`UpdaterError::UnsignedUpdateRejected`), with atomic backup/rollback support.
- **Privacy by Default**:
  - Zero third-party telemetry, crash reporters, or ad SDKs.
  - Diagnostic exports (`fluxtube-diagnostics`) redact tokens, emails, and credentials.
  - Private Mode disables writing local watch and search history.

## 2. Verification & Static Analysis
Run the following before releases:
```bash
cargo clippy --workspace --all-targets
cargo test --workspace
cargo test --manifest-path tests/Cargo.toml
```
When `cargo-audit` / `cargo-deny` are installed:
```bash
cargo audit
cargo deny check
```
