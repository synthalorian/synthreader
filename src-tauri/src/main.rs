#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::Manager;

fn main() {
    tracing_subscriber::fmt::init();

    tauri::Builder::default()
        .setup(|app| {
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let app_dir = app_handle.path().app_data_dir().unwrap();
                let db_path = app_dir.join("library.db");
                std::fs::create_dir_all(&app_dir).ok();
                
                match synthreader_core::db::LibraryDb::open(&db_path).await {
                    Ok(_) => tracing::info!("Library database initialized"),
                    Err(e) => tracing::error!("Failed to initialize database: {}", e),
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! Welcome to Synthreader.", name)
}
