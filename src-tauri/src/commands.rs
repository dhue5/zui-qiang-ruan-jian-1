use crate::{db, prompts, AppState};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine as _;
use rusqlite::params;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter, State};

static CHAT_CANCEL: AtomicBool = AtomicBool::new(false);
static MEDIA_CANCEL: AtomicBool = AtomicBool::new(false);

fn sanitize_name(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' { c } else { '_' })
        .collect()
}

fn resolve_media_path(root: &Path, input: &str) -> Result<PathBuf, String> {
    let p = Path::new(input);
    if p.is_absolute() {
        if p.starts_with(&root) {
            return Ok(p.to_path_buf());
        }
        return Err("path escapes media root".into());
    }
    let joined = root.join(p);
    if !joined.starts_with(&root) {
        return Err("path escapes media root".into());
    }
    Ok(joined)
}

fn media_root_for(relative: &str) -> Result<PathBuf, String> {
    let root = db::media_root();
    resolve_media_path(&root, relative)
}

fn task_db() -> rusqlite::Connection {
    rusqlite::Connection::open(db::app_data_dir().join("chats.db"))
        .unwrap_or_else(|_| rusqlite::Connection::open_in_memory().expect("open in-memory db"))
}

fn update_task_status(id: &str, status: &str, progress: Option<&str>, error: Option<&str>, result_url: Option<&str>, result_data: Option<&str>, completed: bool) {
    let conn = task_db();
    let _ = conn.execute(
        "UPDATE generation_tasks SET status = ?1, progress = COALESCE(?2, progress), error = COALESCE(?3, error),
         result_url = COALESCE(?4, result_url), result_data = COALESCE(?5, result_data),
         completed_at = CASE WHEN ?6 THEN ?7 ELSE completed_at END WHERE id = ?8",
        params![status, progress, error, result_url, result_data, completed, chrono::Utc::now().timestamp_millis(), id],
    );
}

fn task_cancelled(id: &str) -> bool {
    let conn = task_db();
    conn.query_row(
        "SELECT status FROM generation_tasks WHERE id = ?1",
        params![id],
        |r| r.get::<_, String>(0),
    )
    .map(|s| s == "cancelled")
    .unwrap_or(false)
}

fn save_media_bytes(prefix: &str, ext: &str, bytes: &[u8]) -> Result<String, String> {
    use std::time::{SystemTime, UNIX_EPOCH};
    let dir = db::media_root().join("generated");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
    let path = dir.join(format!("{}-{}.{}", prefix, nanos, ext));
    std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().to_string())
}

