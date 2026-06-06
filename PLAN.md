# Synthreader — Development Plan

Thread reader / RSS aggregator. Rust + Tauri. Cross-platform desktop app.

---

## v0.1.0 — Fetch (Now)

- [x] Define core data model: Article, Feed, Folder, Tag
- [x] Implement RSS/Atom feed parsing (`rss` crate)
- [x] HTTP client for fetching feeds + articles
- [x] SQLite schema + migrations (`sqlx`)
- [x] Store feeds, articles, read state in local DB
- [x] Background feed refresh (async)

## v0.2.0 — Read

- [ ] Tauri frontend: article list view
- [ ] Article reader view (clean, distraction-free)
- [ ] Readability-style content extraction
- [ ] Mark read/unread, star articles
- [ ] Full-text search (SQLite FTS or `tantivy`)
- [ ] Keyboard shortcuts (j/k navigation, o open)

## v0.3.0 — Organize

- [x] Folders for feed organization
- [x] Tags for articles
- [x] OPML import/export
- [x] Feed discovery (auto-detect from URL)
- [x] Article filtering (unread only, starred, by tag)
- [x] Custom themes (light/dark/synthwave)

## v0.4.0 — Polish (Complete)

- [x] Feed health monitoring (track fetch errors, last success/failure)
- [x] Feed management (delete feed, edit feed properties, feed stats)
- [x] Article cleanup (bulk delete old articles, duplicate detection)
- [x] Code quality improvements and comprehensive tests
- [x] Error handling: structured feed fetch error tracking

## v0.5.0 — Search & Discovery

- [x] Full-text search across articles (SQLite FTS5)
- [x] Search highlighting in article content
- [x] Feed recommendation engine — suggest feeds based on reading history
- [x] Article similarity detection — "related articles" feature
- [x] Global keyboard shortcuts (j/k navigation, o open, s star, r refresh)
- [x] Quick filter bar — search feeds and articles in real-time
- [x] Import from Pocket/Instapaper OPML exports

## v0.6.0 — Frontend & Polish (Complete)

- [x] Tauri frontend: article list view with virtual scrolling
- [x] Article reader view — clean, distraction-free reading
- [x] Readability-style content extraction from article HTML
- [x] Custom themes: light, dark, synthwave (CSS variables)
- [x] Mark read/unread, star articles from UI
- [x] Offline mode — read cached articles without internet
- [x] Auto-refresh feeds on startup with progress indicator
- [x] Performance: smooth scrolling with 10k+ articles

## v0.7.0 — Pre-Release Polish (Complete)

- [x] Cross-platform builds — Linux, Windows, macOS via Tauri
- [x] Auto-updater — Tauri updater with signature verification
- [x] Keyboard shortcuts config — user-customizable keybindings
- [x] Article sharing — export to Markdown, PDF, email
- [x] Notification system — desktop notifications for new articles
- [x] Import from more sources — Feedly, Inoreader, NewsBlur
- [x] Data export — full database backup/restore
- [x] Onboarding flow — first-run tutorial for new users

## v1.0.0 — Ship It

- [ ] All tests pass, CI green
- [ ] Cross-platform builds: Linux, Windows, macOS
- [ ] Auto-updater (Tauri updater)
- [ ] Performance: 10k+ articles, smooth scrolling
- [ ] Documentation + onboarding
- [ ] App store distribution

---

## Architecture

```
Tauri Frontend (React/Vue/Svelte)
    ↓ IPC
synthreader-core (Rust library)
    ↓
SQLite (local storage)
    ↓
HTTP client (feed fetching)
```

## Key Files

| File | Responsibility |
|------|---------------|
| `src-tauri/src/main.rs` | Tauri app entry |
| `src-tauri/tauri.conf.json` | App config |
| `synthreader-core/src/lib.rs` | Core library |
| `synthreader-core/src/feed.rs` | Feed parsing |
| `synthreader-core/src/article.rs` | Article model |
| `synthreader-core/src/db.rs` | SQLite operations |
| `synthreader-core/src/fetch.rs` | HTTP fetching |
| `synthreader-core/src/formats/` | RSS/Atom/JSON Feed parsers |

## Local Dev

```bash
# Install Tauri CLI:
cargo install tauri-cli

# Run dev server:
cargo tauri dev

# Build:
cargo tauri build
```

## Testing

```bash
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo tauri build
```

## Data Model

```rust
struct Feed {
    id: i64,
    title: String,
    url: String,
    site_url: Option<String>,
    folder_id: Option<i64>,
    last_fetched: Option<DateTime<Utc>>,
}

struct Article {
    id: i64,
    feed_id: i64,
    title: String,
    url: String,
    content: String,
    published_at: Option<DateTime<Utc>>,
    read: bool,
    starred: bool,
}
```

---

*Read the grid.* 🧵
