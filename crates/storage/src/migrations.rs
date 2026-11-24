use crate::error::StorageError;
use chrono::Utc;
use rusqlite::Connection;
use tracing::info;

pub struct Migration {
    pub version: i64,
    pub description: &'static str,
    pub sql: &'static str,
}

pub const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        description: "Initial 13 tables schema",
        sql: r#"
        -- 1. settings
        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        -- 2. accounts
        CREATE TABLE IF NOT EXISTS accounts (
            id TEXT PRIMARY KEY,
            email TEXT NOT NULL,
            name TEXT NOT NULL,
            picture_url TEXT,
            is_active INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_accounts_active ON accounts(is_active);

        -- 3. local_history
        CREATE TABLE IF NOT EXISTS local_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            video_id TEXT NOT NULL,
            title TEXT NOT NULL,
            channel_title TEXT NOT NULL,
            thumbnail_url TEXT NOT NULL,
            duration_seconds INTEGER NOT NULL,
            watched_at TEXT NOT NULL,
            playback_position_seconds INTEGER NOT NULL DEFAULT 0,
            completed INTEGER NOT NULL DEFAULT 0,
            account_id TEXT,
            FOREIGN KEY(account_id) REFERENCES accounts(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS idx_local_history_watched ON local_history(watched_at DESC);
        CREATE INDEX IF NOT EXISTS idx_local_history_video ON local_history(video_id);
        CREATE INDEX IF NOT EXISTS idx_local_history_account ON local_history(account_id);

        -- 4. playlists
        CREATE TABLE IF NOT EXISTS playlists (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            description TEXT,
            item_count INTEGER NOT NULL DEFAULT 0,
            thumbnail_url TEXT,
            is_local INTEGER NOT NULL DEFAULT 1,
            account_id TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY(account_id) REFERENCES accounts(id) ON DELETE SET NULL
        );
        CREATE INDEX IF NOT EXISTS idx_playlists_account ON playlists(account_id);
        CREATE INDEX IF NOT EXISTS idx_playlists_updated ON playlists(updated_at DESC);

        -- 5. playlist_items
        CREATE TABLE IF NOT EXISTS playlist_items (
            id TEXT PRIMARY KEY,
            playlist_id TEXT NOT NULL,
            video_id TEXT NOT NULL,
            title TEXT NOT NULL,
            thumbnail_url TEXT,
            position INTEGER NOT NULL,
            added_at TEXT NOT NULL,
            FOREIGN KEY(playlist_id) REFERENCES playlists(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS idx_playlist_items_order ON playlist_items(playlist_id, position ASC);
        CREATE INDEX IF NOT EXISTS idx_playlist_items_video ON playlist_items(video_id);

        -- 6. channel_groups
        CREATE TABLE IF NOT EXISTS channel_groups (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            color TEXT,
            icon TEXT,
            sort_order INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_channel_groups_sort ON channel_groups(sort_order ASC);

        -- 7. channel_group_items
        CREATE TABLE IF NOT EXISTS channel_group_items (
            group_id TEXT NOT NULL,
            channel_id TEXT NOT NULL,
            channel_title TEXT NOT NULL,
            avatar_url TEXT,
            added_at TEXT NOT NULL,
            PRIMARY KEY(group_id, channel_id),
            FOREIGN KEY(group_id) REFERENCES channel_groups(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS idx_channel_group_items_chan ON channel_group_items(channel_id);

        -- 8. search_history
        CREATE TABLE IF NOT EXISTS search_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            query TEXT NOT NULL UNIQUE,
            searched_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_search_history_time ON search_history(searched_at DESC);

        -- 9. metadata_cache
        CREATE TABLE IF NOT EXISTS metadata_cache (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            expires_at TEXT NOT NULL,
            created_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_metadata_cache_exp ON metadata_cache(expires_at);

        -- 10. bookmarks
        CREATE TABLE IF NOT EXISTS bookmarks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            video_id TEXT NOT NULL,
            title TEXT NOT NULL,
            timestamp_seconds INTEGER NOT NULL,
            note TEXT,
            created_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_bookmarks_video ON bookmarks(video_id, timestamp_seconds ASC);

        -- 11. video_positions
        CREATE TABLE IF NOT EXISTS video_positions (
            video_id TEXT PRIMARY KEY,
            position_seconds INTEGER NOT NULL,
            duration_seconds INTEGER NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_video_positions_time ON video_positions(updated_at DESC);

        -- 12. quota_tracking
        CREATE TABLE IF NOT EXISTS quota_tracking (
            date TEXT NOT NULL,
            pool TEXT NOT NULL,
            units_used INTEGER NOT NULL DEFAULT 0,
            PRIMARY KEY(date, pool)
        );
        "#,
    },
];

pub fn run_migrations(conn: &mut Connection) -> Result<(), StorageError> {
    // 0. schema_migrations table
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL,
            description TEXT NOT NULL
        );
        "#,
    )?;

    let current_version: i64 = conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
        [],
        |row| row.get(0),
    )?;

    for migration in MIGRATIONS {
        if migration.version > current_version {
            info!(
                version = migration.version,
                description = migration.description,
                "Applying database migration"
            );

            let tx = conn.transaction()?;
            tx.execute_batch(migration.sql).map_err(|e| {
                StorageError::MigrationFailed {
                    version: migration.version,
                    reason: e.to_string(),
                }
            })?;

            let now = Utc::now().to_rfc3339();
            tx.execute(
                "INSERT INTO schema_migrations (version, applied_at, description) VALUES (?1, ?2, ?3)",
                rusqlite::params![migration.version, now, migration.description],
            ).map_err(|e| {
                StorageError::MigrationFailed {
                    version: migration.version,
                    reason: e.to_string(),
                }
            })?;

            tx.commit()?;
            info!(version = migration.version, "Migration applied successfully");
        }
    }

    Ok(())
}