async fn media_call_core(
    prompt: &str,
    model_type: &str,
    model_id: Option<&str>,
    api_key: &str,
    base_url: &str,
    progress: &mut (dyn FnMut(&str) + Send),
) -> Result<serde_json::Value, String> {
    if api_key.trim().is_empty() {
        return Err("API key is not configured for media generation".to_string());
    }
    let client = reqwest::Client::new();
    let base = base_url.trim_end_matches('/');
    if MEDIA_CANCEL.load(Ordering::Relaxed) {
        return Err("cancelled".to_string());
    }
    match model_type {
        "image" | "images" => {
            let url = format!("{}/images/generations", base);
            progress("正在请求图像生成...");
            let resp = client
                .post(&url)
                .bearer_auth(api_key)
                .json(&serde_json::json!({
                    "model": model_id.unwrap_or("gpt-image-1"),
                    "prompt": prompt,
                    "n": 1
                }))
                .send()
                .await
                .map_err(|e| e.to_string())?;
            if !resp.status().is_success() {
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();
                return Err(format!("image error {}: {}", status, text));
            }
            let v: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
            let item = &v["data"][0];
            if let Some(url) = item["url"].as_str() {
                Ok(serde_json::json!({"url": url}))
            } else if let Some(b64) = item["b64_json"].as_str() {
                let bytes = BASE64.decode(b64).map_err(|e| e.to_string())?;
                let path = save_media_bytes("image", "png", &bytes)?;
                Ok(serde_json::json!({"path": path}))
            } else {
                Err(format!("unexpected image response: {}", v))
            }
        }
        "text" => {
            let url = format!("{}/chat/completions", base);
            progress("正在请求文本生成...");
            let resp = client
                .post(&url)
                .bearer_auth(api_key)
                .json(&serde_json::json!({
                    "model": model_id.unwrap_or("gpt-4o-mini"),
                    "messages": [{"role": "user", "content": prompt}]
                }))
                .send()
                .await
                .map_err(|e| e.to_string())?;
            if !resp.status().is_success() {
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();
                return Err(format!("text error {}: {}", status, text));
            }
            let v: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
            Ok(serde_json::json!({
                "content": v["choices"][0]["message"]["content"].as_str().unwrap_or("")
            }))
        }
        "audio" => {
            let url = format!("{}/audio/speech", base);
            progress("正在请求语音生成...");
            let resp = client
                .post(&url)
                .bearer_auth(api_key)
                .json(&serde_json::json!({
                    "model": model_id.unwrap_or("tts-1"),
                    "input": prompt,
                    "voice": "alloy",
                    "response_format": "mp3"
                }))
                .send()
                .await
                .map_err(|e| e.to_string())?;
            if !resp.status().is_success() {
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();
                return Err(format!("audio error {}: {}", status, text));
            }
            let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
            let path = save_media_bytes("audio", "mp3", &bytes)?;
            Ok(serde_json::json!({"path": path}))
        }
        "video" | "videos" => {
            if base.contains("dashscope.aliyuncs.com") || std::env::var("DASH_SCOPE_MOCK").is_ok() {
                let api_root = base.trim_end_matches("/api/v1").trim_end_matches("/v1");
                let create_url = format!("{}/api/v1/services/aigc/video-generation/video-synthesis", api_root);
                progress("正在创建视频生成任务...");
                let resp = client
                    .post(&create_url)
                    .bearer_auth(api_key)
                    .header("X-DashScope-Async", "enable")
                    .json(&serde_json::json!({
                        "model": model_id.unwrap_or("wanx2.1-t2v-turbo"),
                        "input": {"prompt": prompt},
                        "parameters": {}
                    }))
                    .send()
                    .await
                    .map_err(|e| e.to_string())?;
                if !resp.status().is_success() {
                    let status = resp.status();
                    let text = resp.text().await.unwrap_or_default();
                    return Err(format!("video create error {}: {}", status, text));
                }
                let v: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
                let task_id = v["output"]["task_id"].as_str().unwrap_or("");
                if task_id.is_empty() {
                    return Err(format!("unexpected video create response: {}", v));
                }
                let poll_url = format!("{}/api/v1/tasks/{}", api_root, task_id);
                for _ in 0..120 {
                    if MEDIA_CANCEL.load(Ordering::Relaxed) {
                        return Err("cancelled".to_string());
                    }
                    progress("正在轮询视频任务...");
                    let pr = client
                        .get(&poll_url)
                        .bearer_auth(&api_key)
                        .send()
                        .await
                        .map_err(|e| e.to_string())?;
                    if !pr.status().is_success() {
                        let status = pr.status();
                        let text = pr.text().await.unwrap_or_default();
                        return Err(format!("video poll error {}: {}", status, text));
                    }
                    let pv: serde_json::Value = pr.json().await.map_err(|e| e.to_string())?;
                    let status = pv["output"]["task_status"].as_str().unwrap_or("");
                    match status {
                        "SUCCEEDED" => {
                            let url = pv["output"]["video_url"]
                                .as_str()
                                .or_else(|| pv["output"]["results"][0]["url"].as_str())
                                .unwrap_or("");
                            if url.is_empty() {
                                return Err(format!("video task succeeded without url: {}", pv));
                            }
                            return Ok(serde_json::json!({"url": url}));
                        }
                        "FAILED" | "CANCELED" => {
                            let msg = pv["output"]["message"].as_str().unwrap_or(status);
                            return Err(format!("video task {}: {}", status, msg));
                        }
                        _ => {}
                    }
                    let poll_secs = std::env::var("DASH_SCOPE_POLL_SECS")
                        .ok()
                        .and_then(|v| v.parse::<u64>().ok())
                        .unwrap_or(5);
                    tokio::time::sleep(std::time::Duration::from_secs(poll_secs)).await;
                }
                Err("video task timed out".to_string())
            } else {
                Err("video generation requires a DashScope base_url (https://dashscope.aliyuncs.com); configure it in provider settings".to_string())
            }
        }
        _ => Err(format!("unsupported media type: {}", model_type)),
    }
}

async fn call_media_provider(
    app: &AppHandle,
    prompt: &str,
    model_type: &str,
    model_id: Option<&str>,
    api_key: &str,
    base_url: &str,
    progress_event: &str,
) -> Result<serde_json::Value, String> {
    let mut emit = |msg: &str| {
        let _ = app.emit(progress_event, msg);
    };
    media_call_core(prompt, model_type, model_id, api_key, base_url, &mut emit).await
}

async fn run_generation_task(app: AppHandle, id: String, request: GenerationRequest) {
    if task_cancelled(&id) {
        return;
    }
    update_task_status(&id, "processing", Some("0"), None, None, None, false);
    let _ = app.emit("task-update", serde_json::json!({"id": id, "status": "processing", "progress": "0"}));
    let _provider = request.provider.as_deref().unwrap_or("openai");
    let (cfg_base, cfg_key) = db::media_provider_config();
    let base_url = request.base_url.clone().or(cfg_base).unwrap_or_else(|| "https://api.openai.com/v1".to_string());
    let api_key = request
        .api_key
        .clone()
        .filter(|v| !v.trim().is_empty())
        .or(cfg_key)
        .unwrap_or_default();
    let model_type = request.model_type.clone().unwrap_or_else(|| "text".to_string());
    let result = call_media_provider(
        &app,
        &request.prompt,
        &model_type,
        request.model_id.as_deref(),
        &api_key,
        &base_url,
        "media-progress",
    )
    .await;
    if task_cancelled(&id) {
        update_task_status(&id, "cancelled", None, None, None, None, true);
        let _ = app.emit("task-update", serde_json::json!({"id": id, "status": "cancelled"}));
        return;
    }
    match result {
        Ok(v) => {
            let result_url = v["url"].as_str().or_else(|| v["path"].as_str()).unwrap_or("").to_string();
            let result_data = v.to_string();
            update_task_status(&id, "completed", Some("100"), None, Some(&result_url), Some(&result_data), true);
            let _ = app.emit("task-update", serde_json::json!({
                "id": id, "status": "completed", "progress": "100",
                "result_url": result_url, "result_data": result_data
            }));
        }
        Err(e) => {
            update_task_status(&id, "failed", None, Some(&e), None, None, true);
            let _ = app.emit("task-update", serde_json::json!({"id": id, "status": "failed", "error": e}));
        }
    }
}

// ---- prompt composer ----

#[tauri::command]
pub fn get_prompt_categories_safe() -> Vec<serde_json::Value> {
    prompts::categories()
}

