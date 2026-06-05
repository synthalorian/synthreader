#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use tauri::Manager;

fn main() {
    tracing_subscriber::fmt::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
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
        .invoke_handler(tauri::generate_handler![
            greet,
            add_books,
            get_books,
            get_book_cover
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! Welcome to Synthreader.", name)
}

#[derive(serde::Serialize)]
struct BookDto {
    id: i64,
    title: String,
    authors: Vec<String>,
    progress: f64,
    has_cover: bool,
    cover_path: Option<String>,
}

#[tauri::command]
async fn add_books(
    app: tauri::AppHandle,
    paths: Vec<String>,
) -> Result<Vec<BookDto>, String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let db_path = app_dir.join("library.db");
    let covers_dir = app_dir.join("covers");
    std::fs::create_dir_all(&covers_dir).map_err(|e| e.to_string())?;

    let db = synthreader_core::db::LibraryDb::open(&db_path)
        .await
        .map_err(|e| e.to_string())?;

    let mut added = Vec::new();

    for path_str in paths {
        let path = std::path::Path::new(&path_str);
        let metadata = match synthreader_core::formats::parse_book(path) {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!("Failed to parse {}: {}", path_str, e);
                continue;
            }
        };

        let cover_path = if let Some(cover_data) = &metadata.cover_data {
            let cover_filename = format!("{}.png", uuid::Uuid::new_v4());
            let cover_file = covers_dir.join(&cover_filename);
            std::fs::write(&cover_file, cover_data).ok();
            Some(cover_file)
        } else {
            None
        };

        let book_id = db
            .add_book(&metadata, &cover_path)
            .await
            .map_err(|e| e.to_string())?;

        let format = synthreader_core::formats::detect_format(path)
            .map(|f| f.as_str().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        db.add_book_file(book_id, &format, path, None)
            .await
            .map_err(|e| e.to_string())?;

        let authors = db.get_book_authors(book_id).await.unwrap_or_default();

        added.push(BookDto {
            id: book_id,
            title: metadata.title,
            authors,
            progress: 0.0,
            has_cover: cover_path.is_some(),
            cover_path: cover_path.map(|p| p.to_string_lossy().to_string()),
        });
    }

    Ok(added)
}

#[tauri::command]
async fn get_books(app: tauri::AppHandle) -> Result<Vec<BookDto>, String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let db_path = app_dir.join("library.db");

    let db = synthreader_core::db::LibraryDb::open(&db_path)
        .await
        .map_err(|e| e.to_string())?;

    let rows = db.get_books().await.map_err(|e| e.to_string())?;
    let mut books = Vec::new();

    for row in rows {
        let authors = db.get_book_authors(row.id).await.unwrap_or_default();
        books.push(BookDto {
            id: row.id,
            title: row.title,
            authors,
            progress: 0.0,
            has_cover: row.cover_path.is_some(),
            cover_path: row.cover_path,
        });
    }

    Ok(books)
}

#[tauri::command]
async fn get_book_cover(
    _app: tauri::AppHandle,
    cover_path: String,
) -> Result<Vec<u8>, String> {
    let path = PathBuf::from(cover_path);
    std::fs::read(&path).map_err(|e| e.to_string())
}
