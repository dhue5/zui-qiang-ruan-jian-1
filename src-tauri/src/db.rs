use rusqlite::{params, Connection};
use std::fs;
use std::path::{Path, PathBuf};

pub fn app_data_dir() -> PathBuf {
    let base = std::env::var("APPDATA").unwrap_or_else(|_| ".".into());
    let dir = PathBuf::from(base).join("com.zuiqiangruanjian.desktop");
    fs::create_dir_all(&dir).ok();
    dir
}

pub fn default_media_dir() -> PathBuf {
    app_data_dir().join("media")
}

pub fn media_root() -> PathBuf {
    let settings = app_data_dir().join("settings.json");
    if let Ok(text) = fs::read_to_string(&settings) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
            if let Some(p) = v["media_root"].as_str() {
                let pb = PathBuf::from(p);
                fs::create_dir_all(&pb).ok();
                return pb;
            }
        }
    }
    default_media_dir()
}

pub fn set_media_root(path: &str) -> std::io::Result<()> {
    let dir = PathBuf::from(path);
    fs::create_dir_all(&dir)?;
    let settings = app_data_dir().join("settings.json");
    let mut v = serde_json::json!({});
    if let Ok(text) = fs::read_to_string(&settings) {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&text) {
            v = parsed;
        }
    }
    if let Some(obj) = v.as_object_mut() {
        obj.insert("media_root".to_string(), serde_json::Value::String(path.to_string()));
    }
    fs::write(settings, serde_json::to_string_pretty(&v).unwrap_or_default())
}

pub fn chat_fallback() -> (Option<String>, Option<String>) {
    let settings = app_data_dir().join("settings.json");
    if let Ok(text) = fs::read_to_string(&settings) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
            return (
                v["chat_api_key"].as_str().map(|s| s.to_string()),
                v["chat_base_url"].as_str().map(|s| s.to_string()),
            );
        }
    }
    (None, None)
}

pub fn media_provider_config() -> (Option<String>, Option<String>) {
    let settings = app_data_dir().join("settings.json");
    if let Ok(text) = fs::read_to_string(&settings) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
            return (
                v["media_base_url"].as_str().map(|s| s.to_string()),
                v["media_api_key"].as_str().map(|s| s.to_string()),
            );
        }
    }
    (None, None)
}

pub fn auto_install_updates() -> bool {
    let settings = app_data_dir().join("settings.json");
    if let Ok(text) = fs::read_to_string(&settings) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
            return v["auto_install_updates"].as_bool().unwrap_or(false);
        }
    }
    false
}

pub fn update_manifest_url() -> Option<String> {
    let settings = app_data_dir().join("settings.json");
    if let Ok(text) = fs::read_to_string(&settings) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
            let url = v["update_manifest_url"].as_str().map(|s| s.to_string());
            if let Some(u) = url {
                if !u.trim().is_empty() {
                    return Some(u);
                }
            }
        }
    }
    None
}

pub fn is_path_allowed(path: &Path) -> bool {
    let Ok(canon) = path.canonicalize() else { return false };
    let roots = [app_data_dir(), media_root()];
    roots.iter().any(|r| {
        if let Ok(rc) = r.canonicalize() {
            canon.starts_with(rc)
        } else {
            false
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_and_crud_roundtrip() {
        let conn = Connection::open_in_memory().expect("open memory db");
        init(&conn).expect("init schema");

        let now = now();
        conn.execute(
            "INSERT INTO conversations (title, created_at, updated_at, model) VALUES ('t', ?1, ?1, 'm')",
            params![now],
        )
        .expect("insert conversation");
        let id = conn.last_insert_rowid();

        let listed = list_conversations(&conn).expect("list");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0]["title"], "t");

        conn.execute(
            "INSERT INTO messages (conversation_id, role, content, created_at, model, deleted)
             VALUES (?1, 'user', 'hi', ?2, 'm', 0)",
            params![id, now],
        )
        .expect("insert message");
        let msgs = messages_for(&conn, id).expect("messages");
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0]["content"], "hi");

        let mid = msgs[0]["id"].as_i64().expect("message id");
        conn.execute("UPDATE messages SET deleted = 1 WHERE id = ?1", params![mid]).expect("soft delete");
        let msgs = messages_for(&conn, id).expect("messages after delete");
        assert_eq!(msgs.len(), 0);
    }

    #[test]
    fn media_root_persists() {
        let tmp = std::env::temp_dir().join(format!("pf-test-{}", std::process::id()));
        set_media_root(tmp.to_str().unwrap()).expect("set media root");
        let got = media_root();
        assert!(got.to_string_lossy().starts_with(tmp.to_string_lossy().as_ref()));
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn default_media_dir_is_under_appdata() {
        assert!(default_media_dir().to_string_lossy().ends_with("media"));
    }
}