#[tauri::command]
pub fn get_thinking_methods_safe() -> Vec<serde_json::Value> {
    prompts::thinking_methods()
}

#[tauri::command]
pub fn get_output_formats_safe() -> Vec<serde_json::Value> {
    prompts::output_formats()
}

#[tauri::command]
pub fn compose_system_prompt(
    category_id: String,
    item_id: String,
    thinking_id: Option<String>,
    output_id: Option<String>,
) -> Result<String, String> {
    prompts::compose(
        &category_id,
        &item_id,
        thinking_id.as_deref(),
        output_id.as_deref(),
    )
}

#[tauri::command]
pub fn compose_prompt_preview(
    category_id: String,
    item_id: String,
    thinking_id: Option<String>,
    output_id: Option<String>,
) -> Result<String, String> {
    prompts::compose(
        &category_id,
        &item_id,
        thinking_id.as_deref(),
        output_id.as_deref(),
    )
}

// ---- conversations ----

#[derive(Serialize)]
pub struct ConversationRow {
    id: i64,
    title: String,
    created_at: String,
    updated_at: String,
    model: String,
}

#[tauri::command]
pub fn create_conversation(
    state: State<AppState>,
    title: String,
    model: String,
) -> Result<i64, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let now = db::now();
    conn.execute(
        "INSERT INTO conversations (title, created_at, updated_at, model) VALUES (?1, ?2, ?2, ?3)",
        params![title, now, model],
    )
    .map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

#[tauri::command]
pub fn list_conversations(state: State<AppState>) -> Result<Vec<serde_json::Value>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::list_conversations(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_conversation(state: State<AppState>, conversation_id: i64) -> Result<serde_json::Value, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    conn.query_row(
        "SELECT id, title, created_at, updated_at, model FROM conversations WHERE id = ?1",
        params![conversation_id],
        |r| {
            Ok(serde_json::json!({
                "id": r.get::<_, i64>(0)?,
                "title": r.get::<_, String>(1)?,
                "created_at": r.get::<_, String>(2)?,
                "updated_at": r.get::<_, String>(3)?,
                "model": r.get::<_, String>(4)?
            }))
        },
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_conversation_messages(state: State<AppState>, conversation_id: i64) -> Result<Vec<serde_json::Value>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::messages_for(&conn, conversation_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_message(
    state: State<AppState>,
    conversation_id: i64,
    role: String,
    content: String,
    model: String,
    media_type: Option<String>,
    media_url: Option<String>,
    media_metadata: Option<String>,
) -> Result<i64, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let now = db::now();
    conn.execute(
        "INSERT INTO messages (conversation_id, role, content, created_at, model, media_type, media_url, media_metadata)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            conversation_id,
            role,
            content,
            now,
            model,
            media_type,
            media_url,
            media_metadata
        ],
    )
    .map_err(|e| e.to_string())?;
    let id = conn.last_insert_rowid();
    conn.execute(
        "UPDATE conversations SET updated_at = ?1 WHERE id = ?2",
        params![now, conversation_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(id)
}

#[tauri::command]
pub fn update_conversation_title(
    state: State<AppState>,
    conversation_id: i64,
    title: String,
) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE conversations SET title = ?1, updated_at = ?2 WHERE id = ?3",
        params![title, db::now(), conversation_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn delete_conversation(state: State<AppState>, conversation_id: i64) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM conversations WHERE id = ?1", params![conversation_id])
        .map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM messages WHERE conversation_id = ?1", params![conversation_id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn delete_message(state: State<AppState>, message_id: i64) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE messages SET deleted = 1 WHERE id = ?1",
        params![message_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn delete_all_conversations(state: State<AppState>) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM conversations", []).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM messages", []).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn search_conversations(state: State<AppState>, query: String) -> Result<Vec<serde_json::Value>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, title, created_at, updated_at, model FROM conversations WHERE title LIKE ?1 ORDER BY updated_at DESC")
        .map_err(|e| e.to_string())?;
    let like = format!("%{}%", query);
    let rows = stmt
        .query_map(params![like], |r| {
            Ok(serde_json::json!({
                "id": r.get::<_, i64>(0)?,
                "title": r.get::<_, String>(1)?,
                "created_at": r.get::<_, String>(2)?,
                "updated_at": r.get::<_, String>(3)?,
                "model": r.get::<_, String>(4)?
            }))
        })
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| e.to_string())?);
    }
    Ok(out)
}

#[tauri::command]
pub fn export_conversation_markdown(state: State<AppState>, conversation_id: i64) -> Result<String, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let conv = get_conversation(state.clone(), conversation_id)?;
    let msgs = db::messages_for(&conn, conversation_id).map_err(|e| e.to_string())?;
    let title = conv["title"].as_str().unwrap_or("对话");
    let mut md = format!("# {}\n\n", title);
    for m in &msgs {
        let role = m["role"].as_str().unwrap_or("");
        let content = m["content"].as_str().unwrap_or("");
        md.push_str(&format!("## {}\n\n{}\n\n", role, content));
    }
    Ok(md)
}

// ---- media ----

#[tauri::command]
pub fn get_media_root_path() -> String {
    db::media_root().to_string_lossy().to_string()
}

