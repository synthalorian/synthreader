use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use std::path::Path;

pub struct LibraryDb {
    pool: SqlitePool,
}

impl LibraryDb {
    pub async fn open(path: &Path) -> anyhow::Result<Self> {
        let url = format!("sqlite:{}", path.display());
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(&url)
            .await?;
        
        Self::init_schema(&pool).await?;
        
        Ok(Self { pool })
    }

    async fn init_schema(pool: &SqlitePool) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS books (
                id INTEGER PRIMARY KEY,
                title TEXT NOT NULL,
                subtitle TEXT,
                description TEXT,
                publisher TEXT,
                published_date TEXT,
                language TEXT,
                isbn_10 TEXT,
                isbn_13 TEXT,
                page_count INTEGER,
                rating REAL,
                cover_path TEXT,
                added_date TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                modified_date TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS authors (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                sort_name TEXT,
                biography TEXT
            );

            CREATE TABLE IF NOT EXISTS book_authors (
                book_id INTEGER REFERENCES books(id) ON DELETE CASCADE,
                author_id INTEGER REFERENCES authors(id) ON DELETE CASCADE,
                role TEXT DEFAULT 'author',
                PRIMARY KEY (book_id, author_id)
            );

            CREATE TABLE IF NOT EXISTS tags (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                color TEXT DEFAULT '#05d9e8'
            );

            CREATE TABLE IF NOT EXISTS book_tags (
                book_id INTEGER REFERENCES books(id) ON DELETE CASCADE,
                tag_id INTEGER REFERENCES tags(id) ON DELETE CASCADE,
                PRIMARY KEY (book_id, tag_id)
            );

            CREATE TABLE IF NOT EXISTS collections (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                sort_order INTEGER DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS book_collections (
                book_id INTEGER REFERENCES books(id) ON DELETE CASCADE,
                collection_id INTEGER REFERENCES collections(id) ON DELETE CASCADE,
                sort_order INTEGER DEFAULT 0,
                PRIMARY KEY (book_id, collection_id)
            );

            CREATE TABLE IF NOT EXISTS book_files (
                id INTEGER PRIMARY KEY,
                book_id INTEGER REFERENCES books(id) ON DELETE CASCADE,
                format TEXT NOT NULL,
                file_path TEXT NOT NULL,
                file_size INTEGER,
                added_date TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS reading_progress (
                book_id INTEGER PRIMARY KEY REFERENCES books(id) ON DELETE CASCADE,
                current_file_id INTEGER REFERENCES book_files(id),
                current_position REAL DEFAULT 0,
                current_cfi TEXT,
                last_read_date TIMESTAMP,
                total_reading_time INTEGER DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS annotations (
                id INTEGER PRIMARY KEY,
                book_id INTEGER REFERENCES books(id) ON DELETE CASCADE,
                file_id INTEGER REFERENCES book_files(id),
                type TEXT NOT NULL,
                start_cfi TEXT NOT NULL,
                end_cfi TEXT,
                start_position REAL,
                end_position REAL,
                color TEXT DEFAULT '#ff9e00',
                note_text TEXT,
                created_date TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                modified_date TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS series (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                description TEXT
            );

            CREATE TABLE IF NOT EXISTS book_series (
                book_id INTEGER REFERENCES books(id) ON DELETE CASCADE,
                series_id INTEGER REFERENCES series(id) ON DELETE CASCADE,
                series_index REAL,
                PRIMARY KEY (book_id, series_id)
            );
            "#
        )
        .execute(pool)
        .await?;

        Ok(())
    }
}
