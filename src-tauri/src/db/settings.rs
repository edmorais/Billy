//! App-wide settings, such as the theme, stored as JSON values by key.

use crate::error::Result;
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Map, Value};

/// All settings as one JSON object.
pub fn get_all(conn: &Connection) -> Result<Value> {
    let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
    let mut map = Map::new();
    for row in rows {
        let (key, value) = row?;
        map.insert(key, serde_json::from_str(&value).unwrap_or(Value::Null));
    }
    Ok(Value::Object(map))
}

/// One setting, or `None` if it was never set.
pub fn get(conn: &Connection, key: &str) -> Result<Option<Value>> {
    let value: Option<String> = conn
        .query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| r.get(0))
        .optional()?;
    Ok(value.map(|v| serde_json::from_str(&v).unwrap_or(Value::Null)))
}

/// Whether Billy keeps a history of opened documents. On unless turned off in Settings.
pub fn history_enabled(conn: &Connection) -> Result<bool> {
    Ok(get(conn, "history")?.and_then(|h| h.get("enabled").and_then(Value::as_bool)).unwrap_or(true))
}

pub fn set(conn: &Connection, key: &str, value: &Value) -> Result<()> {
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT (key) DO UPDATE SET value = excluded.value",
        params![key, value.to_string()],
    )?;
    Ok(())
}