#[tauri::command]
pub fn set_media_root_path(new_path: String) -> Result<(), String> {
    db::set_media_root(&new_path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_default_media_path() -> String {
    db::default_media_dir().to_string_lossy().to_string()
}

#[tauri::command]
pub fn get_media_files() -> Result<Vec<serde_json::Value>, String> {
    let dir = db::media_root();
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for e in entries.flatten() {
            let p = e.path();
            if let Ok(meta) = p.metadata() {
                out.push(serde_json::json!({
                    "name": p.file_name().unwrap_or_default().to_string_lossy(),
                    "relative_path": p.to_string_lossy(),
                    "size": meta.len(),
                    "is_dir": meta.is_dir()
                }));
            }
        }
    }
    Ok(out)
}

#[tauri::command]
pub fn delete_media_file(relative_path: String) -> Result<(), String> {
    let p = media_root_for(&relative_path)?;
    std::fs::remove_file(&p).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn rename_media_file(relative_path: String, new_name: String) -> Result<(), String> {
    let p = media_root_for(&relative_path)?;
    let parent = p.parent().unwrap_or(std::path::Path::new("."));
    let dest = parent.join(sanitize_name(&new_name));
    std::fs::rename(&p, &dest).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_local_image_path(relative_path: String) -> Result<String, String> {
    let p = media_root_for(&relative_path)?;
    Ok(p.to_string_lossy().to_string())
}

#[tauri::command]
pub fn open_path(path: String) -> Result<(), String> {
    let p = PathBuf::from(&path);
    let mut cmd = std::process::Command::new("explorer");
    if p.is_file() {
        cmd.arg("/select,").arg(&path);
    } else {
        cmd.arg(&path);
    }
    cmd.spawn().map(|_| ()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_audio_cover(audio_path: String, cover_base64: String) -> Result<String, String> {
    let cleaned = cover_base64
        .trim_start_matches("data:image/")
        .split(',')
        .last()
        .unwrap_or(&cover_base64);
    let bytes = BASE64.decode(cleaned).map_err(|e| e.to_string())?;
    let dir = db::media_root();
    std::fs::create_dir_all(&dir).ok();
    let stem = std::path::Path::new(&audio_path)
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy();
    let out = dir.join(format!("{}_cover.jpg", stem));
    std::fs::write(&out, bytes).map_err(|e| e.to_string())?;
    Ok(out.to_string_lossy().to_string())
}

#[tauri::command]
pub fn save_video_thumbnail(video_path: String, thumbnail_base64: String) -> Result<String, String> {
    let cleaned = thumbnail_base64
        .trim_start_matches("data:image/")
        .split(',')
        .last()
        .unwrap_or(&thumbnail_base64);
    let bytes = BASE64.decode(cleaned).map_err(|e| e.to_string())?;
    let dir = db::media_root();
    std::fs::create_dir_all(&dir).ok();
    let stem = std::path::Path::new(&video_path)
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy();
    let out = dir.join(format!("{}_thumb.jpg", stem));
    std::fs::write(&out, bytes).map_err(|e| e.to_string())?;
    Ok(out.to_string_lossy().to_string())
}

// ---- notes ----

#[tauri::command]
pub fn create_note(
    state: State<AppState>,
    title: String,
    content: String,
    source_type: String,
    source_id: Option<i64>,
    tags: Option<String>,
) -> Result<i64, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let now = db::now();
    conn.execute(
        "INSERT INTO notes (title, content, source_type, source_id, created_at, updated_at, tags)
         VALUES (?1, ?2, ?3, ?4, ?5, ?5, ?6)",
        params![title, content, source_type, source_id, now, tags],
    )
    .map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

#[tauri::command]
pub fn get_note(state: State<AppState>, note_id: i64) -> Result<serde_json::Value, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    conn.query_row(
        "SELECT id, title, content, source_type, source_id, created_at, updated_at, tags FROM notes WHERE id = ?1",
        params![note_id],
        |r| {
            Ok(serde_json::json!({
                "id": r.get::<_, i64>(0)?,
                "title": r.get::<_, String>(1)?,
                "content": r.get::<_, String>(2)?,
                "source_type": r.get::<_, String>(3)?,
                "source_id": r.get::<_, Option<i64>>(4)?,
                "created_at": r.get::<_, String>(5)?,
                "updated_at": r.get::<_, String>(6)?,
                "tags": r.get::<_, Option<String>>(7)?
            }))
        },
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_note_list(state: State<AppState>) -> Result<Vec<serde_json::Value>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, title, content, source_type, source_id, created_at, updated_at, tags FROM notes ORDER BY updated_at DESC")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok(serde_json::json!({
                "id": r.get::<_, i64>(0)?,
                "title": r.get::<_, String>(1)?,
                "content": r.get::<_, String>(2)?,
                "source_type": r.get::<_, String>(3)?,
                "source_id": r.get::<_, Option<i64>>(4)?,
                "created_at": r.get::<_, String>(5)?,
                "updated_at": r.get::<_, String>(6)?,
                "tags": r.get::<_, Option<String>>(7)?
            }))
        })
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| e.to_string())?);
    }
    Ok(out)
}

#[tauri::command]
pub fn update_note(
    state: State<AppState>,
    note_id: i64,
    title: String,
    content: String,
    tags: Option<String>,
) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE notes SET title = ?1, content = ?2, tags = ?3, updated_at = ?4 WHERE id = ?5",
        params![title, content, tags, db::now(), note_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn delete_note(state: State<AppState>, note_id: i64) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM notes WHERE id = ?1", params![note_id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn delete_all_notes(state: State<AppState>) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM notes", []).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn search_notes(state: State<AppState>, query: String) -> Result<Vec<serde_json::Value>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let like = format!("%{}%", query);
    let mut stmt = conn
        .prepare("SELECT id, title, content FROM notes WHERE title LIKE ?1 OR content LIKE ?1")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![like], |r| {
            Ok(serde_json::json!({
                "id": r.get::<_, i64>(0)?,
                "title": r.get::<_, String>(1)?,
                "content": r.get::<_, String>(2)?
            }))
        })
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| e.to_string())?);
    }
    Ok(out)
}

