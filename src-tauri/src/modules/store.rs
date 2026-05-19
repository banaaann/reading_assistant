use std::path::{Path, PathBuf};

use anyhow::Context;
use chrono::Utc;
use rusqlite::{params, Connection};
use uuid::Uuid;

use crate::modules::models::{
    AppConfig, LookupPayload, StarredEntry, StarredQuery, ToggleStarredResponse,
};

pub struct Store {
    db_path: PathBuf,
}

impl Store {
    pub fn new(base_dir: &Path) -> anyhow::Result<Self> {
        std::fs::create_dir_all(base_dir).context("failed to create app data directory")?;
        let db_path = base_dir.join("reading-assistant.db");
        let store = Self { db_path };
        store.initialize()?;
        Ok(store)
    }

    pub fn config(&self) -> anyhow::Result<AppConfig> {
        let connection = self.connection()?;
        let mut config = AppConfig::default();
        let mut statement = connection
            .prepare("SELECT key, value FROM settings")
            .context("prepare settings query")?;
        let rows = statement.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;

        for row in rows {
            let (key, value) = row?;
            match key.as_str() {
                "hotkey" => config.hotkey = value,
                "trigger_source" => config.trigger_source = value,
                "auto_start" => config.auto_start = value == "true",
                "max_text_length" => {
                    if let Ok(number) = value.parse::<usize>() {
                        config.max_text_length = number;
                    }
                }
                "close_on_focus_loss" => config.close_on_focus_loss = value == "true",
                "popup_width" => {
                    if let Ok(number) = value.parse::<u32>() {
                        config.popup_width = number.clamp(300, 520);
                    }
                }
                "popup_height" => {
                    if let Ok(number) = value.parse::<u32>() {
                        config.popup_height = number.clamp(120, 680);
                    }
                }
                "popup_font_scale" => {
                    if let Ok(number) = value.parse::<f32>() {
                        config.popup_font_scale = number.clamp(0.8, 1.15);
                    }
                }
                _ => {}
            }
        }

        Ok(config)
    }

    pub fn save_config(&self, config: &AppConfig) -> anyhow::Result<AppConfig> {
        let connection = self.connection()?;
        let tx = connection.unchecked_transaction()?;
        upsert_setting(&tx, "hotkey", &config.hotkey)?;
        upsert_setting(&tx, "trigger_source", &config.trigger_source)?;
        upsert_setting(
            &tx,
            "auto_start",
            if config.auto_start { "true" } else { "false" },
        )?;
        upsert_setting(&tx, "max_text_length", &config.max_text_length.to_string())?;
        upsert_setting(
            &tx,
            "close_on_focus_loss",
            if config.close_on_focus_loss {
                "true"
            } else {
                "false"
            },
        )?;
        upsert_setting(&tx, "popup_width", &config.popup_width.to_string())?;
        upsert_setting(&tx, "popup_height", &config.popup_height.to_string())?;
        upsert_setting(
            &tx,
            "popup_font_scale",
            &config.popup_font_scale.to_string(),
        )?;
        tx.commit()?;
        Ok(config.clone())
    }

