//! Mock SQLite Storage Engine & Database Factory for FluxTube E2E Testing.

use rusqlite::{Connection, Result};
use tempfile::TempDir;

/// Canonical 13-table DDL schema for FluxTube SQLite Database (PROJECT.md lines 62-63, 77-81).
pub const FLUXTUBE_SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS schema_migrations (
    version INTEGER PRIMARY KEY,
    applied_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS accounts (
    id TEXT PRIMARY KEY,
    email TEXT NOT NULL,
    display_name TEXT NOT NULL,
    avatar_url TEXT,
    is_active INTEGER DEFAULT 0,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS local_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id TEXT,
    video_id TEXT NOT NULL,
    title TEXT NOT NULL,
    channel_id TEXT NOT NULL,
    channel_title TEXT NOT NULL,
    thumbnail_url TEXT,
    last_position_seconds INTEGER NOT NULL,
    duration_seconds INTEGER NOT NULL,
    completed INTEGER DEFAULT 0,
    watched_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(account_id) REFERENCES accounts(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS local_playlists (
    id TEXT PRIMARY KEY,
    account_id TEXT,
    title TEXT NOT NULL,
    description TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(account_id) REFERENCES accounts(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS local_playlist_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    playlist_id TEXT NOT NULL,
    video_id TEXT NOT NULL,
    title TEXT NOT NULL,
    thumbnail_url TEXT,
    position INTEGER NOT NULL,
    added_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(playlist_id) REFERENCES local_playlists(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS channel_groups (
    id TEXT PRIMARY KEY,
    account_id TEXT,
    name TEXT NOT NULL,
    color TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(account_id) REFERENCES accounts(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS channel_group_members (
    group_id TEXT NOT NULL,
    channel_id TEXT NOT NULL,
    channel_title TEXT NOT NULL,
    PRIMARY KEY (group_id, channel_id),
    FOREIGN KEY(group_id) REFERENCES channel_groups(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS search_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    query TEXT NOT NULL UNIQUE,
    searched_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS metadata_cache (
    key TEXT PRIMARY KEY,
    content TEXT NOT NULL,
    cached_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    expires_at TIMESTAMP NOT NULL
);

CREATE TABLE IF NOT EXISTS offline_playback_markers (
    video_id TEXT PRIMARY KEY,
    marker_seconds INTEGER NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS video_bookmarks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    video_id TEXT NOT NULL,
    position_seconds INTEGER NOT NULL,
    note TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS api_quota_tracking (
    pool TEXT PRIMARY KEY,
    units_used INTEGER DEFAULT 0,
    reset_at TIMESTAMP NOT NULL
);
"#;

/// Configures SQLite standard pragmas for FluxTube.
pub fn apply_pragmas(conn: &Connection, is_disk: bool) -> Result<()> {
    if is_disk {
        conn.execute_batch("PRAGMA journal_mode = WAL;")?;
    }
    conn.execute_batch(
        r#"
        PRAGMA synchronous = NORMAL;
        PRAGMA foreign_keys = ON;
        PRAGMA busy_timeout = 5000;
        "#,
    )?;
    Ok(())
}

/// Creates a fresh in-memory SQLite database initialized with all 13 canonical tables.
pub fn create_test_db() -> Connection {
    let conn = Connection::open_in_memory().expect("Failed to open in-memory SQLite connection");
    apply_pragmas(&conn, false).expect("Failed to apply SQLite pragmas");
    conn.execute_batch(FLUXTUBE_SCHEMA_SQL)
        .expect("Failed to initialize FluxTube database schema");
    conn
}

/// Creates a temporary on-disk SQLite database wrapped with TempDir for auto-cleanup.
pub fn create_test_db_on_disk() -> (Connection, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temporary directory");
    let db_path = temp_dir.path().join("fluxtube_test.db");
    let conn = Connection::open(&db_path).expect("Failed to open on-disk SQLite connection");
    apply_pragmas(&conn, true).expect("Failed to apply SQLite disk pragmas");
    conn.execute_batch(FLUXTUBE_SCHEMA_SQL)
        .expect("Failed to initialize FluxTube database schema on disk");
    (conn, temp_dir)
}

/// Helper wrapper providing typed query convenience for tests.
pub struct TestStorageDb {
    pub conn: Connection,
}

impl TestStorageDb {
    pub fn in_memory() -> Self {
        Self {
            conn: create_test_db(),
        }
    }

    pub fn table_count(&self) -> usize {
        let count: usize = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);
        count
    }

    pub fn has_table(&self, name: &str) -> bool {
        let exists: bool = self
            .conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
                [name],
                |row| row.get(0),
            )
            .unwrap_or(false);
        exists
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn test_create_test_db_schema() {
        let db = TestStorageDb::in_memory();
        assert_eq!(db.table_count(), 13);

        let required_tables = [
            "schema_migrations",
            "settings",
            "accounts",
            "local_history",
            "local_playlists",
            "local_playlist_items",
            "channel_groups",
            "channel_group_members",
            "search_history",
            "metadata_cache",
            "offline_playback_markers",
            "video_bookmarks",
            "api_quota_tracking",
        ];

        for table in &required_tables {
            assert!(db.has_table(table), "Missing required table: {table}");
        }
    }

    #[test]
    fn test_cascading_foreign_key() {
        let conn = create_test_db();
        conn.execute(
            "INSERT INTO accounts (id, email, display_name) VALUES ('acc1', 'user@example.com', 'Tester')",
            [],
        ).unwrap();

        conn.execute(
            "INSERT INTO local_history (account_id, video_id, title, channel_id, channel_title, last_position_seconds, duration_seconds)
             VALUES ('acc1', 'vid1', 'Test Video', 'ch1', 'Channel 1', 10, 100)",
            [],
        ).unwrap();

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM local_history WHERE account_id = 'acc1'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);

        // Delete account -> history should cascade delete
        conn.execute("DELETE FROM accounts WHERE id = 'acc1'", []).unwrap();
        let post_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM local_history WHERE account_id = 'acc1'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(post_count, 0);
    }
}
