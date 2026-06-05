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

- [ ] Folders for feed organization
- [ ] Tags for articles
- [ ] OPML import/export
- [ ] Feed discovery (auto-detect from URL)
- [ ] Article filtering (unread only, starred, by tag)
- [ ] Custom themes (light/dark/synthwave)

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