    pub fn is_starred(&self, normalized_text: &str) -> anyhow::Result<bool> {
        let connection = self.connection()?;
        let exists = connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM starred_entries WHERE normalized_text = ?1)",
            [normalized_text],
            |row| row.get::<_, i64>(0),
        )?;
        Ok(exists == 1)
    }

    pub fn toggle_starred(&self, payload: &LookupPayload) -> anyhow::Result<ToggleStarredResponse> {
        let connection = self.connection()?;
        let normalized_text = payload.normalized_text();
        if let Some(existing) =
            self.find_starred_by_normalized_with_connection(&connection, normalized_text)?
        {
            connection.execute(
                "DELETE FROM starred_entries WHERE normalized_text = ?1",
                [normalized_text],
            )?;
            return Ok(ToggleStarredResponse {
                is_starred: false,
                entry: Some(existing),
            });
        }

        let now = Utc::now().to_rfc3339();
        let entry = StarredEntry {
            id: Uuid::new_v4().to_string(),
            entry_type: payload.entry_type().to_string(),
            source_text: payload.source_text().to_string(),
            normalized_text: normalized_text.to_string(),
            display_title: payload.display_title(),
            payload: payload.clone(),
            created_at: now.clone(),
            updated_at: now,
        };

        connection.execute(
            "INSERT INTO starred_entries (
                id, entry_type, source_text, normalized_text, display_title, payload, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                entry.id,
                entry.entry_type,
                entry.source_text,
                entry.normalized_text,
                entry.display_title,
                serde_json::to_string(&entry.payload)?,
                entry.created_at,
                entry.updated_at
            ],
        )?;

        Ok(ToggleStarredResponse {
            is_starred: true,
            entry: Some(entry),
        })
    }

    pub fn list_starred(&self, query: StarredQuery) -> anyhow::Result<Vec<StarredEntry>> {
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT id, entry_type, source_text, normalized_text, display_title, payload, created_at, updated_at
             FROM starred_entries
             WHERE (?1 = '' OR display_title LIKE ?2 OR source_text LIKE ?2)
               AND (?3 = '' OR entry_type = ?3)
             ORDER BY updated_at DESC",
        )?;

        let search = query.search.unwrap_or_default();
        let filter = if search.is_empty() {
            String::new()
        } else {
            format!("%{search}%")
        };
        let entry_type = query.entry_type.unwrap_or_default();

        let rows = statement.query_map(params![search, filter, entry_type], map_starred_entry)?;
        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    pub fn remove_starred(&self, id: &str) -> anyhow::Result<()> {
        let connection = self.connection()?;
        connection.execute("DELETE FROM starred_entries WHERE id = ?1", [id])?;
        Ok(())
    }

    fn initialize(&self) -> anyhow::Result<()> {
        let connection = self.connection()?;
        connection.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS starred_entries (
                id TEXT PRIMARY KEY,
                entry_type TEXT NOT NULL,
                source_text TEXT NOT NULL,
                normalized_text TEXT NOT NULL UNIQUE,
                display_title TEXT NOT NULL,
                payload TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            ",
        )?;
        self.save_config(&self.config()?)?;
        Ok(())
    }

    fn connection(&self) -> anyhow::Result<Connection> {
        Connection::open(&self.db_path).context("failed to open sqlite database")
    }

    fn find_starred_by_normalized_with_connection(
        &self,
        connection: &Connection,
        normalized_text: &str,
    ) -> anyhow::Result<Option<StarredEntry>> {
        let mut statement = connection.prepare(
            "SELECT id, entry_type, source_text, normalized_text, display_title, payload, created_at, updated_at
             FROM starred_entries
             WHERE normalized_text = ?1",
        )?;

        let mut rows = statement.query([normalized_text])?;
        if let Some(row) = rows.next()? {
            Ok(Some(map_starred_row(row)?))
        } else {
            Ok(None)
        }
    }
}

fn upsert_setting(connection: &Connection, key: &str, value: &str) -> anyhow::Result<()> {
    connection.execute(
        "INSERT INTO settings(key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

fn map_starred_entry(row: &rusqlite::Row<'_>) -> rusqlite::Result<StarredEntry> {
    map_starred_row(row)
}

fn map_starred_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<StarredEntry> {
    let payload_text: String = row.get(5)?;
    let payload = serde_json::from_str::<LookupPayload>(&payload_text).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            payload_text.len(),
            rusqlite::types::Type::Text,
            Box::new(error),
        )
    })?;

    Ok(StarredEntry {
        id: row.get(0)?,
        entry_type: row.get(1)?,
        source_text: row.get(2)?,
        normalized_text: row.get(3)?,
        display_title: row.get(4)?,
        payload,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
    })
}