#[tauri::command]
pub fn export_note_markdown(state: State<AppState>, note_id: i64) -> Result<String, String> {
    let note = get_note(state, note_id)?;
    Ok(format!(
        "# {}\n\n{}",
        note["title"].as_str().unwrap_or(""),
        note["content"].as_str().unwrap_or("")
    ))
}

// ---- canvas ----

#[tauri::command]
pub fn create_canvas(
    state: State<AppState>,
    title: String,
    content: String,
    source_type: String,
    source_id: Option<i64>,
    tags: Option<String>,
) -> Result<i64, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let now = db::now();
    conn.execute(
        "INSERT INTO canvases (title, content, source_type, source_id, created_at, updated_at, tags)
         VALUES (?1, ?2, ?3, ?4, ?5, ?5, ?6)",
        params![title, content, source_type, source_id, now, tags],
    )
    .map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

#[tauri::command]
pub fn get_canvas(state: State<AppState>, canvas_id: i64) -> Result<serde_json::Value, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    conn.query_row(
        "SELECT id, title, content, source_type, source_id, created_at, updated_at, tags FROM canvases WHERE id = ?1",
        params![canvas_id],
        |r| {
            Ok(serde_json::json!({
                "id": r.get::<_, i64>(0)?,
                "title": r.get::<_, String>(1)?,
                "content": r.get::<_, String>(2)?,
                "source_type": r.get::<_, String>(3)?,
                "source_id": r.get::<_, Option<i64>>(4)?,
                "created_at": r.get::<_, String>(5)?,
                "updated_at": r.get::<_, String>(6)?,
                "tags": r.get::<_, Option<String>>(7)?
            }))
        },
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_canvas_list(state: State<AppState>) -> Result<Vec<serde_json::Value>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, title, content, source_type, source_id, created_at, updated_at, tags FROM canvases ORDER BY updated_at DESC")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok(serde_json::json!({
                "id": r.get::<_, i64>(0)?,
                "title": r.get::<_, String>(1)?,
                "content": r.get::<_, String>(2)?,
                "source_type": r.get::<_, String>(3)?,
                "source_id": r.get::<_, Option<i64>>(4)?,
                "created_at": r.get::<_, String>(5)?,
                "updated_at": r.get::<_, String>(6)?,
                "tags": r.get::<_, Option<String>>(7)?
            }))
        })
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| e.to_string())?);
    }
    Ok(out)
}

#[tauri::command]
pub fn update_canvas(
    state: State<AppState>,
    canvas_id: i64,
    title: String,
    content: String,
    tags: Option<String>,
) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE canvases SET title = ?1, content = ?2, tags = ?3, updated_at = ?4 WHERE id = ?5",
        params![title, content, tags, db::now(), canvas_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn delete_canvas(state: State<AppState>, canvas_id: i64) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM canvases WHERE id = ?1", params![canvas_id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn search_canvases(state: State<AppState>, query: String) -> Result<Vec<serde_json::Value>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let like = format!("%{}%", query);
    let mut stmt = conn
        .prepare("SELECT id, title, content FROM canvases WHERE title LIKE ?1 OR content LIKE ?1")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![like], |r| {
            Ok(serde_json::json!({
                "id": r.get::<_, i64>(0)?,
                "title": r.get::<_, String>(1)?,
                "content": r.get::<_, String>(2)?
            }))
        })
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| e.to_string())?);
    }
    Ok(out)
}

// ---- misc ----

#[tauri::command]
pub fn db_health_check(state: State<AppState>) -> Result<bool, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    conn.query_row("SELECT 1", [], |_| Ok(())).map_err(|e| e.to_string())?;
    Ok(true)
}

#[tauri::command]
pub fn debug_log(message: String) {
    eprintln!("[debug] {}", message);
}

// ---- chat streaming (OpenAI-compatible) ----

#[derive(serde::Deserialize)]
pub struct ChatRequest {
    pub messages: Vec<ChatMessage>,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub system_prompt: Option<String>,
    pub max_tokens: Option<u32>,
    pub reasoning_effort: Option<String>,
}

