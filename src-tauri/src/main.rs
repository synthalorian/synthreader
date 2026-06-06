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
            set_font_preference,
            add_folder,
            get_folders,
            update_folder,
            delete_folder,
            get_feeds_by_folder,
            move_feed_to_folder,
            add_tag,
            get_tags,
            tag_article,
            untag_article,
            get_article_tags,
            get_articles_by_tag,
            delete_tag,
            get_unread_articles,
            get_starred_articles,
            get_unread_count,
            import_opml,
            export_opml,
            discover_feeds,
            delete_feed,
            update_feed,
            get_feed_stats,
            get_all_feed_stats,
            get_library_stats,
            delete_article,
            delete_old_articles,
            mark_all_articles_read,
            find_duplicate_articles
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! Welcome to Synthreader.", name)
}

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

#[tauri::command]
async fn add_folder(
    app: tauri::AppHandle,
    name: String,
    parent_id: Option<i64>,
    sort_order: i32,
) -> Result<i64, String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let db_path = app_dir.join("library.db");

    let db = synthreader_core::db::LibraryDb::open(&db_path)
        .await
        .map_err(|e| e.to_string())?;

    db.add_folder(&name, parent_id, sort_order)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_folders(app: tauri::AppHandle) -> Result<Vec<synthreader_core::Folder>, String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let db_path = app_dir.join("library.db");

    let db = synthreader_core::db::LibraryDb::open(&db_path)
        .await
        .map_err(|e| e.to_string())?;

    db.get_folders().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn update_folder(
    app: tauri::AppHandle,
    folder_id: i64,
    name: String,
    parent_id: Option<i64>,
    sort_order: i32,
) -> Result<(), String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let db_path = app_dir.join("library.db");

    let db = synthreader_core::db::LibraryDb::open(&db_path)
        .await
        .map_err(|e| e.to_string())?;

    db.update_folder(folder_id, &name, parent_id, sort_order)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_folder(app: tauri::AppHandle, folder_id: i64) -> Result<(), String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let db_path = app_dir.join("library.db");

    let db = synthreader_core::db::LibraryDb::open(&db_path)
        .await
        .map_err(|e| e.to_string())?;

    db.delete_folder(folder_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_feeds_by_folder(
    app: tauri::AppHandle,
    folder_id: i64,
) -> Result<Vec<synthreader_core::Feed>, String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let db_path = app_dir.join("library.db");

    let db = synthreader_core::db::LibraryDb::open(&db_path)
        .await
        .map_err(|e| e.to_string())?;

    db.get_feeds_by_folder(folder_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn move_feed_to_folder(
    app: tauri::AppHandle,
    feed_id: i64,
    folder_id: Option<i64>,
) -> Result<(), String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let db_path = app_dir.join("library.db");

    let db = synthreader_core::db::LibraryDb::open(&db_path)
        .await
        .map_err(|e| e.to_string())?;

    db.move_feed_to_folder(feed_id, folder_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn add_tag(
    app: tauri::AppHandle,
    name: String,
    color: Option<String>,
) -> Result<i64, String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let db_path = app_dir.join("library.db");

    let db = synthreader_core::db::LibraryDb::open(&db_path)
        .await
        .map_err(|e| e.to_string())?;

    db.add_tag(&name, color.as_deref())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_tags(app: tauri::AppHandle) -> Result<Vec<synthreader_core::Tag>, String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let db_path = app_dir.join("library.db");

    let db = synthreader_core::db::LibraryDb::open(&db_path)
        .await
        .map_err(|e| e.to_string())?;

    db.get_tags().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn tag_article(
    app: tauri::AppHandle,
    article_id: i64,
    tag_id: i64,
) -> Result<(), String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let db_path = app_dir.join("library.db");

    let db = synthreader_core::db::LibraryDb::open(&db_path)
        .await
        .map_err(|e| e.to_string())?;

    db.tag_article(article_id, tag_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn untag_article(
    app: tauri::AppHandle,
    article_id: i64,
    tag_id: i64,
) -> Result<(), String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let db_path = app_dir.join("library.db");

    let db = synthreader_core::db::LibraryDb::open(&db_path)
        .await
        .map_err(|e| e.to_string())?;

    db.untag_article(article_id, tag_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_article_tags(
    app: tauri::AppHandle,
    article_id: i64,
) -> Result<Vec<synthreader_core::Tag>, String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let db_path = app_dir.join("library.db");

    let db = synthreader_core::db::LibraryDb::open(&db_path)
        .await
        .map_err(|e| e.to_string())?;

    db.get_article_tags(article_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_articles_by_tag(
    app: tauri::AppHandle,
    tag_id: i64,
    limit: Option<i64>,
) -> Result<Vec<synthreader_core::Article>, String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let db_path = app_dir.join("library.db");

    let db = synthreader_core::db::LibraryDb::open(&db_path)
        .await
        .map_err(|e| e.to_string())?;

    db.get_articles_by_tag(tag_id, limit)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_tag(app: tauri::AppHandle, tag_id: i64) -> Result<(), String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let db_path = app_dir.join("library.db");

    let db = synthreader_core::db::LibraryDb::open(&db_path)
        .await
        .map_err(|e| e.to_string())?;

    db.delete_tag(tag_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_unread_articles(
    app: tauri::AppHandle,
    limit: Option<i64>,
) -> Result<Vec<synthreader_core::Article>, String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let db_path = app_dir.join("library.db");

    let db = synthreader_core::db::LibraryDb::open(&db_path)
        .await
        .map_err(|e| e.to_string())?;

    db.get_unread_articles(limit)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_starred_articles(
    app: tauri::AppHandle,
    limit: Option<i64>,
) -> Result<Vec<synthreader_core::Article>, String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let db_path = app_dir.join("library.db");

    let db = synthreader_core::db::LibraryDb::open(&db_path)
        .await
        .map_err(|e| e.to_string())?;

    db.get_starred_articles(limit)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_unread_count(app: tauri::AppHandle) -> Result<i64, String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let db_path = app_dir.join("library.db");

    let db = synthreader_core::db::LibraryDb::open(&db_path)
        .await
        .map_err(|e| e.to_string())?;

    db.get_unread_count().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn import_opml(
    app: tauri::AppHandle,
    xml_content: String,
) -> Result<serde_json::Value, String> {
    let doc = synthreader_core::parse_opml(&xml_content).map_err(|e| e.to_string())?;

    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let db_path = app_dir.join("library.db");

    let db = synthreader_core::db::LibraryDb::open(&db_path)
        .await
        .map_err(|e| e.to_string())?;

    let mut imported_feeds = 0;
    let mut imported_folders = 0;
    let mut errors = Vec::new();

    for folder in &doc.folders {
        let folder_id = db
            .add_folder(&folder.title, None, imported_folders)
            .await
            .map_err(|e| e.to_string())?;
        imported_folders += 1;

        for feed in &folder.feeds {
            match db
                .add_feed(
                    &feed.title,
                    &feed.url,
                    feed.site_url.as_deref(),
                    None,
                    Some(folder_id),
                )
                .await
            {
                Ok(_) => imported_feeds += 1,
                Err(e) => errors.push(format!("Failed to import feed '{}': {}", feed.title, e)),
            }
        }
    }

    for feed in &doc.feeds {
        match db
            .add_feed(&feed.title, &feed.url, feed.site_url.as_deref(), None, None)
            .await
        {
            Ok(_) => imported_feeds += 1,
            Err(e) => errors.push(format!("Failed to import feed '{}': {}", feed.title, e)),
        }
    }

    let result = serde_json::json!({
        "imported_feeds": imported_feeds,
        "imported_folders": imported_folders,
        "errors": errors,
    });

    Ok(result)
}

#[tauri::command]
async fn export_opml(app: tauri::AppHandle) -> Result<String, String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let db_path = app_dir.join("library.db");

    let db = synthreader_core::db::LibraryDb::open(&db_path)
        .await
        .map_err(|e| e.to_string())?;

    let feeds = db.get_feeds().await.map_err(|e| e.to_string())?;
    let folders = db.get_folders().await.map_err(|e| e.to_string())?;

    let mut opml_folders = Vec::new();
    let mut root_feeds = Vec::new();

    for folder in folders {
        let folder_feeds: Vec<synthreader_core::OpmlFeed> = feeds
            .iter()
            .filter(|f| f.folder_id == Some(folder.id))
            .map(|f| synthreader_core::OpmlFeed {
                title: f.title.clone(),
                url: f.url.clone(),
                site_url: f.site_url.clone(),
            })
            .collect();

        if !folder_feeds.is_empty() {
            opml_folders.push(synthreader_core::OpmlFolder {
                title: folder.name.clone(),
                feeds: folder_feeds,
            });
        }
    }

    for feed in feeds.iter().filter(|f| f.folder_id.is_none()) {
        root_feeds.push(synthreader_core::OpmlFeed {
            title: feed.title.clone(),
            url: feed.url.clone(),
            site_url: feed.site_url.clone(),
        });
    }

    let opml = synthreader_core::generate_opml("Synthreader Feeds", &opml_folders, &root_feeds);
    Ok(opml)
}

#[tauri::command]
async fn discover_feeds(url: String) -> Result<Vec<synthreader_core::DiscoveredFeed>, String> {
    synthreader_core::discover_feeds(&url)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_feed(app: tauri::AppHandle, feed_id: i64) -> Result<(), String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let db_path = app_dir.join("library.db");

    let db = synthreader_core::db::LibraryDb::open(&db_path)
        .await
        .map_err(|e| e.to_string())?;

    db.delete_feed(feed_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn update_feed(
    app: tauri::AppHandle,
    feed_id: i64,
    title: String,
    url: String,
    site_url: Option<String>,
    description: Option<String>,
    folder_id: Option<i64>,
) -> Result<(), String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let db_path = app_dir.join("library.db");

    let db = synthreader_core::db::LibraryDb::open(&db_path)
        .await
        .map_err(|e| e.to_string())?;

    db.update_feed(feed_id, &title, &url, site_url.as_deref(), description.as_deref(), folder_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_feed_stats(
    app: tauri::AppHandle,
    feed_id: i64,
) -> Result<Option<synthreader_core::models::FeedStats>, String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let db_path = app_dir.join("library.db");

    let db = synthreader_core::db::LibraryDb::open(&db_path)
        .await
        .map_err(|e| e.to_string())?;

    db.get_feed_stats(feed_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_all_feed_stats(
    app: tauri::AppHandle,
) -> Result<Vec<synthreader_core::models::FeedStats>, String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let db_path = app_dir.join("library.db");

    let db = synthreader_core::db::LibraryDb::open(&db_path)
        .await
        .map_err(|e| e.to_string())?;

    db.get_all_feed_stats()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_library_stats(
    app: tauri::AppHandle,
) -> Result<synthreader_core::models::LibraryStats, String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let db_path = app_dir.join("library.db");

    let db = synthreader_core::db::LibraryDb::open(&db_path)
        .await
        .map_err(|e| e.to_string())?;

    db.get_library_stats()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_article(app: tauri::AppHandle, article_id: i64) -> Result<(), String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let db_path = app_dir.join("library.db");

    let db = synthreader_core::db::LibraryDb::open(&db_path)
        .await
        .map_err(|e| e.to_string())?;

    db.delete_article(article_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_old_articles(
    app: tauri::AppHandle,
    older_than_days: i64,
    keep_starred: bool,
) -> Result<usize, String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let db_path = app_dir.join("library.db");

    let db = synthreader_core::db::LibraryDb::open(&db_path)
        .await
        .map_err(|e| e.to_string())?;

    db.delete_old_articles(older_than_days, keep_starred)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn mark_all_articles_read(
    app: tauri::AppHandle,
    feed_id: Option<i64>,
) -> Result<usize, String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let db_path = app_dir.join("library.db");

    let db = synthreader_core::db::LibraryDb::open(&db_path)
        .await
        .map_err(|e| e.to_string())?;

    db.mark_all_articles_read(feed_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn find_duplicate_articles(
    app: tauri::AppHandle,
    feed_id: i64,
) -> Result<Vec<(i64, String)>, String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let db_path = app_dir.join("library.db");

    let db = synthreader_core::db::LibraryDb::open(&db_path)
        .await
        .map_err(|e| e.to_string())?;

    db.find_duplicate_articles(feed_id)
        .await
        .map_err(|e| e.to_string())
}

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
