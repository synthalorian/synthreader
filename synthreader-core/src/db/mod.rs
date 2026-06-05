use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use std::path::Path;

pub struct LibraryDb {
    pool: SqlitePool,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BookRow {
    pub id: i64,
    pub title: String,
    pub cover_path: Option<String>,
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

    pub async fn add_book(
        &self,
        metadata: &crate::BookMetadata,
        cover_path: &Option<std::path::PathBuf>,
    ) -> anyhow::Result<i64> {
        let cover = cover_path.as_ref().map(|p| p.to_string_lossy().to_string());
        
        let id: i64 = sqlx::query_scalar(
            r#"
            INSERT INTO books (title, subtitle, description, publisher, published_date, language, isbn_10, isbn_13, page_count, cover_path)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            RETURNING id
            "#
        )
        .bind(&metadata.title)
        .bind(&metadata.subtitle)
        .bind(&metadata.description)
        .bind(&metadata.publisher)
        .bind(&metadata.published_date)
        .bind(&metadata.language)
        .bind(&metadata.isbn_10)
        .bind(&metadata.isbn_13)
        .bind(metadata.page_count)
        .bind(&cover)
        .fetch_one(&self.pool)
        .await?;

        // Add authors
        for author in &metadata.authors {
            let author_id: i64 = sqlx::query_scalar(
                "INSERT INTO authors (name, sort_name) VALUES (?1, ?1) ON CONFLICT(name) DO UPDATE SET name=excluded.name RETURNING id"
            )
            .bind(author)
            .fetch_one(&self.pool)
            .await?;

            sqlx::query(
                "INSERT INTO book_authors (book_id, author_id) VALUES (?1, ?2) ON CONFLICT DO NOTHING"
            )
            .bind(id)
            .bind(author_id)
            .execute(&self.pool)
            .await?;
        }

        Ok(id)
    }

    pub async fn add_book_file(
        &self,
        book_id: i64,
        format: &str,
        file_path: &Path,
        file_size: Option<i64>,
    ) -> anyhow::Result<()> {
        sqlx::query(
            "INSERT INTO book_files (book_id, format, file_path, file_size) VALUES (?1, ?2, ?3, ?4)"
        )
        .bind(book_id)
        .bind(format)
        .bind(file_path.to_string_lossy())
        .bind(file_size)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_books(&self) -> anyhow::Result<Vec<BookRow>> {
        let books = sqlx::query_as::<_, BookRow>(
            "SELECT id, title, cover_path FROM books ORDER BY added_date DESC"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(books)
    }

    pub async fn get_book_authors(&self, book_id: i64) -> anyhow::Result<Vec<String>> {
        let authors = sqlx::query_scalar::<_, String>(
            "SELECT a.name FROM authors a JOIN book_authors ba ON a.id = ba.author_id WHERE ba.book_id = ?1"
        )
        .bind(book_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(authors)
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