#[derive(serde::Deserialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[tauri::command]
pub async fn stream_chat_raw(
    app: AppHandle,
    request: ChatRequest,
) -> Result<(), String> {
    CHAT_CANCEL.store(false, Ordering::Relaxed);
    // Provider fallback comes from settings.json (chat_api_key / chat_base_url);
    // the source tree must never embed a secret.
    let (fallback_key, fallback_base) = db::chat_fallback();
    let api_key = request
        .api_key
        .clone()
        .filter(|value| !value.trim().is_empty())
        .or(fallback_key)
        .unwrap_or_default();
    let base_url = request
        .base_url
        .clone()
        .or(fallback_base)
        .unwrap_or_else(|| "https://api.openai.com/v1".into());
    let model = request.model.clone().unwrap_or_else(|| "gpt-4o-mini".into());
    let temperature = request.temperature.unwrap_or(0.7);
    // Use a generous completion budget and the provider's strongest reasoning mode
    // unless the model configuration explicitly overrides them.
    let max_tokens = request.max_tokens.unwrap_or(32768);
    let reasoning_effort = request
        .reasoning_effort
        .clone()
        .unwrap_or_else(|| "high".into());

    let mut messages: Vec<serde_json::Value> = Vec::new();
    if let Some(sp) = &request.system_prompt {
        if !sp.is_empty() {
            messages.push(serde_json::json!({"role": "system", "content": sp}));
        }
    }
    for m in &request.messages {
        messages.push(serde_json::json!({"role": m.role, "content": m.content}));
    }

    let client = reqwest::Client::new();
    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));
    let result = async {
        let resp = client
            .post(&url)
            .bearer_auth(api_key)
            .json(&serde_json::json!({
                "model": model,
                "messages": messages,
                "temperature": temperature,
                "max_tokens": max_tokens,
                "max_completion_tokens": max_tokens,
                "reasoning_effort": reasoning_effort,
                "stream": true
            }))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("chat error {}: {}", status, text));
        }

        let mut stream = resp.bytes_stream();
        use futures_util::StreamExt;
        let mut buf = String::new();
        while let Some(chunk) = stream.next().await {
            if CHAT_CANCEL.load(Ordering::Relaxed) {
                return Err("cancelled".to_string());
            }
            let chunk = chunk.map_err(|e| e.to_string())?;
            let s = String::from_utf8_lossy(&chunk);
            buf.push_str(&s);
            while let Some(pos) = buf.find('\n') {
                let line = buf[..pos].to_string();
                buf = buf[pos + 1..].to_string();
                let line = line.trim();
                if line.is_empty() || !line.starts_with("data:") {
                    continue;
                }
                let data = line[5..].trim();
                if data == "[DONE]" {
                    return Ok(());
                }
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(data) {
                    let has_delta = v["choices"][0]["delta"]["content"].as_str().is_some()
                        || v["choices"][0]["delta"]["reasoning_content"].as_str().is_some();
                    if has_delta {
                        let _ = app.emit("chat-raw-chunk", format!("data: {}", data));
                    }
                }
            }
        }
        Ok(())
    }
    .await;

    match result {
        Ok(()) => {
            let _ = app.emit("chat-raw-end", "");
            Ok(())
        }
        Err(e) => {
            if e == "cancelled" {
                let _ = app.emit("chat-raw-end", "");
                Ok(())
            } else {
                let _ = app.emit("chat-raw-error", e.clone());
                Ok(())
            }
        }
    }
}

#[tauri::command]
pub fn cancel_chat() -> Result<(), String> {
    CHAT_CANCEL.store(true, Ordering::Relaxed);
    Ok(())
}

// ---- generation tasks (stub persistence) ----

#[derive(serde::Deserialize, serde::Serialize)]
pub struct GenerationRequest {
    pub prompt: String,
    #[serde(alias = "modelId")]
    pub model_id: Option<String>,
    #[serde(alias = "modelProvider")]
    pub model_provider: Option<String>,
    #[serde(alias = "modelType")]
    pub model_type: Option<String>,
    pub provider: Option<String>,
    #[serde(alias = "baseUrl")]
    pub base_url: Option<String>,
    #[serde(alias = "apiKey")]
    pub api_key: Option<String>,
    #[serde(alias = "imageSize")]
    pub image_size: Option<String>,
    #[serde(alias = "aspectRatio")]
    pub aspect_ratio: Option<String>,
    #[serde(alias = "numInferenceSteps")]
    pub num_inference_steps: Option<i64>,
    #[serde(alias = "numImages")]
    pub num_images: Option<i64>,
}

#[tauri::command]
pub fn submit_generation_task(
    app: AppHandle,
    state: State<AppState>,
    request: GenerationRequest,
) -> Result<String, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let id = uuidish();
    let now_ms = chrono::Utc::now().timestamp_millis();
    conn.execute(
        "INSERT INTO generation_tasks (id, status, timestamp, model_id, model_provider, model_type, provider, request_data)
         VALUES (?1, 'pending', ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            id,
            now_ms,
            request.model_id,
            request.model_provider,
            request.model_type,
            request.provider,
            serde_json::to_string(&request).unwrap_or_default()
        ],
    )
    .map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn(run_generation_task(app, id.clone(), request));
    Ok(id)
}

#[tauri::command]
pub fn cancel_generation_task(state: State<AppState>, task_id: String) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE generation_tasks SET status = 'cancelled' WHERE id = ?1",
        params![task_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_pending_tasks(state: State<AppState>) -> Result<Vec<serde_json::Value>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, status, progress, error, timestamp, completed_at, model_id, model_provider, model_type, provider, request_data, result_url, result_data FROM generation_tasks ORDER BY timestamp DESC")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok(serde_json::json!({
                "id": r.get::<_, String>(0)?,
                "status": r.get::<_, String>(1)?,
                "progress": r.get::<_, Option<String>>(2)?,
                "error": r.get::<_, Option<String>>(3)?,
                "timestamp": r.get::<_, i64>(4)?,
                "completed_at": r.get::<_, Option<i64>>(5)?,
                "model_id": r.get::<_, Option<String>>(6)?,
                "model_provider": r.get::<_, Option<String>>(7)?,
                "model_type": r.get::<_, Option<String>>(8)?,
                "provider": r.get::<_, Option<String>>(9)?,
                "request_data": r.get::<_, Option<String>>(10)?,
                "result_url": r.get::<_, Option<String>>(11)?,
                "result_data": r.get::<_, Option<String>>(12)?
            }))
        })
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| e.to_string())?);
    }
    Ok(out)
}

