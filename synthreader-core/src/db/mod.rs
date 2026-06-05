use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use std::path::Path;

use crate::models::{Article, Feed, Folder, Tag};

pub struct LibraryDb {
    pool: SqlitePool,
}

impl LibraryDb {
    pub(crate) fn pool(&self) -> &SqlitePool {
        &self.pool
    }
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

    // -- Book methods (legacy, kept for backward compatibility) --

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

    // -- Feed methods --

    pub async fn add_feed(
        &self,
        title: &str,
        url: &str,
        site_url: Option<&str>,
        description: Option<&str>,
        folder_id: Option<i64>,
    ) -> anyhow::Result<i64> {
        let id: i64 = sqlx::query_scalar(
            r#"
            INSERT INTO feeds (title, url, site_url, description, folder_id)
            VALUES (?1, ?2, ?3, ?4, ?5)
            RETURNING id
            "#
        )
        .bind(title)
        .bind(url)
        .bind(site_url)
        .bind(description)
        .bind(folder_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(id)
    }

    pub async fn get_feeds(&self) -> anyhow::Result<Vec<Feed>> {
        let feeds = sqlx::query_as::<_, Feed>(
            "SELECT id, title, url, site_url, description, folder_id, last_fetched, created_at FROM feeds ORDER BY created_at DESC"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(feeds)
    }

    pub async fn update_feed_last_fetched(
        &self,
        feed_id: i64,
    ) -> anyhow::Result<()> {
        sqlx::query(
            "UPDATE feeds SET last_fetched = CURRENT_TIMESTAMP WHERE id = ?1"
        )
        .bind(feed_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // -- Article methods --

    #[allow(clippy::too_many_arguments)]
    pub async fn add_article(
        &self,
        feed_id: i64,
        title: &str,
        url: &str,
        content: Option<&str>,
        summary: Option<&str>,
        author: Option<&str>,
        published_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> anyhow::Result<i64> {
        let id: i64 = sqlx::query_scalar(
            r#"
            INSERT INTO articles (feed_id, title, url, content, summary, author, published_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            RETURNING id
            "#
        )
        .bind(feed_id)
        .bind(title)
        .bind(url)
        .bind(content)
        .bind(summary)
        .bind(author)
        .bind(published_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(id)
    }

    pub async fn get_articles_for_feed(
        &self,
        feed_id: i64,
    ) -> anyhow::Result<Vec<Article>> {
        let articles = sqlx::query_as::<_, Article>(
            r#"
            SELECT id, feed_id, title, url, content, summary, author, published_at, read, starred, created_at
            FROM articles
            WHERE feed_id = ?1
            ORDER BY published_at DESC NULLS LAST, created_at DESC
            "#
        )
        .bind(feed_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(articles)
    }

    pub async fn mark_article_read(
        &self,
        article_id: i64,
        read: bool,
    ) -> anyhow::Result<()> {
        sqlx::query(
            "UPDATE articles SET read = ?1 WHERE id = ?2"
        )
        .bind(read)
        .bind(article_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn star_article(
        &self,
        article_id: i64,
        starred: bool,
    ) -> anyhow::Result<()> {
        sqlx::query(
            "UPDATE articles SET starred = ?1 WHERE id = ?2"
        )
        .bind(starred)
        .bind(article_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // -- Folder methods --

    pub async fn add_folder(
        &self,
        name: &str,
        parent_id: Option<i64>,
        sort_order: i32,
    ) -> anyhow::Result<i64> {
        let id: i64 = sqlx::query_scalar(
            r#"
            INSERT INTO folders (name, parent_id, sort_order)
            VALUES (?1, ?2, ?3)
            RETURNING id
            "#
        )
        .bind(name)
        .bind(parent_id)
        .bind(sort_order)
        .fetch_one(&self.pool)
        .await?;

        Ok(id)
    }

    pub async fn get_folders(&self) -> anyhow::Result<Vec<Folder>> {
        let folders = sqlx::query_as::<_, Folder>(
            "SELECT id, name, parent_id, sort_order FROM folders ORDER BY sort_order, name"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(folders)
    }

    // -- Tag methods --

    pub async fn add_tag(
        &self,
        name: &str,
        color: Option<&str>,
    ) -> anyhow::Result<i64> {
        let id: i64 = sqlx::query_scalar(
            r#"
            INSERT INTO tags (name, color)
            VALUES (?1, ?2)
            ON CONFLICT(name) DO UPDATE SET name=excluded.name
            RETURNING id
            "#
        )
        .bind(name)
        .bind(color)
        .fetch_one(&self.pool)
        .await?;

        Ok(id)
    }

    pub async fn get_tags(&self) -> anyhow::Result<Vec<Tag>> {
        let tags = sqlx::query_as::<_, Tag>(
            "SELECT id, name, color FROM tags ORDER BY name"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(tags)
    }

    pub async fn tag_article(
        &self,
        article_id: i64,
        tag_id: i64,
    ) -> anyhow::Result<()> {
        sqlx::query(
            "INSERT INTO article_tags (article_id, tag_id) VALUES (?1, ?2) ON CONFLICT DO NOTHING"
        )
        .bind(article_id)
        .bind(tag_id)
        .execute(&self.pool)
        .await?;

        Ok(())
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

            CREATE TABLE IF NOT EXISTS feeds (
                id INTEGER PRIMARY KEY,
                title TEXT NOT NULL,
                url TEXT NOT NULL UNIQUE,
                site_url TEXT,
                description TEXT,
                folder_id INTEGER REFERENCES folders(id) ON DELETE SET NULL,
                last_fetched TIMESTAMP,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS articles (
                id INTEGER PRIMARY KEY,
                feed_id INTEGER NOT NULL REFERENCES feeds(id) ON DELETE CASCADE,
                title TEXT NOT NULL,
                url TEXT NOT NULL,
                content TEXT,
                summary TEXT,
                author TEXT,
                published_at TIMESTAMP,
                read INTEGER DEFAULT 0,
                starred INTEGER DEFAULT 0,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS folders (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                parent_id INTEGER REFERENCES folders(id) ON DELETE CASCADE,
                sort_order INTEGER DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS article_tags (
                article_id INTEGER REFERENCES articles(id) ON DELETE CASCADE,
                tag_id INTEGER REFERENCES tags(id) ON DELETE CASCADE,
                PRIMARY KEY (article_id, tag_id)
            );
            "#
        )
        .execute(pool)
        .await?;

        Ok(())
    }
}