pub fn open_db() -> rusqlite::Result<Connection> {
    let db_path = app_data_dir().join("chats.db");
    Connection::open(db_path)
}

pub fn init(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS conversations (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            created_at TEXT,
            updated_at TEXT,
            model TEXT
        );
        CREATE TABLE IF NOT EXISTS messages (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            conversation_id INTEGER,
            role TEXT,
            content TEXT,
            created_at TEXT,
            model TEXT,
            deleted BOOLEAN DEFAULT 0,
            media_type TEXT,
            media_url TEXT,
            media_metadata TEXT
        );
        CREATE TABLE IF NOT EXISTS generation_tasks (
            id TEXT PRIMARY KEY,
            message_id TEXT,
            conversation_id TEXT,
            status TEXT,
            progress TEXT,
            error TEXT,
            timestamp INTEGER,
            completed_at INTEGER,
            model_id TEXT,
            model_provider TEXT,
            model_type TEXT,
            provider TEXT,
            request_data TEXT,
            result_url TEXT,
            result_data TEXT
        );
        CREATE TABLE IF NOT EXISTS canvases (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT,
            content TEXT,
            source_type TEXT,
            source_id INTEGER,
            created_at TEXT,
            updated_at TEXT,
            tags TEXT
        );
        CREATE TABLE IF NOT EXISTS notes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT,
            content TEXT,
            source_type TEXT,
            source_id INTEGER,
            created_at TEXT,
            updated_at TEXT,
            tags TEXT
        );
        "#,
    )
}

pub fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

pub fn list_conversations(conn: &Connection) -> rusqlite::Result<Vec<serde_json::Value>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, created_at, updated_at, model FROM conversations ORDER BY updated_at DESC",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(serde_json::json!({
            "id": r.get::<_, i64>(0)?,
            "title": r.get::<_, String>(1)?,
            "created_at": r.get::<_, String>(2)?,
            "updated_at": r.get::<_, String>(3)?,
            "model": r.get::<_, String>(4)?
        }))
    })?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

pub fn messages_for(conn: &Connection, conversation_id: i64) -> rusqlite::Result<Vec<serde_json::Value>> {
    let mut stmt = conn.prepare(
        "SELECT id, conversation_id, role, content, created_at, model, media_type, media_url, media_metadata
         FROM messages WHERE conversation_id = ?1 AND deleted = 0 ORDER BY id",
    )?;
    let rows = stmt.query_map(params![conversation_id], |r| {
        Ok(serde_json::json!({
            "id": r.get::<_, i64>(0)?,
            "conversation_id": r.get::<_, i64>(1)?,
            "role": r.get::<_, String>(2)?,
            "content": r.get::<_, String>(3)?,
            "created_at": r.get::<_, String>(4)?,
            "model": r.get::<_, String>(5)?,
            "media_type": r.get::<_, Option<String>>(6)?,
            "media_url": r.get::<_, Option<String>>(7)?,
            "media_metadata": r.get::<_, Option<String>>(8)?
        }))
    })?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}