#[tauri::command]
pub fn check_timeout_tasks(state: State<AppState>) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let cutoff = chrono::Utc::now().timestamp_millis() - 30 * 60 * 1000;
    conn.execute(
        "UPDATE generation_tasks SET status = 'failed', error = 'timeout', completed_at = ?1
         WHERE status IN ('pending','processing') AND timestamp < ?1",
        params![cutoff],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn generate_media(
    app: AppHandle,
    prompt: String,
    model_type: Option<String>,
    model_id: Option<String>,
    api_key: Option<String>,
) -> Result<String, String> {
    MEDIA_CANCEL.store(false, Ordering::Relaxed);
    let model_type = model_type.unwrap_or_else(|| "text".to_string());
    let (cfg_base, cfg_key) = db::media_provider_config();
    let api_key = api_key
        .filter(|v| !v.trim().is_empty())
        .or(cfg_key)
        .unwrap_or_default();
    let base_url = cfg_base.unwrap_or_else(|| "https://api.openai.com/v1".to_string());
    match call_media_provider(
        &app,
        &prompt,
        &model_type,
        model_id.as_deref(),
        &api_key,
        &base_url,
        "media-progress",
    )
    .await
    {
        Ok(v) => {
            if let Some(u) = v["url"].as_str() {
                Ok(u.to_string())
            } else if let Some(p) = v["path"].as_str() {
                Ok(p.to_string())
            } else if let Some(c) = v["content"].as_str() {
                Ok(c.to_string())
            } else {
                Ok(v.to_string())
            }
        }
        Err(e) => Err(e),
    }
}

#[tauri::command]
pub fn cancel_media_generation() -> Result<(), String> {
    MEDIA_CANCEL.store(true, Ordering::Relaxed);
    Ok(())
}

fn uuidish() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("task-{:x}", nanos)
}

// ---- external / update ----

