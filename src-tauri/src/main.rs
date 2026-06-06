#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tauri::Manager;

#[derive(serde::Serialize)]
struct ChapterDto {
    href: String,
    title: String,
}

#[derive(serde::Serialize)]
struct FontDto {
    name: String,
    path: String,
    family: String,
}

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
                    Ok(db) => {
                        tracing::info!("Library database initialized");
                        let db = Arc::new(db);

                        // Start background feed refresh every 15 minutes
                        let refresh_service = Arc::new(synthreader_core::refresh::FeedRefreshService::new(db));
                        refresh_service.start_background(Duration::from_secs(900));
                        tracing::info!("Background feed refresh started (15 min interval)");
                    }
                    Err(e) => tracing::error!("Failed to initialize database: {}", e),
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            add_books,
            get_books,
            get_book_cover,
            add_feed,
            get_feeds,
            get_articles,
            get_all_articles,
            get_article,
            mark_article_read,
            star_article,
            search_articles,
            extract_article_content,
            refresh_feeds,
            get_system_fonts,
            get_book_chapters,
            get_chapter_content,
            get_font_preference,
            set_font_preference
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! Welcome to Synthreader.", name)
}

// -- Feed commands --

#[tauri::command]
async fn add_feed(
    app: tauri::AppHandle,
    title: String,
    url: String,
    site_url: Option<String>,
    description: Option<String>,
    folder_id: Option<i64>,
) -> Result<i64, String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let db_path = app_dir.join("library.db");

    let db = synthreader_core::db::LibraryDb::open(&db_path)
        .await
        .map_err(|e| e.to_string())?;

    let id = db
        .add_feed(&title, &url, site_url.as_deref(), description.as_deref(), folder_id)
        .await
        .map_err(|e| e.to_string())?;

    Ok(id)
}

#[tauri::command]
async fn get_feeds(app: tauri::AppHandle) -> Result<Vec<synthreader_core::Feed>, String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let db_path = app_dir.join("library.db");

    let db = synthreader_core::db::LibraryDb::open(&db_path)
        .await
        .map_err(|e| e.to_string())?;

    db.get_feeds().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_articles(
    app: tauri::AppHandle,
    feed_id: i64,
) -> Result<Vec<synthreader_core::Article>, String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let db_path = app_dir.join("library.db");

    let db = synthreader_core::db::LibraryDb::open(&db_path)
        .await
        .map_err(|e| e.to_string())?;

    db.get_articles_for_feed(feed_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_all_articles(
    app: tauri::AppHandle,
    limit: Option<i64>,
) -> Result<Vec<synthreader_core::Article>, String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let db_path = app_dir.join("library.db");

    let db = synthreader_core::db::LibraryDb::open(&db_path)
        .await
        .map_err(|e| e.to_string())?;

    db.get_all_articles(limit)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_article(
    app: tauri::AppHandle,
    article_id: i64,
) -> Result<Option<synthreader_core::Article>, String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let db_path = app_dir.join("library.db");

    let db = synthreader_core::db::LibraryDb::open(&db_path)
        .await
        .map_err(|e| e.to_string())?;

    db.get_article(article_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn mark_article_read(
    app: tauri::AppHandle,
    article_id: i64,
    read: bool,
) -> Result<(), String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let db_path = app_dir.join("library.db");

    let db = synthreader_core::db::LibraryDb::open(&db_path)
        .await
        .map_err(|e| e.to_string())?;

    db.mark_article_read(article_id, read)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn star_article(
    app: tauri::AppHandle,
    article_id: i64,
    starred: bool,
) -> Result<(), String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let db_path = app_dir.join("library.db");

    let db = synthreader_core::db::LibraryDb::open(&db_path)
        .await
        .map_err(|e| e.to_string())?;

    db.star_article(article_id, starred)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn search_articles(
    app: tauri::AppHandle,
    query: String,
    limit: Option<i64>,
) -> Result<Vec<synthreader_core::Article>, String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let db_path = app_dir.join("library.db");

    let db = synthreader_core::db::LibraryDb::open(&db_path)
        .await
        .map_err(|e| e.to_string())?;

    db.search_articles(&query, limit)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn extract_article_content(
    app: tauri::AppHandle,
    article_id: i64,
    url: String,
) -> Result<synthreader_core::ExtractedContent, String> {
    let extracted = synthreader_core::fetch_and_extract(&url)
        .await
        .map_err(|e| e.to_string())?;

    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let db_path = app_dir.join("library.db");

    let db = synthreader_core::db::LibraryDb::open(&db_path)
        .await
        .map_err(|e| e.to_string())?;

    db.update_article_content(article_id, Some(&extracted.content))
        .await
        .map_err(|e| e.to_string())?;

    Ok(extracted)
}

#[tauri::command]
async fn refresh_feeds(app: tauri::AppHandle) -> Result<synthreader_core::RefreshStats, String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let db_path = app_dir.join("library.db");

    let db = synthreader_core::db::LibraryDb::open(&db_path)
        .await
        .map_err(|e| e.to_string())?;

    let db = Arc::new(db);
    synthreader_core::run_refresh(db)
        .await
        .map_err(|e| e.to_string())
}

// -- Book commands (legacy) --

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

// -- System font commands --

#[tauri::command]
fn get_system_fonts() -> Result<Vec<FontDto>, String> {
    let fonts = synthreader_core::fonts::discover_system_fonts();
    Ok(fonts
        .into_iter()
        .map(|f| FontDto {
            name: f.name,
            path: f.path,
            family: f.family,
        })
        .collect())
}

#[tauri::command]
async fn get_font_preference(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let db_path = app_dir.join("library.db");

    let db = synthreader_core::db::LibraryDb::open(&db_path)
        .await
        .map_err(|e| e.to_string())?;

    db.get_setting("reader_font_path")
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn set_font_preference(
    app: tauri::AppHandle,
    font_path: String,
) -> Result<(), String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let db_path = app_dir.join("library.db");

    let db = synthreader_core::db::LibraryDb::open(&db_path)
        .await
        .map_err(|e| e.to_string())?;

    db.set_setting("reader_font_path", &font_path)
        .await
        .map_err(|e| e.to_string())
}

// -- Book reader commands --

#[tauri::command]
async fn get_book_chapters(
    app: tauri::AppHandle,
    book_id: i64,
) -> Result<Vec<ChapterDto>, String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let db_path = app_dir.join("library.db");

    let db = synthreader_core::db::LibraryDb::open(&db_path)
        .await
        .map_err(|e| e.to_string())?;

    let file_path = db
        .get_book_file_path(book_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Book file not found")?;

    let path = PathBuf::from(&file_path);
    let epub = synthreader_core::formats::epub::parse_epub(&path)
        .map_err(|e| e.to_string())?;

    let chapters = epub
        .toc
        .into_iter()
        .map(|entry| ChapterDto {
            href: entry.href,
            title: entry.label,
        })
        .collect();

    Ok(chapters)
}

#[tauri::command]
async fn get_chapter_content(
    app: tauri::AppHandle,
    book_id: i64,
    href: String,
) -> Result<String, String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let db_path = app_dir.join("library.db");

    let db = synthreader_core::db::LibraryDb::open(&db_path)
        .await
        .map_err(|e| e.to_string())?;

    let file_path = db
        .get_book_file_path(book_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Book file not found")?;

    let path = PathBuf::from(&file_path);
    synthreader_core::formats::epub::get_chapter_content(&path, &href)
        .map_err(|e| e.to_string())
}
