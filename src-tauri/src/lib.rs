pub mod commands;
pub mod db;
pub mod prompts;

use rusqlite::Connection;
use std::sync::Mutex;

pub struct AppState {
    pub db: Mutex<Connection>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let conn = db::open_db().expect("failed to open database");
    db::init(&conn).expect("failed to init schema");

    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_log::Builder::new().build())
        .manage(AppState { db: Mutex::new(conn) })
        .invoke_handler(tauri::generate_handler![
            commands::get_prompt_categories_safe,
            commands::get_thinking_methods_safe,
            commands::get_output_formats_safe,
            commands::compose_system_prompt,
            commands::compose_prompt_preview,
            commands::create_conversation,
            commands::list_conversations,
            commands::get_conversation,
            commands::get_conversation_messages,
            commands::save_message,
            commands::update_conversation_title,
            commands::delete_conversation,
            commands::delete_all_conversations,
            commands::delete_message,
            commands::search_conversations,
            commands::export_conversation_markdown,
            commands::get_media_root_path,
            commands::set_media_root_path,
            commands::get_media_files,
            commands::get_default_media_path,
            commands::delete_media_file,
            commands::rename_media_file,
            commands::get_local_image_path,
            commands::open_path,
            commands::save_audio_cover,
            commands::save_video_thumbnail,
            commands::create_note,
            commands::get_note,
            commands::get_note_list,
            commands::update_note,
            commands::delete_note,
            commands::delete_all_notes,
            commands::search_notes,
            commands::export_note_markdown,
            commands::create_canvas,
            commands::get_canvas,
            commands::get_canvas_list,
            commands::update_canvas,
            commands::delete_canvas,
            commands::search_canvases,
            commands::stream_chat_raw,
            commands::cancel_chat,
            commands::submit_generation_task,
            commands::cancel_generation_task,
            commands::get_pending_tasks,
            commands::check_timeout_tasks,
            commands::generate_media,
            commands::cancel_media_generation,
            commands::read_external_file,
            commands::download_and_save_world_model,
            commands::download_update_with_auth,
            commands::check_update_with_token,
            commands::db_health_check,
            commands::debug_log,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