#[tauri::command]
pub fn read_external_file(file_path: String) -> Result<String, String> {
    std::fs::read_to_string(&file_path).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn download_and_save_world_model(
    app: AppHandle,
    world_id: String,
    world_name: String,
    spz_url: String,
) -> Result<String, String> {
    if spz_url.trim().is_empty() {
        return Err("SPZ file URL is empty".to_string());
    }
    let _ = app.emit("media-progress", "正在下载世界模型...");
    let resp = reqwest::get(&spz_url).await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("world model download {}: {}", status, text));
    }
    let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
    let dir = db::media_root().join("world_models");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let safe_name = if world_name.trim().is_empty() { sanitize_name(&world_id) } else { sanitize_name(&world_name) };
    let path = dir.join(format!("{}.spz", safe_name));
    std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;
    let _ = app.emit("media-progress", "世界模型已保存");
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn download_update_with_auth(app: AppHandle, url: String, asset_id: String) -> Result<String, String> {
    if url.trim().is_empty() {
        return Err("update download URL is empty".to_string());
    }
    let dir = db::app_data_dir().join("updates");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let url_ext = std::path::Path::new(&url)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .filter(|e| e == "exe" || e == "msi");
    let mut filename = if asset_id.trim().is_empty() {
        "update.bin".to_string()
    } else {
        sanitize_name(&asset_id)
    };
    if !filename.ends_with(".exe") && !filename.ends_with(".msi") {
        if let Some(ext) = url_ext {
            filename.push('.');
            filename.push_str(&ext);
        }
    }
    let dest = dir.join(filename);
    let client = reqwest::Client::new();
    if let Some(local) = url.strip_prefix("file://") {
        let local = local.trim_start_matches('/');
        let src = std::path::PathBuf::from(local);
        if !src.exists() {
            return Err(format!("update file not found: {}", src.to_string_lossy()));
        }
        std::fs::copy(&src, &dest).map_err(|e| e.to_string())?;
        let _ = app.emit("download-progress", 100);
    } else {
        let mut resp = client.get(&url).send().await.map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("update download {}: {}", status, text));
        }
        let total = resp.content_length().unwrap_or(0);
        let mut downloaded: u64 = 0;
        let mut out = std::fs::File::create(&dest).map_err(|e| e.to_string())?;
        while let Some(chunk) = resp.chunk().await.map_err(|e| e.to_string())? {
            downloaded += chunk.len() as u64;
            use std::io::Write;
            out.write_all(&chunk).map_err(|e| e.to_string())?;
            let pct = if total > 0 { (downloaded as f64 / total as f64 * 100.0).round() as u32 } else { 0 };
            let _ = app.emit("download-progress", pct);
        }
    }
    if db::auto_install_updates() {
        let ext = dest
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        let dest_str = dest.to_string_lossy().to_string();
        match ext.as_str() {
            "msi" => {
                let _ = std::process::Command::new("msiexec")
                    .args(["/i", &dest_str, "/qn"])
                    .spawn();
            }
            "exe" => {
                let _ = std::process::Command::new(&dest).arg("/S").spawn();
            }
            _ => {}
        }
    }
    Ok(dest.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn check_update_with_token(_token: Option<String>) -> Result<serde_json::Value, String> {
    let manifest_path = db::app_data_dir().join("update-manifest.json");
    let manifest_url = db::update_manifest_url();
    let text = if let Some(url) = manifest_url {
        let client = reqwest::Client::new();
        let resp = client.get(&url).send().await.map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            // Remote manifest unreachable: fall back to local file, then "no update".
            if manifest_path.exists() {
                std::fs::read_to_string(&manifest_path).map_err(|e| e.to_string())?
            } else {
                return Ok(serde_json::Value::Null);
            }
        } else {
            resp.text().await.map_err(|e| e.to_string())?
        }
    } else if manifest_path.exists() {
        std::fs::read_to_string(&manifest_path).map_err(|e| e.to_string())?
    } else {
        return Ok(serde_json::Value::Null);
    };
    let v: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let latest = v["version"].as_str().unwrap_or("");
    let current = env!("CARGO_PKG_VERSION");
    if latest.is_empty() || latest == current {
        return Ok(serde_json::Value::Null);
    }
    Ok(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_keeps_safe_chars() {
        assert_eq!(sanitize_name("test_world-01.spz"), "test_world-01.spz");
        assert_eq!(sanitize_name("a/b\\c:d"), "a_b_c_d");
    }

    #[test]
    fn media_root_accepts_relative_and_absolute_inside_root() {
        let root = std::env::temp_dir().join("pf-resolve-test");
        let rel = resolve_media_path(&root, "x.png").expect("relative ok");
        assert!(rel.starts_with(&root));

        let abs = root.join("sub").join("y.png");
        let ok = resolve_media_path(&root, abs.to_str().unwrap()).expect("absolute inside ok");
        assert_eq!(ok, abs);

        let outside = resolve_media_path(&root, "C:\\Windows\\win.ini").expect_err("outside rejected");
        assert!(outside.contains("escapes"));
    }

    fn start_mock(handler: impl Fn(&str, &str) -> (u16, Vec<u8>) + Send + 'static) -> String {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock");
        let addr = listener.local_addr().expect("addr");
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let mut buf = Vec::new();
                let mut tmp = [0u8; 1024];
                loop {
                    match stream.read(&mut tmp) {
                        Ok(0) => break,
                        Ok(n) => {
                            buf.extend_from_slice(&tmp[..n]);
                            if buf.windows(4).any(|w| w == b"\r\n\r\n") {
                                break;
                            }
                            if buf.len() > 65536 {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
                let text = String::from_utf8_lossy(&buf);
                let request_line = text.lines().next().unwrap_or("");
                let mut parts = request_line.split_whitespace();
                let method = parts.next().unwrap_or("GET").to_string();
                let path = parts.next().unwrap_or("/").to_string();
                let (status, body) = handler(&method, &path);
                let head = format!(
                    "HTTP/1.1 {}\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    status, body.len()
                );
                let _ = stream.write_all(head.as_bytes());
                let _ = stream.write_all(&body);
                let _ = stream.flush();
                let _ = stream.shutdown(std::net::Shutdown::Write);
            }
        });
        format!("http://{}/v1", addr)
    }

    #[test]
    fn media_text_returns_content() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let base = start_mock(|method, path| {
                assert_eq!(method, "POST");
                assert_eq!(path, "/v1/chat/completions");
                (200, br#"{"choices":[{"message":{"content":"hello from mock"}}]}"#.to_vec())
            });
            let mut progress = Vec::new();
            let r = media_call_core(
                "ping", "text", Some("mock-model"), "key", &base,
                &mut |msg| progress.push(msg.to_string()),
            ).await.expect("text ok");
            assert_eq!(r["content"], "hello from mock");
            assert!(progress.iter().any(|p| p.contains("文本")));
        });
    }

    #[test]
    fn media_image_returns_url() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let base = start_mock(|method, path| {
                assert_eq!(method, "POST");
                assert_eq!(path, "/v1/images/generations");
                (200, br#"{"data":[{"url":"https://example.com/a.png"}]}"#.to_vec())
            });
            let mut progress = Vec::new();
            let r = media_call_core(
                "a cat", "image", Some("mock-image"), "key", &base,
                &mut |msg| progress.push(msg.to_string()),
            ).await.expect("image ok");
            assert_eq!(r["url"], "https://example.com/a.png");
            assert!(progress.iter().any(|p| p.contains("图像")));
        });
    }

    #[test]
    fn media_audio_saves_file() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let base = start_mock(|method, path| {
                assert_eq!(method, "POST");
                assert_eq!(path, "/v1/audio/speech");
                (200, b"MP3FAKEBYTES".to_vec())
            });
            let mut progress = Vec::new();
            let r = media_call_core(
                "say hi", "audio", Some("tts-1"), "key", &base,
                &mut |msg| progress.push(msg.to_string()),
            ).await.expect("audio ok");
            let path = r["path"].as_str().expect("audio path");
            assert!(std::path::Path::new(path).exists());
            let _ = std::fs::remove_file(path);
            assert!(progress.iter().any(|p| p.contains("语音")));
        });
    }

    #[test]
    fn media_video_polls_dashscope_task() {
        std::env::set_var("DASH_SCOPE_MOCK", "1");
        std::env::set_var("DASH_SCOPE_POLL_SECS", "0");
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let get_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let count_ref = std::sync::Arc::clone(&get_count);
            let base = start_mock(move |method, path| {
                if method == "POST" {
                    assert!(path.ends_with("/video-synthesis"));
                    return (200, br#"{"output":{"task_id":"t1"}}"#.to_vec());
                }
                assert_eq!(method, "GET");
                assert!(path.ends_with("/tasks/t1"));
                let n = count_ref.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                if n == 0 {
                    (200, br#"{"output":{"task_status":"RUNNING"}}"#.to_vec())
                } else {
                    (200, br#"{"output":{"task_status":"SUCCEEDED","video_url":"https://example.com/v.mp4"}}"#.to_vec())
                }
            });
            let mut progress = Vec::new();
            let r = media_call_core(
                "a storm", "video", Some("wanx2.1-t2v-turbo"), "key", &base,
                &mut |msg| progress.push(msg.to_string()),
            ).await.expect("video ok");
            assert_eq!(r["url"], "https://example.com/v.mp4");
            assert!(progress.iter().any(|p| p.contains("轮询")));
        });
    }
}
