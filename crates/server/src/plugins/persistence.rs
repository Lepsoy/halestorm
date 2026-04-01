use bevy::prelude::*;
use halestorm_common::types::TilePosition;
use rusqlite::Connection;

/// Database resource wrapping a SQLite connection.
/// Uses Mutex for thread-safety (Bevy requires Send + Sync for Resources).
#[derive(Resource)]
pub struct Database {
    conn: std::sync::Mutex<Connection>,
}

pub struct PersistencePlugin;

impl Plugin for PersistencePlugin {
    fn build(&self, app: &mut App) {
        let db_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../halestorm.db");
        let db = Database::open(db_path).expect("Failed to open database");
        app.insert_resource(db);
    }
}

/// Stored account data.
#[allow(dead_code)]
pub struct AccountRow {
    pub id: i64,
    pub username: String,
    pub password_hash: String,
}

/// Stored character data.
#[allow(dead_code)]
pub struct CharacterRow {
    pub id: i64,
    pub account_id: i64,
    pub name: String,
    pub class: String,
    pub position_x: i32,
    pub position_y: i32,
}

impl Database {
    pub fn open(path: &str) -> Result<Self, String> {
        let conn = Connection::open(path).map_err(|e| format!("SQLite open failed: {e}"))?;

        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS accounts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                username TEXT NOT NULL UNIQUE,
                password_hash TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE TABLE IF NOT EXISTS characters (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                account_id INTEGER NOT NULL REFERENCES accounts(id),
                name TEXT NOT NULL,
                class TEXT NOT NULL,
                position_x INTEGER NOT NULL DEFAULT 15,
                position_y INTEGER NOT NULL DEFAULT 10,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            ",
        )
        .map_err(|e| format!("Schema creation failed: {e}"))?;

        info!("Database opened at {path}");
        Ok(Self {
            conn: std::sync::Mutex::new(conn),
        })
    }

    /// Create a new account. Returns the account id.
    pub fn create_account(&self, username: &str, password_hash: &str) -> Result<i64, String> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO accounts (username, password_hash) VALUES (?1, ?2)",
            rusqlite::params![username, password_hash],
        )
        .map_err(|e| format!("{e}"))?;
        Ok(conn.last_insert_rowid())
    }

    /// Look up an account by username.
    pub fn get_account(&self, username: &str) -> Option<AccountRow> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, username, password_hash FROM accounts WHERE username = ?1",
            rusqlite::params![username],
            |row| {
                Ok(AccountRow {
                    id: row.get(0)?,
                    username: row.get(1)?,
                    password_hash: row.get(2)?,
                })
            },
        )
        .ok()
    }

    /// Create a new character for an account.
    pub fn create_character(
        &self,
        account_id: i64,
        name: &str,
        class: &str,
        spawn: TilePosition,
    ) -> Result<i64, String> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO characters (account_id, name, class, position_x, position_y) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![account_id, name, class, spawn.x, spawn.y],
        )
        .map_err(|e| format!("{e}"))?;
        Ok(conn.last_insert_rowid())
    }

    /// Get all characters for an account.
    pub fn get_characters(&self, account_id: i64) -> Vec<CharacterRow> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT id, account_id, name, class, position_x, position_y FROM characters WHERE account_id = ?1")
            .unwrap();
        stmt.query_map(rusqlite::params![account_id], |row| {
            Ok(CharacterRow {
                id: row.get(0)?,
                account_id: row.get(1)?,
                name: row.get(2)?,
                class: row.get(3)?,
                position_x: row.get(4)?,
                position_y: row.get(5)?,
            })
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect()
    }

    /// Get a specific character by id.
    pub fn get_character_by_id(&self, character_id: i64) -> Option<CharacterRow> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, account_id, name, class, position_x, position_y FROM characters WHERE id = ?1",
            rusqlite::params![character_id],
            |row| {
                Ok(CharacterRow {
                    id: row.get(0)?,
                    account_id: row.get(1)?,
                    name: row.get(2)?,
                    class: row.get(3)?,
                    position_x: row.get(4)?,
                    position_y: row.get(5)?,
                })
            },
        )
        .ok()
    }

    /// Update character position.
    pub fn save_character_position(&self, character_id: i64, pos: TilePosition) {
        let conn = self.conn.lock().unwrap();
        let _ = conn.execute(
            "UPDATE characters SET position_x = ?1, position_y = ?2 WHERE id = ?3",
            rusqlite::params![pos.x, pos.y, character_id],
        );
    }
}
