# Contributing to FluxTube

1. Keep modules decoupled along crate boundaries (`core` must not depend on `ui` or network).
2. Never store secrets, API keys, or refresh tokens in plaintext files, SQLite, or logs.
3. All user-facing strings in `crates/ui` must come from `crates/localization` (`en.json`, `uk.json`, `ru.json`).
4. Run full verification before submitting PRs:
   ```bash
   cargo clippy --workspace --all-targets
   cargo test --workspace
   cargo test --manifest-path tests/Cargo.toml
   ```
5. Document any YouTube Data API v3 limitations honestly in `FEATURE_MATRIX.md` rather than mocking unavailable endpoints in production.
