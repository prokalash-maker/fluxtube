pub mod error;
pub mod migrations;

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use rusqlite::{params, Connection, OptionalExtension};

use fluxtube_core::{
    Bookmark, ChannelGroup, ChannelGroupItem, HistoryEntry, Playlist, PlaylistItem, QuotaPool,
    QuotaStatus, UserProfile, VideoPosition,
};

pub use error::StorageError;

#[derive(Clone)]
pub struct StorageManager {
    conn: Arc<Mutex<Connection>>,
}

impl StorageManager {
    fn init_connection(mut conn: Connection) -> Result<Self, StorageError> {
        conn.execute_batch(
            r#"
            PRAGMA journal_mode = WAL;
            PRAGMA foreign_keys = ON;
            PRAGMA busy_timeout = 5000;
            PRAGMA synchronous = NORMAL;
            PRAGMA cache_size = -2000;
            PRAGMA temp_store = MEMORY;
            "#,
        )?;

        migrations::run_migrations(&mut conn)?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let conn = Connection::open(path)?;
        Self::init_connection(conn)
    }

    pub fn open_in_memory() -> Result<Self, StorageError> {
        let conn = Connection::open_in_memory()?;
        Self::init_connection(conn)
    }

    // Maintenance
    pub fn integrity_check(&self) -> Result<bool, StorageError> {
        let conn = self.conn.lock();
        let status: String = conn.query_row("PRAGMA integrity_check;", [], |r| r.get(0))?;
        Ok(status == "ok")
    }

    pub fn vacuum(&self) -> Result<(), StorageError> {
        let conn = self.conn.lock();
        conn.execute_batch("VACUUM;")?;
        Ok(())
    }

