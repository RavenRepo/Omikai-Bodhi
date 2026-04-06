use rusqlite::Connection;

use crate::Result;

/// Current schema version for migrations.
pub const SCHEMA_VERSION: i32 = 1;

/// Initialize the database schema.
pub fn init_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER PRIMARY KEY
        );

        CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY,
            name TEXT,
            model TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            total_tokens INTEGER DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS messages (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            tool_calls TEXT,
            created_at TEXT NOT NULL,
            FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_messages_session_id ON messages(session_id);
        CREATE INDEX IF NOT EXISTS idx_sessions_updated_at ON sessions(updated_at);
        "#,
    )?;

    // Set schema version if not set
    let version: Option<i32> =
        conn.query_row("SELECT version FROM schema_version LIMIT 1", [], |row| row.get(0)).ok();

    if version.is_none() {
        conn.execute("INSERT INTO schema_version (version) VALUES (?1)", [SCHEMA_VERSION])?;
    }

    Ok(())
}

/// Run any pending migrations.
pub fn migrate(conn: &Connection) -> Result<()> {
    let current_version: i32 = conn
        .query_row("SELECT version FROM schema_version LIMIT 1", [], |row| row.get(0))
        .unwrap_or(0);

    if current_version < SCHEMA_VERSION {
        // Future migrations go here
        // match current_version {
        //     0 => migrate_v0_to_v1(conn)?,
        //     1 => migrate_v1_to_v2(conn)?,
        //     _ => {}
        // }

        conn.execute("UPDATE schema_version SET version = ?1", [SCHEMA_VERSION])?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_schema() {
        let conn = Connection::open_in_memory().unwrap();
        init_schema(&conn).unwrap();

        // Verify tables exist
        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert!(tables.contains(&"sessions".to_string()));
        assert!(tables.contains(&"messages".to_string()));
        assert!(tables.contains(&"schema_version".to_string()));
    }

    #[test]
    fn test_schema_version() {
        let conn = Connection::open_in_memory().unwrap();
        init_schema(&conn).unwrap();

        let version: i32 =
            conn.query_row("SELECT version FROM schema_version", [], |row| row.get(0)).unwrap();

        assert_eq!(version, SCHEMA_VERSION);
    }
}
