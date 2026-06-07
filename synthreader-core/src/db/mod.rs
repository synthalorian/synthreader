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

    #[cfg(test)]
    pub(crate) fn from_pool(pool: SqlitePool) -> Self {
        Self { pool }
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
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let url = format!("sqlite:///{}?mode=rwc", path.display());
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

    pub async fn delete_book(&self, book_id: i64) -> anyhow::Result<()> {
        // Get cover path before deletion
        let cover_path: Option<String> = sqlx::query_scalar(
            "SELECT cover_path FROM books WHERE id = ?1"
        )
        .bind(book_id)
        .fetch_optional(&self.pool)
        .await?;

        // Delete book (cascades to book_authors, book_files via FK constraints)
        sqlx::query("DELETE FROM books WHERE id = ?1")
            .bind(book_id)
            .execute(&self.pool)
            .await?;

        // Clean up cover file if exists
        if let Some(path) = cover_path {
            let _ = std::fs::remove_file(path);
        }

        Ok(())
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
            "SELECT id, title, url, site_url, description, folder_id, last_fetched, last_error, consecutive_errors, created_at FROM feeds ORDER BY created_at DESC"
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

    pub async fn delete_feed(&self, feed_id: i64) -> anyhow::Result<()> {
        // Articles will be deleted via ON DELETE CASCADE
        sqlx::query("DELETE FROM feeds WHERE id = ?1")
            .bind(feed_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn update_feed(
        &self,
        feed_id: i64,
        title: &str,
        url: &str,
        site_url: Option<&str>,
        description: Option<&str>,
        folder_id: Option<i64>,
    ) -> anyhow::Result<()> {
        sqlx::query(
            "UPDATE feeds SET title = ?1, url = ?2, site_url = ?3, description = ?4, folder_id = ?5 WHERE id = ?6"
        )
        .bind(title)
        .bind(url)
        .bind(site_url)
        .bind(description)
        .bind(folder_id)
        .bind(feed_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn record_feed_error(
        &self,
        feed_id: i64,
        error: &str,
    ) -> anyhow::Result<()> {
        sqlx::query(
            "UPDATE feeds SET last_error = ?1, consecutive_errors = consecutive_errors + 1 WHERE id = ?2"
        )
        .bind(error)
        .bind(feed_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn clear_feed_error(
        &self,
        feed_id: i64,
    ) -> anyhow::Result<()> {
        sqlx::query(
            "UPDATE feeds SET last_error = NULL, consecutive_errors = 0 WHERE id = ?1"
        )
        .bind(feed_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_feed_stats(
        &self,
        feed_id: i64,
    ) -> anyhow::Result<Option<crate::models::FeedStats>> {
        let stats = sqlx::query_as::<_, crate::models::FeedStats>(
            r#"
            SELECT
                f.id as feed_id,
                f.title as feed_title,
                COUNT(a.id) as total_articles,
                SUM(CASE WHEN a.read = 0 THEN 1 ELSE 0 END) as unread_count,
                SUM(CASE WHEN a.starred = 1 THEN 1 ELSE 0 END) as starred_count,
                f.last_fetched,
                f.last_error,
                f.consecutive_errors
            FROM feeds f
            LEFT JOIN articles a ON a.feed_id = f.id
            WHERE f.id = ?1
            GROUP BY f.id
            "#
        )
        .bind(feed_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(stats)
    }

    pub async fn get_all_feed_stats(
        &self,
    ) -> anyhow::Result<Vec<crate::models::FeedStats>> {
        let stats = sqlx::query_as::<_, crate::models::FeedStats>(
            r#"
            SELECT
                f.id as feed_id,
                f.title as feed_title,
                COUNT(a.id) as total_articles,
                SUM(CASE WHEN a.read = 0 THEN 1 ELSE 0 END) as unread_count,
                SUM(CASE WHEN a.starred = 1 THEN 1 ELSE 0 END) as starred_count,
                f.last_fetched,
                f.last_error,
                f.consecutive_errors
            FROM feeds f
            LEFT JOIN articles a ON a.feed_id = f.id
            GROUP BY f.id
            ORDER BY f.title
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(stats)
    }

    pub async fn get_library_stats(&self) -> anyhow::Result<crate::models::LibraryStats> {
        let total_feeds: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM feeds")
            .fetch_one(&self.pool)
            .await?;

        let total_articles: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM articles")
            .fetch_one(&self.pool)
            .await?;

        let total_unread: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM articles WHERE read = 0")
            .fetch_one(&self.pool)
            .await?;

        let total_starred: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM articles WHERE starred = 1")
            .fetch_one(&self.pool)
            .await?;

        let feeds_with_errors: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM feeds WHERE consecutive_errors > 0"
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(crate::models::LibraryStats {
            total_feeds,
            total_articles,
            total_unread,
            total_starred,
            feeds_with_errors,
        })
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

    pub async fn get_all_articles(
        &self,
        limit: Option<i64>,
    ) -> anyhow::Result<Vec<Article>> {
        let limit = limit.unwrap_or(500);
        let articles = sqlx::query_as::<_, Article>(
            r#"
            SELECT id, feed_id, title, url, content, summary, author, published_at, read, starred, created_at
            FROM articles
            ORDER BY published_at DESC NULLS LAST, created_at DESC
            LIMIT ?1
            "#
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(articles)
    }

    pub async fn get_unread_articles(
        &self,
        limit: Option<i64>,
    ) -> anyhow::Result<Vec<Article>> {
        let limit = limit.unwrap_or(500);
        let articles = sqlx::query_as::<_, Article>(
            r#"
            SELECT id, feed_id, title, url, content, summary, author, published_at, read, starred, created_at
            FROM articles
            WHERE read = 0
            ORDER BY published_at DESC NULLS LAST, created_at DESC
            LIMIT ?1
            "#
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(articles)
    }

    pub async fn get_starred_articles(
        &self,
        limit: Option<i64>,
    ) -> anyhow::Result<Vec<Article>> {
        let limit = limit.unwrap_or(500);
        let articles = sqlx::query_as::<_, Article>(
            r#"
            SELECT id, feed_id, title, url, content, summary, author, published_at, read, starred, created_at
            FROM articles
            WHERE starred = 1
            ORDER BY published_at DESC NULLS LAST, created_at DESC
            LIMIT ?1
            "#
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(articles)
    }

    pub async fn get_unread_count(&self) -> anyhow::Result<i64> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM articles WHERE read = 0"
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(count)
    }

    pub async fn get_feed_unread_count(
        &self,
        feed_id: i64,
    ) -> anyhow::Result<i64> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM articles WHERE feed_id = ?1 AND read = 0"
        )
        .bind(feed_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(count)
    }

    pub async fn get_article(&self, article_id: i64) -> anyhow::Result<Option<Article>> {
        let article = sqlx::query_as::<_, Article>(
            r#"
            SELECT id, feed_id, title, url, content, summary, author, published_at, read, starred, created_at
            FROM articles
            WHERE id = ?1
            "#
        )
        .bind(article_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(article)
    }

    pub async fn search_articles(
        &self,
        query: &str,
        limit: Option<i64>,
    ) -> anyhow::Result<Vec<Article>> {
        let limit = limit.unwrap_or(100);
        let search_query = format!("{}*", query);

        let articles = sqlx::query_as::<_, Article>(
            r#"
            SELECT a.id, a.feed_id, a.title, a.url, a.content, a.summary, a.author, a.published_at, a.read, a.starred, a.created_at
            FROM articles_fts fts
            JOIN articles a ON a.id = fts.rowid
            WHERE articles_fts MATCH ?1
            ORDER BY rank
            LIMIT ?2
            "#
        )
        .bind(search_query)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(articles)
    }

    pub async fn update_article_content(
        &self,
        article_id: i64,
        content: Option<&str>,
    ) -> anyhow::Result<()> {
        sqlx::query(
            "UPDATE articles SET content = ?1 WHERE id = ?2"
        )
        .bind(content)
        .bind(article_id)
        .execute(&self.pool)
        .await?;

        Ok(())
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

    pub async fn update_folder(
        &self,
        folder_id: i64,
        name: &str,
        parent_id: Option<i64>,
        sort_order: i32,
    ) -> anyhow::Result<()> {
        sqlx::query(
            "UPDATE folders SET name = ?1, parent_id = ?2, sort_order = ?3 WHERE id = ?4"
        )
        .bind(name)
        .bind(parent_id)
        .bind(sort_order)
        .bind(folder_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn delete_folder(&self, folder_id: i64) -> anyhow::Result<()> {
        // Feeds in this folder will have folder_id set to NULL due to ON DELETE SET NULL
        sqlx::query("DELETE FROM folders WHERE id = ?1")
            .bind(folder_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn get_feeds_by_folder(
        &self,
        folder_id: i64,
    ) -> anyhow::Result<Vec<Feed>> {
        let feeds = sqlx::query_as::<_, Feed>(
            "SELECT id, title, url, site_url, description, folder_id, last_fetched, last_error, consecutive_errors, created_at FROM feeds WHERE folder_id = ?1 ORDER BY created_at DESC"
        )
        .bind(folder_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(feeds)
    }

    pub async fn move_feed_to_folder(
        &self,
        feed_id: i64,
        folder_id: Option<i64>,
    ) -> anyhow::Result<()> {
        sqlx::query("UPDATE feeds SET folder_id = ?1 WHERE id = ?2")
            .bind(folder_id)
            .bind(feed_id)
            .execute(&self.pool)
            .await?;

        Ok(())
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

    pub async fn untag_article(
        &self,
        article_id: i64,
        tag_id: i64,
    ) -> anyhow::Result<()> {
        sqlx::query(
            "DELETE FROM article_tags WHERE article_id = ?1 AND tag_id = ?2"
        )
        .bind(article_id)
        .bind(tag_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_article_tags(
        &self,
        article_id: i64,
    ) -> anyhow::Result<Vec<Tag>> {
        let tags = sqlx::query_as::<_, Tag>(
            r#"
            SELECT t.id, t.name, t.color
            FROM tags t
            JOIN article_tags at ON t.id = at.tag_id
            WHERE at.article_id = ?1
            ORDER BY t.name
            "#
        )
        .bind(article_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(tags)
    }

    pub async fn get_articles_by_tag(
        &self,
        tag_id: i64,
        limit: Option<i64>,
    ) -> anyhow::Result<Vec<Article>> {
        let limit = limit.unwrap_or(500);
        let articles = sqlx::query_as::<_, Article>(
            r#"
            SELECT a.id, a.feed_id, a.title, a.url, a.content, a.summary, a.author, a.published_at, a.read, a.starred, a.created_at
            FROM articles a
            JOIN article_tags at ON a.id = at.article_id
            WHERE at.tag_id = ?1
            ORDER BY a.published_at DESC NULLS LAST, a.created_at DESC
            LIMIT ?2
            "#
        )
        .bind(tag_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(articles)
    }

    pub async fn delete_tag(&self, tag_id: i64) -> anyhow::Result<()> {
        sqlx::query("DELETE FROM tags WHERE id = ?1")
            .bind(tag_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn delete_old_articles(
        &self,
        older_than_days: i64,
        keep_starred: bool,
    ) -> anyhow::Result<usize> {
        let cutoff = chrono::Utc::now() - chrono::Duration::days(older_than_days);

        let result = if keep_starred {
            sqlx::query(
                "DELETE FROM articles WHERE created_at < ?1 AND starred = 0"
            )
            .bind(cutoff)
            .execute(&self.pool)
            .await?
        } else {
            sqlx::query(
                "DELETE FROM articles WHERE created_at < ?1"
            )
            .bind(cutoff)
            .execute(&self.pool)
            .await?
        };

        Ok(result.rows_affected() as usize)
    }

    pub async fn delete_article(&self, article_id: i64) -> anyhow::Result<()> {
        sqlx::query("DELETE FROM articles WHERE id = ?1")
            .bind(article_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn mark_all_articles_read(
        &self,
        feed_id: Option<i64>,
    ) -> anyhow::Result<usize> {
        let result = if let Some(fid) = feed_id {
            sqlx::query("UPDATE articles SET read = 1 WHERE feed_id = ?1 AND read = 0")
                .bind(fid)
                .execute(&self.pool)
                .await?
        } else {
            sqlx::query("UPDATE articles SET read = 1 WHERE read = 0")
                .execute(&self.pool)
                .await?
        };

        Ok(result.rows_affected() as usize)
    }

    pub async fn find_duplicate_articles(
        &self,
        feed_id: i64,
    ) -> anyhow::Result<Vec<(i64, String)>> {
        let duplicates = sqlx::query_as::<_, (i64, String)>(
            r#"
            SELECT id, title
            FROM articles
            WHERE feed_id = ?1
              AND id NOT IN (
                SELECT MIN(id)
                FROM articles
                WHERE feed_id = ?1
                GROUP BY url
              )
            "#
        )
        .bind(feed_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(duplicates)
    }

    // -- Settings methods --

    pub async fn get_setting(&self, key: &str) -> anyhow::Result<Option<String>> {
        let value: Option<String> = sqlx::query_scalar(
            "SELECT value FROM settings WHERE key = ?1"
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await?;

        Ok(value)
    }

    pub async fn set_setting(&self, key: &str, value: &str) -> anyhow::Result<()> {
        sqlx::query(
            "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value"
        )
        .bind(key)
        .bind(value)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // -- Book file methods --

    pub async fn get_book_file_path(&self, book_id: i64) -> anyhow::Result<Option<String>> {
        let path: Option<String> = sqlx::query_scalar(
            "SELECT file_path FROM book_files WHERE book_id = ?1 LIMIT 1"
        )
        .bind(book_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(path)
    }

    pub async fn save_reading_progress(
        &self,
        book_id: i64,
        chapter_index: i64,
        scroll_position: f64,
    ) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            INSERT INTO reading_progress (book_id, current_position, current_cfi, last_read_date, total_reading_time)
            VALUES (?1, ?2, ?3, CURRENT_TIMESTAMP, 0)
            ON CONFLICT(book_id) DO UPDATE SET
                current_position = excluded.current_position,
                current_cfi = excluded.current_cfi,
                last_read_date = excluded.last_read_date
            "#
        )
        .bind(book_id)
        .bind(scroll_position)
        .bind(chapter_index.to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_reading_progress(
        &self,
        book_id: i64,
    ) -> anyhow::Result<Option<(i64, f64)>> {
        let row: Option<(String, f64)> = sqlx::query_as(
            "SELECT current_cfi, current_position FROM reading_progress WHERE book_id = ?1"
        )
        .bind(book_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.and_then(|(cfi, pos)| {
            let chapter_index = cfi.parse().ok()?;
            Some((chapter_index, pos))
        }))
    }

    pub(crate) async fn init_schema(pool: &SqlitePool) -> anyhow::Result<()> {
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
                last_error TEXT,
                consecutive_errors INTEGER DEFAULT 0,
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

            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            CREATE VIRTUAL TABLE IF NOT EXISTS articles_fts USING fts5(
                title, content,
                content='articles',
                content_rowid='id'
            );

            CREATE TRIGGER IF NOT EXISTS articles_fts_insert AFTER INSERT ON articles BEGIN
                INSERT INTO articles_fts(rowid, title, content)
                VALUES (new.id, new.title, COALESCE(new.content, ''));
            END;

            CREATE TRIGGER IF NOT EXISTS articles_fts_delete AFTER DELETE ON articles BEGIN
                INSERT INTO articles_fts(articles_fts, rowid, title, content)
                VALUES ('delete', old.id, old.title, COALESCE(old.content, ''));
            END;

            CREATE TRIGGER IF NOT EXISTS articles_fts_update AFTER UPDATE ON articles BEGIN
                INSERT INTO articles_fts(articles_fts, rowid, title, content)
                VALUES ('delete', old.id, old.title, COALESCE(old.content, ''));
                INSERT INTO articles_fts(rowid, title, content)
                VALUES (new.id, new.title, COALESCE(new.content, ''));
            END;
            "#
        )
        .execute(pool)
        .await?;

        // Populate FTS index if empty (first run or migration)
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM articles_fts")
            .fetch_one(pool)
            .await?;

        if count == 0 {
            sqlx::query(
                "INSERT INTO articles_fts(rowid, title, content)
                 SELECT id, title, COALESCE(content, '') FROM articles"
            )
            .execute(pool)
            .await?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn create_test_db() -> LibraryDb {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect("sqlite::memory:")
            .await
            .unwrap();

        LibraryDb::init_schema(&pool).await.unwrap();

        LibraryDb { pool }
    }

    #[tokio::test]
    async fn test_add_and_get_feed() {
        let db = create_test_db().await;

        let feed_id = db
            .add_feed("Test Feed", "https://example.com/feed", Some("https://example.com"), Some("A test feed"), None)
            .await
            .unwrap();

        assert!(feed_id > 0);

        let feeds = db.get_feeds().await.unwrap();
        assert_eq!(feeds.len(), 1);
        assert_eq!(feeds[0].title, "Test Feed");
        assert_eq!(feeds[0].url, "https://example.com/feed");
    }

    #[tokio::test]
    async fn test_delete_feed() {
        let db = create_test_db().await;

        let feed_id = db
            .add_feed("Test Feed", "https://example.com/feed", None, None, None)
            .await
            .unwrap();

        db.delete_feed(feed_id).await.unwrap();

        let feeds = db.get_feeds().await.unwrap();
        assert!(feeds.is_empty());
    }

    #[tokio::test]
    async fn test_update_feed() {
        let db = create_test_db().await;

        let feed_id = db
            .add_feed("Original", "https://example.com/feed", None, None, None)
            .await
            .unwrap();

        db.update_feed(feed_id, "Updated", "https://example.com/updated", Some("https://example.com"), Some("Updated desc"), None)
            .await
            .unwrap();

        let feeds = db.get_feeds().await.unwrap();
        assert_eq!(feeds[0].title, "Updated");
        assert_eq!(feeds[0].url, "https://example.com/updated");
    }

    #[tokio::test]
    async fn test_feed_error_tracking() {
        let db = create_test_db().await;

        let feed_id = db
            .add_feed("Test Feed", "https://example.com/feed", None, None, None)
            .await
            .unwrap();

        db.record_feed_error(feed_id, "Network timeout").await.unwrap();

        let stats = db.get_feed_stats(feed_id).await.unwrap().unwrap();
        assert_eq!(stats.last_error, Some("Network timeout".to_string()));
        assert_eq!(stats.consecutive_errors, 1);

        db.record_feed_error(feed_id, "DNS failure").await.unwrap();

        let stats = db.get_feed_stats(feed_id).await.unwrap().unwrap();
        assert_eq!(stats.consecutive_errors, 2);

        db.clear_feed_error(feed_id).await.unwrap();

        let stats = db.get_feed_stats(feed_id).await.unwrap().unwrap();
        assert_eq!(stats.last_error, None);
        assert_eq!(stats.consecutive_errors, 0);
    }

    #[tokio::test]
    async fn test_feed_stats() {
        let db = create_test_db().await;

        let feed_id = db
            .add_feed("Test Feed", "https://example.com/feed", None, None, None)
            .await
            .unwrap();

        db.add_article(feed_id, "Article 1", "https://example.com/1", None, None, None, None)
            .await
            .unwrap();
        db.add_article(feed_id, "Article 2", "https://example.com/2", None, None, None, None)
            .await
            .unwrap();

        let stats = db.get_feed_stats(feed_id).await.unwrap().unwrap();
        assert_eq!(stats.total_articles, 2);
        assert_eq!(stats.unread_count, 2);
        assert_eq!(stats.starred_count, 0);
    }

    #[tokio::test]
    async fn test_library_stats() {
        let db = create_test_db().await;

        let feed_id = db
            .add_feed("Test Feed", "https://example.com/feed", None, None, None)
            .await
            .unwrap();

        db.add_article(feed_id, "Article 1", "https://example.com/1", None, None, None, None)
            .await
            .unwrap();
        db.add_article(feed_id, "Article 2", "https://example.com/2", None, None, None, None)
            .await
            .unwrap();

        db.mark_article_read(1, true).await.unwrap();
        db.star_article(2, true).await.unwrap();

        let stats = db.get_library_stats().await.unwrap();
        assert_eq!(stats.total_feeds, 1);
        assert_eq!(stats.total_articles, 2);
        assert_eq!(stats.total_unread, 1);
        assert_eq!(stats.total_starred, 1);
    }

    #[tokio::test]
    async fn test_delete_old_articles() {
        let db = create_test_db().await;

        let feed_id = db
            .add_feed("Test Feed", "https://example.com/feed", None, None, None)
            .await
            .unwrap();

        db.add_article(feed_id, "Article 1", "https://example.com/1", None, None, None, None)
            .await
            .unwrap();

        let deleted = db.delete_old_articles(0, false).await.unwrap();
        assert_eq!(deleted, 1);

        let articles = db.get_all_articles(None).await.unwrap();
        assert!(articles.is_empty());
    }

    #[tokio::test]
    async fn test_mark_all_articles_read() {
        let db = create_test_db().await;

        let feed_id = db
            .add_feed("Test Feed", "https://example.com/feed", None, None, None)
            .await
            .unwrap();

        db.add_article(feed_id, "Article 1", "https://example.com/1", None, None, None, None)
            .await
            .unwrap();
        db.add_article(feed_id, "Article 2", "https://example.com/2", None, None, None, None)
            .await
            .unwrap();

        let marked = db.mark_all_articles_read(Some(feed_id)).await.unwrap();
        assert_eq!(marked, 2);

        let unread = db.get_unread_articles(None).await.unwrap();
        assert!(unread.is_empty());
    }

    #[tokio::test]
    async fn test_find_duplicate_articles() {
        let db = create_test_db().await;

        let feed_id = db
            .add_feed("Test Feed", "https://example.com/feed", None, None, None)
            .await
            .unwrap();

        db.add_article(feed_id, "Article 1", "https://example.com/dup", None, None, None, None)
            .await
            .unwrap();
        db.add_article(feed_id, "Article 2", "https://example.com/dup", None, None, None, None)
            .await
            .unwrap();
        db.add_article(feed_id, "Article 3", "https://example.com/unique", None, None, None, None)
            .await
            .unwrap();

        let duplicates = db.find_duplicate_articles(feed_id).await.unwrap();
        assert_eq!(duplicates.len(), 1);
        assert_eq!(duplicates[0].1, "Article 2");
    }

    #[tokio::test]
    async fn test_folder_operations() {
        let db = create_test_db().await;

        let folder_id = db.add_folder("Tech", None, 0).await.unwrap();
        assert!(folder_id > 0);

        let folders = db.get_folders().await.unwrap();
        assert_eq!(folders.len(), 1);
        assert_eq!(folders[0].name, "Tech");

        db.update_folder(folder_id, "Technology", None, 1).await.unwrap();
        let folders = db.get_folders().await.unwrap();
        assert_eq!(folders[0].name, "Technology");
        assert_eq!(folders[0].sort_order, 1);

        db.delete_folder(folder_id).await.unwrap();
        let folders = db.get_folders().await.unwrap();
        assert!(folders.is_empty());
    }

    #[tokio::test]
    async fn test_tag_operations() {
        let db = create_test_db().await;

        let tag_id = db.add_tag("Important", Some("#ff0000")).await.unwrap();
        assert!(tag_id > 0);

        let feed_id = db
            .add_feed("Test Feed", "https://example.com/feed", None, None, None)
            .await
            .unwrap();

        let article_id = db
            .add_article(feed_id, "Article 1", "https://example.com/1", None, None, None, None)
            .await
            .unwrap();

        db.tag_article(article_id, tag_id).await.unwrap();

        let tags = db.get_article_tags(article_id).await.unwrap();
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].name, "Important");

        db.untag_article(article_id, tag_id).await.unwrap();
        let tags = db.get_article_tags(article_id).await.unwrap();
        assert!(tags.is_empty());
    }
    #[tokio::test]
    async fn test_add_and_get_book() {
        let db = create_test_db().await;

        let metadata = crate::BookMetadata {
            title: "Test Book".to_string(),
            subtitle: None,
            authors: vec!["Test Author".to_string()],
            description: None,
            publisher: None,
            published_date: None,
            language: None,
            isbn_10: None,
            isbn_13: None,
            page_count: None,
            cover_data: None,
            cover_url: None,
        };

        let book_id = db.add_book(&metadata, &None).await.unwrap();
        assert!(book_id > 0);

        let books = db.get_books().await.unwrap();
        assert_eq!(books.len(), 1);
        assert_eq!(books[0].title, "Test Book");

        let authors = db.get_book_authors(book_id).await.unwrap();
        assert_eq!(authors, vec!["Test Author"]);
    }
}