    // Settings
    pub fn get_setting(&self, key: &str) -> Result<Option<String>, StorageError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = ?1")?;
        let res = stmt.query_row([key], |row| row.get(0)).optional()?;
        Ok(res)
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<(), StorageError> {
        let conn = self.conn.lock();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            r#"
            INSERT INTO settings (key, value, updated_at)
            VALUES (?1, ?2, ?3)
            ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at
            "#,
            params![key, value, now],
        )?;
        Ok(())
    }

    pub fn delete_setting(&self, key: &str) -> Result<(), StorageError> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM settings WHERE key = ?1", params![key])?;
        Ok(())
    }

    // Accounts
    pub fn save_account(&self, profile: &UserProfile, is_active: bool) -> Result<(), StorageError> {
        let conn = self.conn.lock();
        let now = profile.created_at.to_rfc3339();
        let active_val = if is_active { 1 } else { 0 };

        if is_active {
            conn.execute("UPDATE accounts SET is_active = 0", [])?;
        }

        conn.execute(
            r#"
            INSERT INTO accounts (id, email, name, picture_url, is_active, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ON CONFLICT(id) DO UPDATE SET
                email = excluded.email,
                name = excluded.name,
                picture_url = excluded.picture_url,
                is_active = excluded.is_active
            "#,
            params![
                profile.id,
                profile.email,
                profile.name,
                profile.picture,
                active_val,
                now
            ],
        )?;
        Ok(())
    }

    pub fn get_active_account(&self) -> Result<Option<UserProfile>, StorageError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, email, name, picture_url, is_active, created_at FROM accounts WHERE is_active = 1 LIMIT 1",
        )?;
        let res = stmt
            .query_row([], |row| {
                let created_str: String = row.get(5)?;
                let created_at = DateTime::parse_from_rfc3339(&created_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());
                let active_val: i32 = row.get(4)?;
                Ok(UserProfile {
                    id: row.get(0)?,
                    email: row.get(1)?,
                    name: row.get(2)?,
                    picture: row.get(3)?,
                    is_active: active_val != 0,
                    created_at,
                })
            })
            .optional()?;
        Ok(res)
    }

    pub fn get_all_accounts(&self) -> Result<Vec<UserProfile>, StorageError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, email, name, picture_url, is_active, created_at FROM accounts ORDER BY created_at ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            let created_str: String = row.get(5)?;
            let created_at = DateTime::parse_from_rfc3339(&created_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());
            let active_val: i32 = row.get(4)?;
            Ok(UserProfile {
                id: row.get(0)?,
                email: row.get(1)?,
                name: row.get(2)?,
                picture: row.get(3)?,
                is_active: active_val != 0,
                created_at,
            })
        })?;

        let mut accounts = Vec::new();
        for acc in rows {
            accounts.push(acc?);
        }
        Ok(accounts)
    }

    pub fn set_active_account(&self, account_id: &str) -> Result<(), StorageError> {
        let conn = self.conn.lock();
        conn.execute("UPDATE accounts SET is_active = 0", [])?;
        let updated = conn.execute(
            "UPDATE accounts SET is_active = 1 WHERE id = ?1",
            params![account_id],
        )?;
        if updated == 0 {
            return Err(StorageError::NotFound(format!(
                "Account id {} not found",
                account_id
            )));
        }
        Ok(())
    }

    pub fn delete_account(&self, account_id: &str) -> Result<(), StorageError> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM accounts WHERE id = ?1", params![account_id])?;
        Ok(())
    }

    // History
    pub fn add_history_entry(&self, entry: &HistoryEntry) -> Result<(), StorageError> {
        let conn = self.conn.lock();
        let watched_at = entry.watched_at.to_rfc3339();
        let completed = if entry.completed { 1 } else { 0 };

        conn.execute(
            r#"
            INSERT INTO local_history
                (video_id, title, channel_title, thumbnail_url, duration_seconds, watched_at, playback_position_seconds, completed, account_id)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            "#,
            params![
                entry.video_id,
                entry.title,
                entry.channel_title,
                entry.thumbnail_url,
                entry.duration_seconds,
                watched_at,
                entry.playback_position_seconds,
                completed,
                entry.account_id
            ],
        )?;
        Ok(())
    }

    pub fn get_history(
        &self,
        account_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<HistoryEntry>, StorageError> {
        let conn = self.conn.lock();
        let query = match account_id {
            Some(_) => {
                "SELECT id, video_id, title, channel_title, thumbnail_url, duration_seconds, watched_at, playback_position_seconds, completed, account_id
                 FROM local_history WHERE account_id = ?1 ORDER BY watched_at DESC LIMIT ?2"
            }
            None => {
                "SELECT id, video_id, title, channel_title, thumbnail_url, duration_seconds, watched_at, playback_position_seconds, completed, account_id
                 FROM local_history WHERE account_id IS NULL ORDER BY watched_at DESC LIMIT ?1"
            }
        };

        let map_row = |row: &rusqlite::Row| {
            let watched_str: String = row.get(6)?;
            let watched_at = DateTime::parse_from_rfc3339(&watched_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());
            let completed_int: i32 = row.get(8)?;
            Ok(HistoryEntry {
                id: Some(row.get(0)?),
                video_id: row.get(1)?,
                title: row.get(2)?,
                channel_title: row.get(3)?,
                thumbnail_url: row.get(4)?,
                duration_seconds: row.get(5)?,
                watched_at,
                playback_position_seconds: row.get(7)?,
                completed: completed_int != 0,
                account_id: row.get(9)?,
            })
        };

        let mut entries = Vec::new();
        if let Some(acc) = account_id {
            let mut stmt = conn.prepare(query)?;
            let rows = stmt.query_map(params![acc, limit as i64], map_row)?;
            for r in rows {
                entries.push(r?);
            }
        } else {
            let mut stmt = conn.prepare(query)?;
            let rows = stmt.query_map(params![limit as i64], map_row)?;
            for r in rows {
                entries.push(r?);
            }
        }
        Ok(entries)
    }

    pub fn clear_history(&self, account_id: Option<&str>) -> Result<(), StorageError> {
        let conn = self.conn.lock();
        match account_id {
            Some(acc) => {
                conn.execute(
                    "DELETE FROM local_history WHERE account_id = ?1",
                    params![acc],
                )?;
            }
            None => {
                conn.execute("DELETE FROM local_history WHERE account_id IS NULL", [])?;
            }
        }
        Ok(())
    }

    pub fn remove_history_entry(
        &self,
        video_id: &str,
        account_id: Option<&str>,
    ) -> Result<(), StorageError> {
        let conn = self.conn.lock();
        match account_id {
            Some(acc) => {
                conn.execute(
                    "DELETE FROM local_history WHERE video_id = ?1 AND account_id = ?2",
                    params![video_id, acc],
                )?;
            }
            None => {
                conn.execute(
                    "DELETE FROM local_history WHERE video_id = ?1 AND account_id IS NULL",
                    params![video_id],
                )?;
            }
        }
        Ok(())
    }

    // Search History
    pub fn add_search_query(&self, query: &str) -> Result<(), StorageError> {
        let conn = self.conn.lock();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            r#"
            INSERT INTO search_history (query, searched_at)
            VALUES (?1, ?2)
            ON CONFLICT(query) DO UPDATE SET searched_at = excluded.searched_at
            "#,
            params![query, now],
        )?;
        Ok(())
    }

    pub fn get_search_history(&self, limit: usize) -> Result<Vec<String>, StorageError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT query FROM search_history ORDER BY searched_at DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit as i64], |row| row.get(0))?;
        let mut results = Vec::new();
        for r in rows {
            results.push(r?);
        }
        Ok(results)
    }

    pub fn clear_search_history(&self) -> Result<(), StorageError> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM search_history", [])?;
        Ok(())
    }

    pub fn remove_search_query(&self, query: &str) -> Result<(), StorageError> {
        let conn = self.conn.lock();
        conn.execute(
            "DELETE FROM search_history WHERE query = ?1",
            params![query],
        )?;
        Ok(())
    }

    // Video Positions
    pub fn save_position(
        &self,
        video_id: &str,
        pos_secs: u64,
        duration_secs: u64,
    ) -> Result<(), StorageError> {
        let conn = self.conn.lock();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            r#"
            INSERT INTO video_positions (video_id, position_seconds, duration_seconds, updated_at)
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(video_id) DO UPDATE SET
                position_seconds = excluded.position_seconds,
                duration_seconds = excluded.duration_seconds,
                updated_at = excluded.updated_at
            "#,
            params![video_id, pos_secs as i64, duration_secs as i64, now],
        )?;
        Ok(())
    }

    pub fn get_position(&self, video_id: &str) -> Result<Option<VideoPosition>, StorageError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT video_id, position_seconds, duration_seconds, updated_at FROM video_positions WHERE video_id = ?1",
        )?;
        let res = stmt
            .query_row(params![video_id], |row| {
                let updated_str: String = row.get(3)?;
                let updated_at = DateTime::parse_from_rfc3339(&updated_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());
                let pos_i64: i64 = row.get(1)?;
                let dur_i64: i64 = row.get(2)?;
                Ok(VideoPosition {
                    video_id: row.get(0)?,
                    position_seconds: pos_i64 as u64,
                    duration_seconds: dur_i64 as u64,
                    updated_at,
                })
            })
            .optional()?;
        Ok(res)
    }

    pub fn delete_position(&self, video_id: &str) -> Result<(), StorageError> {
        let conn = self.conn.lock();
        conn.execute(
            "DELETE FROM video_positions WHERE video_id = ?1",
            params![video_id],
        )?;
        Ok(())
    }

    pub fn clear_all_positions(&self) -> Result<(), StorageError> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM video_positions", [])?;
        Ok(())
    }

    // Bookmarks
    pub fn add_bookmark(&self, bookmark: &Bookmark) -> Result<i64, StorageError> {
        let conn = self.conn.lock();
        let now = bookmark.created_at.to_rfc3339();
        conn.execute(
            r#"
            INSERT INTO bookmarks (video_id, title, timestamp_seconds, note, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5)
            "#,
            params![
                bookmark.video_id,
                bookmark.title,
                bookmark.timestamp_seconds as i64,
                bookmark.note,
                now
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn get_bookmarks(&self, video_id: &str) -> Result<Vec<Bookmark>, StorageError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, video_id, title, timestamp_seconds, note, created_at FROM bookmarks WHERE video_id = ?1 ORDER BY timestamp_seconds ASC",
        )?;
        let rows = stmt.query_map(params![video_id], |row| {
            let created_str: String = row.get(5)?;
            let created_at = DateTime::parse_from_rfc3339(&created_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());
            let ts_i64: i64 = row.get(3)?;
            Ok(Bookmark {
                id: row.get(0)?,
                video_id: row.get(1)?,
                title: row.get(2)?,
                timestamp_seconds: ts_i64 as u64,
                note: row.get(4)?,
                created_at,
            })
        })?;

        let mut list = Vec::new();
        for b in rows {
            list.push(b?);
        }
        Ok(list)
    }

    pub fn get_all_bookmarks(&self) -> Result<Vec<Bookmark>, StorageError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, video_id, title, timestamp_seconds, note, created_at FROM bookmarks ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            let created_str: String = row.get(5)?;
            let created_at = DateTime::parse_from_rfc3339(&created_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());
            let ts_i64: i64 = row.get(3)?;
            Ok(Bookmark {
                id: row.get(0)?,
                video_id: row.get(1)?,
                title: row.get(2)?,
                timestamp_seconds: ts_i64 as u64,
                note: row.get(4)?,
                created_at,
            })
        })?;

        let mut list = Vec::new();
        for b in rows {
            list.push(b?);
        }
        Ok(list)
    }

    pub fn delete_bookmark(&self, id: i64) -> Result<(), StorageError> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM bookmarks WHERE id = ?1", params![id])?;
        Ok(())
    }

    // Playlists
    pub fn create_playlist(&self, playlist: &Playlist) -> Result<(), StorageError> {
        let conn = self.conn.lock();
        let now = Utc::now().to_rfc3339();
        let is_local_int = if playlist.is_local { 1 } else { 0 };

        conn.execute(
            r#"
            INSERT INTO playlists
                (id, title, description, item_count, thumbnail_url, is_local, account_id, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            ON CONFLICT(id) DO UPDATE SET
                title = excluded.title,
                description = excluded.description,
                item_count = excluded.item_count,
                thumbnail_url = excluded.thumbnail_url,
                updated_at = excluded.updated_at
            "#,
            params![
                playlist.id,
                playlist.title,
                playlist.description,
                playlist.item_count,
                playlist.thumbnail_url,
                is_local_int,
                playlist.account_id,
                now,
                now
            ],
        )?;
        Ok(())
    }

    pub fn get_playlists(
        &self,
        account_id: Option<&str>,
    ) -> Result<Vec<Playlist>, StorageError> {
        let conn = self.conn.lock();
        let query = match account_id {
            Some(_) => {
                "SELECT id, title, description, item_count, thumbnail_url, is_local, account_id, created_at, updated_at
                 FROM playlists WHERE account_id = ?1 OR is_local = 1 ORDER BY updated_at DESC"
            }
            None => {
                "SELECT id, title, description, item_count, thumbnail_url, is_local, account_id, created_at, updated_at
                 FROM playlists WHERE is_local = 1 ORDER BY updated_at DESC"
            }
        };

        let map_row = |row: &rusqlite::Row| {
            let created_str: String = row.get(7)?;
            let updated_str: String = row.get(8)?;
            let created_at = DateTime::parse_from_rfc3339(&created_str)
                .map(|dt| dt.with_timezone(&Utc))
                .ok();
            let updated_at = DateTime::parse_from_rfc3339(&updated_str)
                .map(|dt| dt.with_timezone(&Utc))
                .ok();
            let is_local_int: i32 = row.get(5)?;
            Ok(Playlist {
                id: row.get(0)?,
                title: row.get(1)?,
                description: row.get(2)?,
                item_count: row.get(3)?,
                thumbnail_url: row.get(4)?,
                is_local: is_local_int != 0,
                account_id: row.get(6)?,
                channel_id: None,
                channel_title: None,
                created_at,
                updated_at,
            })
        };

        let mut playlists = Vec::new();
        if let Some(acc) = account_id {
            let mut stmt = conn.prepare(query)?;
            let rows = stmt.query_map(params![acc], map_row)?;
            for p in rows {
                playlists.push(p?);
            }
        } else {
            let mut stmt = conn.prepare(query)?;
            let rows = stmt.query_map([], map_row)?;
            for p in rows {
                playlists.push(p?);
            }
        }
        Ok(playlists)
    }

    pub fn get_playlist_by_id(
        &self,
        playlist_id: &str,
    ) -> Result<Option<Playlist>, StorageError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, title, description, item_count, thumbnail_url, is_local, account_id, created_at, updated_at
             FROM playlists WHERE id = ?1",
        )?;
        let res = stmt
            .query_row(params![playlist_id], |row| {
                let created_str: String = row.get(7)?;
                let updated_str: String = row.get(8)?;
                let created_at = DateTime::parse_from_rfc3339(&created_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .ok();
                let updated_at = DateTime::parse_from_rfc3339(&updated_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .ok();
                let is_local_int: i32 = row.get(5)?;
                Ok(Playlist {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    description: row.get(2)?,
                    item_count: row.get(3)?,
                    thumbnail_url: row.get(4)?,
                    is_local: is_local_int != 0,
                    account_id: row.get(6)?,
                    channel_id: None,
                    channel_title: None,
                    created_at,
                    updated_at,
                })
            })
            .optional()?;
        Ok(res)
    }

    pub fn delete_playlist(&self, playlist_id: &str) -> Result<(), StorageError> {
        let conn = self.conn.lock();
        conn.execute(
            "DELETE FROM playlists WHERE id = ?1",
            params![playlist_id],
        )?;
        Ok(())
    }

    pub fn add_playlist_item(&self, item: &PlaylistItem) -> Result<(), StorageError> {
        let conn = self.conn.lock();
        let now = item.added_at.to_rfc3339();
        conn.execute(
            r#"
            INSERT INTO playlist_items (id, playlist_id, video_id, title, thumbnail_url, position, added_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            ON CONFLICT(id) DO UPDATE SET
                title = excluded.title,
                thumbnail_url = excluded.thumbnail_url,
                position = excluded.position
            "#,
            params![
                item.id,
                item.playlist_id,
                item.video_id,
                item.title,
                item.thumbnail_url,
                item.position,
                now
            ],
        )?;

        // update item count
        conn.execute(
            "UPDATE playlists SET item_count = (SELECT COUNT(*) FROM playlist_items WHERE playlist_id = ?1), updated_at = ?2 WHERE id = ?1",
            params![item.playlist_id, now],
        )?;

        Ok(())
    }

    pub fn get_playlist_items(
        &self,
        playlist_id: &str,
    ) -> Result<Vec<PlaylistItem>, StorageError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, playlist_id, video_id, title, thumbnail_url, position, added_at
             FROM playlist_items WHERE playlist_id = ?1 ORDER BY position ASC",
        )?;
        let rows = stmt.query_map(params![playlist_id], |row| {
            let added_str: String = row.get(6)?;
            let added_at = DateTime::parse_from_rfc3339(&added_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());
            Ok(PlaylistItem {
                id: row.get(0)?,
                playlist_id: row.get(1)?,
                video_id: row.get(2)?,
                title: row.get(3)?,
                thumbnail_url: row.get(4)?,
                position: row.get(5)?,
                added_at,
            })
        })?;

        let mut items = Vec::new();
        for it in rows {
            items.push(it?);
        }
        Ok(items)
    }

    pub fn remove_playlist_item(&self, item_id: &str) -> Result<(), StorageError> {
        let conn = self.conn.lock();
        let playlist_id: Option<String> = conn
            .query_row(
                "SELECT playlist_id FROM playlist_items WHERE id = ?1",
                params![item_id],
                |r| r.get(0),
            )
            .optional()?;

        if let Some(pid) = playlist_id {
            conn.execute(
                "DELETE FROM playlist_items WHERE id = ?1",
                params![item_id],
            )?;
            let now = Utc::now().to_rfc3339();
            conn.execute(
                "UPDATE playlists SET item_count = (SELECT COUNT(*) FROM playlist_items WHERE playlist_id = ?1), updated_at = ?2 WHERE id = ?1",
                params![pid, now],
            )?;
        }

        Ok(())
    }

    // Channel Groups
    pub fn create_channel_group(
        &self,
        id: &str,
        name: &str,
        color: Option<&str>,
        icon: Option<&str>,
    ) -> Result<(), StorageError> {
        let conn = self.conn.lock();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            r#"
            INSERT INTO channel_groups (id, name, color, icon, sort_order, created_at)
            VALUES (?1, ?2, ?3, ?4, (SELECT COALESCE(MAX(sort_order), 0) + 1 FROM channel_groups), ?5)
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                color = excluded.color,
                icon = excluded.icon
            "#,
            params![id, name, color, icon, now],
        )?;
        Ok(())
    }

    pub fn get_channel_groups(&self) -> Result<Vec<ChannelGroup>, StorageError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, name, color, icon, sort_order, created_at FROM channel_groups ORDER BY sort_order ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            let created_str: String = row.get(5)?;
            let created_at = DateTime::parse_from_rfc3339(&created_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());
            Ok(ChannelGroup {
                id: row.get(0)?,
                name: row.get(1)?,
                color: row.get(2)?,
                icon: row.get(3)?,
                sort_order: row.get(4)?,
                created_at,
            })
        })?;

        let mut groups = Vec::new();
        for g in rows {
            groups.push(g?);
        }
        Ok(groups)
    }

    pub fn delete_channel_group(&self, id: &str) -> Result<(), StorageError> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM channel_groups WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn add_channel_to_group(
        &self,
        group_id: &str,
        channel_id: &str,
        title: &str,
        avatar: Option<&str>,
    ) -> Result<(), StorageError> {
        let conn = self.conn.lock();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            r#"
            INSERT INTO channel_group_items (group_id, channel_id, channel_title, avatar_url, added_at)
            VALUES (?1, ?2, ?3, ?4, ?5)
            ON CONFLICT(group_id, channel_id) DO UPDATE SET
                channel_title = excluded.channel_title,
                avatar_url = excluded.avatar_url
            "#,
            params![group_id, channel_id, title, avatar, now],
        )?;
        Ok(())
    }

    pub fn get_channels_in_group(
        &self,
        group_id: &str,
    ) -> Result<Vec<ChannelGroupItem>, StorageError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT group_id, channel_id, channel_title, avatar_url, added_at FROM channel_group_items WHERE group_id = ?1 ORDER BY added_at DESC",
        )?;
        let rows = stmt.query_map(params![group_id], |row| {
            let added_str: String = row.get(4)?;
            let added_at = DateTime::parse_from_rfc3339(&added_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());
            Ok(ChannelGroupItem {
                group_id: row.get(0)?,
                channel_id: row.get(1)?,
                channel_title: row.get(2)?,
                avatar_url: row.get(3)?,
                added_at,
            })
        })?;

        let mut items = Vec::new();
        for it in rows {
            items.push(it?);
        }
        Ok(items)
    }

    pub fn remove_channel_from_group(
        &self,
        group_id: &str,
        channel_id: &str,
    ) -> Result<(), StorageError> {
        let conn = self.conn.lock();
        conn.execute(
            "DELETE FROM channel_group_items WHERE group_id = ?1 AND channel_id = ?2",
            params![group_id, channel_id],
        )?;
        Ok(())
    }

    // Quota Tracking
    pub fn record_quota_usage(
        &self,
        pool: QuotaPool,
        units: u32,
    ) -> Result<QuotaStatus, StorageError> {
        let conn = self.conn.lock();
        let today = Utc::now().format("%Y-%m-%d").to_string();
        let pool_str = pool.to_string();

        conn.execute(
            r#"
            INSERT INTO quota_tracking (date, pool, units_used)
            VALUES (?1, ?2, ?3)
            ON CONFLICT(date, pool) DO UPDATE SET units_used = units_used + excluded.units_used
            "#,
            params![today, pool_str, units],
        )?;

        let used_today: u32 = conn.query_row(
            "SELECT units_used FROM quota_tracking WHERE date = ?1 AND pool = ?2",
            params![today, pool_str],
            |r| r.get(0),
        )?;

        let daily_limit: u32 = match pool {
            QuotaPool::General => 10_000,
            QuotaPool::Search => 100,
        };

        let remaining = daily_limit.saturating_sub(used_today);
        let tomorrow_midnight = Utc::now()
            .date_naive()
            .succ_opt()
            .and_then(|d| d.and_hms_opt(0, 0, 0))
            .map(|dt| DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc))
            .unwrap_or_else(Utc::now);

        Ok(QuotaStatus {
            pool,
            used_today,
            daily_limit,
            remaining,
            reset_at: tomorrow_midnight,
        })
    }

    pub fn get_quota_status(&self, pool: QuotaPool) -> Result<QuotaStatus, StorageError> {
        let conn = self.conn.lock();
        let today = Utc::now().format("%Y-%m-%d").to_string();
        let pool_str = pool.to_string();

        let used_today: u32 = conn
            .query_row(
                "SELECT units_used FROM quota_tracking WHERE date = ?1 AND pool = ?2",
                params![today, pool_str],
                |r| r.get(0),
            )
            .optional()?
            .unwrap_or(0);

        let daily_limit: u32 = match pool {
            QuotaPool::General => 10_000,
            QuotaPool::Search => 100,
        };

        let remaining = daily_limit.saturating_sub(used_today);
        let tomorrow_midnight = Utc::now()
            .date_naive()
            .succ_opt()
            .and_then(|d| d.and_hms_opt(0, 0, 0))
            .map(|dt| DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc))
            .unwrap_or_else(Utc::now);

        Ok(QuotaStatus {
            pool,
            used_today,
            daily_limit,
            remaining,
            reset_at: tomorrow_midnight,
        })
    }

    // Metadata Cache
    pub fn cache_set(
        &self,
        key: &str,
        value: &str,
        ttl: Duration,
    ) -> Result<(), StorageError> {
        let conn = self.conn.lock();
        let now = Utc::now();
        let expires_at = (now + chrono::Duration::from_std(ttl).unwrap_or(chrono::Duration::days(1)))
            .to_rfc3339();
        let created_at = now.to_rfc3339();

        conn.execute(
            r#"
            INSERT INTO metadata_cache (key, value, expires_at, created_at)
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(key) DO UPDATE SET
                value = excluded.value,
                expires_at = excluded.expires_at,
                created_at = excluded.created_at
            "#,
            params![key, value, expires_at, created_at],
        )?;
        Ok(())
    }

    pub fn cache_get(&self, key: &str) -> Result<Option<String>, StorageError> {
        let conn = self.conn.lock();
        let now = Utc::now().to_rfc3339();
        let mut stmt = conn.prepare(
            "SELECT value FROM metadata_cache WHERE key = ?1 AND expires_at > ?2",
        )?;
        let res = stmt.query_row(params![key, now], |r| r.get(0)).optional()?;
        Ok(res)
    }

    pub fn cache_cleanup(&self) -> Result<usize, StorageError> {
        let conn = self.conn.lock();
        let now = Utc::now().to_rfc3339();
        let deleted = conn.execute(
            "DELETE FROM metadata_cache WHERE expires_at <= ?1",
            params![now],
        )?;
        Ok(deleted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fluxtube_core::UserProfile;

    #[test]
    fn test_open_in_memory_and_integrity() {
        let storage = StorageManager::open_in_memory().expect("open in memory");
        assert!(storage.integrity_check().expect("check integrity"));
    }

    #[test]
    fn test_settings_crud() {
        let storage = StorageManager::open_in_memory().unwrap();
        assert_eq!(storage.get_setting("theme").unwrap(), None);

        storage.set_setting("theme", "graphite_dark").unwrap();
        assert_eq!(
            storage.get_setting("theme").unwrap(),
            Some("graphite_dark".to_string())
        );

        storage.set_setting("theme", "arctic_light").unwrap();
        assert_eq!(
            storage.get_setting("theme").unwrap(),
            Some("arctic_light".to_string())
        );

        storage.delete_setting("theme").unwrap();
        assert_eq!(storage.get_setting("theme").unwrap(), None);
    }

    #[test]
    fn test_accounts_and_foreign_key_cascade() {
        let storage = StorageManager::open_in_memory().unwrap();
        let user = UserProfile {
            id: "user_123".to_string(),
            email: "test@example.com".to_string(),
            name: "John Doe".to_string(),
            picture: Some("https://example.com/pic.png".to_string()),
            is_active: true,
            created_at: Utc::now(),
        };

        storage.save_account(&user, true).unwrap();
        let active = storage.get_active_account().unwrap().expect("has active");
        assert_eq!(active.id, "user_123");
        assert_eq!(active.email, "test@example.com");

        // Add history tied to this account
        let entry = HistoryEntry {
            id: None,
            video_id: "vid_abc".to_string(),
            title: "Test Video".to_string(),
            channel_title: "Test Channel".to_string(),
            thumbnail_url: "thumb.jpg".to_string(),
            duration_seconds: 300,
            watched_at: Utc::now(),
            playback_position_seconds: 150,
            completed: false,
            account_id: Some("user_123".to_string()),
        };
        storage.add_history_entry(&entry).unwrap();

        let hist = storage.get_history(Some("user_123"), 10).unwrap();
        assert_eq!(hist.len(), 1);
        assert_eq!(hist[0].video_id, "vid_abc");

        // Delete account -> foreign key cascade should delete history
        storage.delete_account("user_123").unwrap();
        let hist_after = storage.get_history(Some("user_123"), 10).unwrap();
        assert_eq!(hist_after.len(), 0);
    }

    #[test]
    fn test_search_history() {
        let storage = StorageManager::open_in_memory().unwrap();
        storage.add_search_query("rust lang").unwrap();
        storage.add_search_query("iced tutorial").unwrap();
        storage.add_search_query("rust lang").unwrap(); // upsert timestamp

        let searches = storage.get_search_history(10).unwrap();
        assert_eq!(searches.len(), 2);
        assert_eq!(searches[0], "rust lang");
        assert_eq!(searches[1], "iced tutorial");

        storage.remove_search_query("rust lang").unwrap();
        assert_eq!(storage.get_search_history(10).unwrap(), vec!["iced tutorial"]);

        storage.clear_search_history().unwrap();
        assert!(storage.get_search_history(10).unwrap().is_empty());
    }

    #[test]
    fn test_video_positions_and_bookmarks() {
        let storage = StorageManager::open_in_memory().unwrap();
        storage.save_position("v1", 120, 600).unwrap();
        let pos = storage.get_position("v1").unwrap().expect("position exists");
        assert_eq!(pos.position_seconds, 120);
        assert_eq!(pos.duration_seconds, 600);

        let bm = Bookmark {
            id: 0,
            video_id: "v1".to_string(),
            title: "Intro segment".to_string(),
            timestamp_seconds: 30,
            note: Some("Check this".to_string()),
            created_at: Utc::now(),
        };
        let b_id = storage.add_bookmark(&bm).unwrap();
        assert!(b_id > 0);

        let bookmarks = storage.get_bookmarks("v1").unwrap();
        assert_eq!(bookmarks.len(), 1);
        assert_eq!(bookmarks[0].title, "Intro segment");

        storage.delete_bookmark(b_id).unwrap();
        assert!(storage.get_bookmarks("v1").unwrap().is_empty());
    }

    #[test]
    fn test_playlists_and_items() {
        let storage = StorageManager::open_in_memory().unwrap();
        let pl = Playlist {
            id: "pl_favorites".to_string(),
            title: "Favorites".to_string(),
            description: "My favorite videos".to_string(),
            item_count: 0,
            thumbnail_url: None,
            is_local: true,
            account_id: None,
            channel_id: None,
            channel_title: None,
            created_at: None,
            updated_at: None,
        };
        storage.create_playlist(&pl).unwrap();

        let item1 = PlaylistItem {
            id: "item_1".to_string(),
            playlist_id: "pl_favorites".to_string(),
            video_id: "vid_1".to_string(),
            title: "Video 1".to_string(),
            thumbnail_url: "thumb1.jpg".to_string(),
            position: 1,
            added_at: Utc::now(),
        };
        storage.add_playlist_item(&item1).unwrap();

        let items = storage.get_playlist_items("pl_favorites").unwrap();
        assert_eq!(items.len(), 1);

        let updated_pl = storage.get_playlist_by_id("pl_favorites").unwrap().unwrap();
        assert_eq!(updated_pl.item_count, 1);

        storage.remove_playlist_item("item_1").unwrap();
        let updated_pl2 = storage.get_playlist_by_id("pl_favorites").unwrap().unwrap();
        assert_eq!(updated_pl2.item_count, 0);
    }

    #[test]
    fn test_channel_groups() {
        let storage = StorageManager::open_in_memory().unwrap();
        storage.create_channel_group("tech", "Tech Channels", Some("#00FF00"), Some("laptop")).unwrap();

        let groups = storage.get_channel_groups().unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].name, "Tech Channels");

        storage.add_channel_to_group("tech", "ch_1", "Rust Official", Some("avatar.png")).unwrap();
        let channels = storage.get_channels_in_group("tech").unwrap();
        assert_eq!(channels.len(), 1);
        assert_eq!(channels[0].channel_title, "Rust Official");

        storage.remove_channel_from_group("tech", "ch_1").unwrap();
        assert!(storage.get_channels_in_group("tech").unwrap().is_empty());
    }

    #[test]
    fn test_quota_tracking() {
        let storage = StorageManager::open_in_memory().unwrap();
        let status = storage.get_quota_status(QuotaPool::General).unwrap();
        assert_eq!(status.used_today, 0);
        assert_eq!(status.daily_limit, 10_000);
        assert_eq!(status.remaining, 10_000);

        let updated = storage.record_quota_usage(QuotaPool::General, 250).unwrap();
        assert_eq!(updated.used_today, 250);
        assert_eq!(updated.remaining, 9750);
    }

    #[test]
    fn test_metadata_cache() {
        let storage = StorageManager::open_in_memory().unwrap();
        assert_eq!(storage.cache_get("video_metadata_1").unwrap(), None);

        storage
            .cache_set(
                "video_metadata_1",
                "{\"title\":\"Test\"}",
                Duration::from_secs(60),
            )
            .unwrap();
        assert_eq!(
            storage.cache_get("video_metadata_1").unwrap(),
            Some("{\"title\":\"Test\"}".to_string())
        );

        // Immediate expired cache entry
        storage
            .cache_set(
                "expired_key",
                "expired_val",
                Duration::from_secs(0),
            )
            .unwrap();
        let cleaned = storage.cache_cleanup().unwrap();
        assert!(cleaned >= 1);
        assert_eq!(storage.cache_get("expired_key").unwrap(), None);
    }

    #[test]
    fn test_file_database() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test_fluxtube.sqlite");

        {
            let storage = StorageManager::open(&db_path).expect("open file db");
            storage.set_setting("app_version", "1.0.0").unwrap();
            assert_eq!(
                storage.get_setting("app_version").unwrap(),
                Some("1.0.0".to_string())
            );
        }

        // Reopen existing db
        {
            let storage = StorageManager::open(&db_path).expect("reopen file db");
            assert_eq!(
                storage.get_setting("app_version").unwrap(),
                Some("1.0.0".to_string())
            );
        }
    }
}
