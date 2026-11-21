//! App-wide settings, such as the theme, stored as JSON values by key.

use crate::error::Result;
use rusqlite::{Connection, params};
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

pub fn set(conn: &Connection, key: &str, value: &Value) -> Result<()> {
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT (key) DO UPDATE SET value = excluded.value",
        params![key, value.to_string()],
    )?;
    Ok(())
}
