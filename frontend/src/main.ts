import './styles/theme.css'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import './reader/EpubRenderer'

interface Book {
  id: number
  title: string
  authors: string[]
  coverPath?: string
  progress: number
}

interface Feed {
  id: number
  title: string
  url: string
  site_url?: string
}

interface Article {
  id: number
  feed_id: number
  title: string
  url: string
  content?: string
  summary?: string
  author?: string
  published_at?: string
  read: boolean
  starred: boolean
  created_at: string
}

interface ExtractedContent {
  title?: string
  content: string
  text_content: string
  author?: string
  excerpt?: string
  site_name?: string
  lang?: string
  published_time?: string
}

type View = 'books' | 'articles' | 'reader' | 'feeds'
type ArticleFilter = 'all' | 'unread' | 'starred'

let books: Book[] = []
let feeds: Feed[] = []
let articles: Article[] = []
let currentView: View = 'articles'
let articleFilter: ArticleFilter = 'all'
let selectedArticleIndex = -1
let searchQuery = ''
let currentArticle: Article | null = null
let extractedContent: ExtractedContent | null = null
let feedArticles: Map<number, Article[]> = new Map()
let expandedFeeds: Set<number> = new Set()

function renderApp() {
  const app = document.getElementById('app')!
  app.innerHTML = `
    <div class="app-layout">
      <aside class="sidebar">
        <div class="sidebar-header">
          <h2 class="logo-small">SYNTHREADER</h2>
        </div>
        <nav class="sidebar-nav">
          <div class="nav-section">
            <h3 class="nav-title">Feeds</h3>
            <a href="#" class="nav-item ${currentView === 'articles' ? 'active' : ''}" data-view="articles">
              <span>✍️</span> Articles <span class="count" id="count-articles">0</span>
            </a>
            <a href="#" class="nav-item ${currentView === 'feeds' ? 'active' : ''}" data-view="feeds">
              <span>📡</span> Feeds <span class="count" id="count-feeds">0</span>
            </a>
          </div>
          <div class="nav-section">
            <h3 class="nav-title">Library</h3>
            <a href="#" class="nav-item ${currentView === 'books' ? 'active' : ''}" data-view="books">
              <span>📚</span> All Books <span class="count" id="count-all">0</span>
            </a>
          </div>
        </nav>
      </aside>
      <main class="main-content" id="main-content">
        ${renderMainContent()}
      </main>
    </div>
    <div class="keyboard-hint" id="keyboard-hint">
      <kbd>j</kbd> down <kbd>k</kbd> up <kbd>o</kbd> open <kbd>r</kbd> read <kbd>s</kbd> star <kbd>/</kbd> search <kbd>esc</kbd> close
    </div>
  `

  setupEventListeners()
  loadData()
}

function renderMainContent(): string {
  if (currentView === 'books') {
    return renderBooksView()
  } else if (currentView === 'articles') {
    return renderArticlesView()
  } else if (currentView === 'feeds') {
    return renderFeedsView()
  } else if (currentView === 'reader') {
    return ''  // Reader renders directly into main-content
  }
  return ''
}

function renderBooksView(): string {
  return `
    <header class="top-bar">
      <div class="search-box">
        <input type="text" id="search-input" placeholder="Search your library..." />
      </div>
      <div class="actions">
        <button class="btn-neon" id="add-books-btn">+ Add Books</button>
      </div>
    </header>
    <div class="content-area" id="content-area">
      <div class="empty-state" id="empty-state">
        <div class="empty-icon">📚</div>
        <h2>Your library is empty</h2>
        <p>Drop ebook files here or click "Add Books" to get started</p>
        <button class="btn-neon" id="empty-add-btn">Add Books</button>
      </div>
      <div class="book-grid" id="book-grid"></div>
    </div>
  `
}

function renderArticlesView(): string {
  return `
    <header class="top-bar">
      <div class="search-box">
        <input type="text" id="search-input" placeholder="Search articles... (press / to focus)" value="${escapeHtml(searchQuery)}" />
      </div>
      <div class="actions">
        <button class="btn-neon" id="add-feed-btn">+ Add Feed</button>
        <button class="btn-neon" id="refresh-feeds-btn" style="margin-left: 8px">↻ Refresh</button>
      </div>
    </header>
    <div class="content-area" id="content-area">
      <div class="feed-form" id="feed-form" style="display: none;">
        <input type="text" id="feed-url-input" placeholder="Enter website or feed URL..." />
        <button class="btn-neon" id="submit-feed-btn">Add</button>
        <button class="reader-btn" id="cancel-feed-btn">Cancel</button>
      </div>
      <div class="filter-tabs" id="filter-tabs">
        <button class="filter-tab ${articleFilter === 'all' ? 'active' : ''}" data-filter="all">All</button>
        <button class="filter-tab ${articleFilter === 'unread' ? 'active' : ''}" data-filter="unread">Unread</button>
        <button class="filter-tab ${articleFilter === 'starred' ? 'active' : ''}" data-filter="starred">Starred</button>
      </div>
      <div class="article-list" id="article-list"></div>
      <div class="empty-state" id="empty-state" style="display: none;">
        <div class="empty-icon">✍️</div>
        <h2>No articles yet</h2>
        <p>Add feeds to start collecting articles</p>
      </div>
    </div>
  `
}

function renderFeedsView(): string {
  return `
    <header class="top-bar">
      <div class="search-box">
        <input type="text" id="search-input" placeholder="Search feeds..." />
      </div>
      <div class="actions">
        <button class="btn-neon" id="add-feed-btn">+ Add Feed</button>
        <button class="btn-neon" id="refresh-feeds-btn" style="margin-left: 8px">↻ Refresh</button>
      </div>
    </header>
    <div class="content-area" id="content-area">
      <div class="feed-form" id="feed-form" style="display: none;">
        <input type="text" id="feed-url-input" placeholder="Enter website or feed URL..." />
        <button class="btn-neon" id="submit-feed-btn">Add</button>
        <button class="reader-btn" id="cancel-feed-btn">Cancel</button>
      </div>
      <div class="feed-list" id="feed-list"></div>
      <div class="empty-state" id="empty-state" style="display: none;">
        <div class="empty-icon">📡</div>
        <h2>No feeds yet</h2>
        <p>Add RSS feeds to start collecting articles</p>
      </div>
    </div>
  `
}

function renderReaderView(): string {
  if (!currentArticle || !extractedContent) return ''

  const meta = []
  if (extractedContent.site_name || currentArticle.author) {
    meta.push(`<span>${escapeHtml(extractedContent.site_name || currentArticle.author || '')}</span>`)
  }
  if (currentArticle.published_at) {
    meta.push(`<span>${formatDate(currentArticle.published_at)}</span>`)
  }
  meta.push(`<a href="${escapeHtml(currentArticle.url)}" target="_blank" rel="noopener">Original ↗</a>`)

  return `
    <div class="reader-overlay" id="reader-overlay">
      <header class="reader-header">
        <span class="reader-title">${escapeHtml(currentArticle.title)}</span>
        <div class="reader-actions">
          <button class="reader-btn" id="reader-star-btn">${currentArticle.starred ? '★' : '☆'} Star</button>
          <button class="reader-btn" id="reader-mark-read-btn">${currentArticle.read ? 'Mark Unread' : 'Mark Read'}</button>
          <button class="reader-btn" id="reader-close-btn">Close ×</button>
        </div>
      </header>
      <div class="reader-content">
        <article class="reader-article">
          <div class="reader-meta-bar">
            ${meta.join(' <span>|</span> ')}
          </div>
          <h1>${escapeHtml(extractedContent.title || currentArticle.title)}</h1>
          ${extractedContent.content}
        </article>
      </div>
    </div>
  `
}

function renderArticleList() {
  const list = document.getElementById('article-list')
  const emptyState = document.getElementById('empty-state')
  if (!list || !emptyState) return

  let filtered = articles

  if (articleFilter === 'unread') {
    filtered = articles.filter(a => !a.read)
  } else if (articleFilter === 'starred') {
    filtered = articles.filter(a => a.starred)
  }

  if (searchQuery.trim()) {
    const q = searchQuery.toLowerCase()
    filtered = filtered.filter(a =>
      a.title.toLowerCase().includes(q) ||
      (a.summary && a.summary.toLowerCase().includes(q)) ||
      (a.author && a.author.toLowerCase().includes(q))
    )
  }

  if (filtered.length === 0) {
    list.style.display = 'none'
    emptyState.style.display = 'flex'
    return
  }

  list.style.display = 'flex'
  emptyState.style.display = 'none'

  const feedMap = new Map(feeds.map(f => [f.id, f]))

  list.innerHTML = filtered.map((article, index) => {
    const feed = feedMap.get(article.feed_id)
    const isSelected = index === selectedArticleIndex
    return `
      <div class="article-item ${article.read ? 'read' : ''} ${isSelected ? 'selected' : ''}" data-id="${article.id}" data-index="${index}">
        <span class="article-star ${article.starred ? 'starred' : ''}" data-id="${article.id}" data-action="star">
          ${article.starred ? '★' : '☆'}
        </span>
        <div class="article-content" data-id="${article.id}" data-action="open">
          <div class="article-title">${escapeHtml(article.title)}</div>
          <div class="article-meta">
            <span class="feed-name">${escapeHtml(feed?.title || 'Unknown Feed')}</span>
            ${article.author ? `<span class="author">by ${escapeHtml(article.author)}</span>` : ''}
            <span>${formatDate(article.published_at || article.created_at)}</span>
          </div>
          ${article.summary ? `<div class="article-summary">${escapeHtml(article.summary)}</div>` : ''}
        </div>
        <div class="article-actions">
          <button class="article-read-btn" data-id="${article.id}" data-action="toggle-read">
            ${article.read ? 'Unread' : 'Read'}
          </button>
        </div>
      </div>
    `
  }).join('')

  document.getElementById('count-articles')!.textContent = String(articles.length)
}

async function loadData() {
  // Always load feeds and articles for counts
  await Promise.all([loadFeeds(), loadArticles()])
  if (currentView === 'books') {
    await loadBooks()
  }
}

async function loadFeeds() {
  try {
    feeds = await invoke<Feed[]>('get_feeds')
    document.getElementById('count-feeds')!.textContent = String(feeds.length)
  } catch (e) {
    console.error('Failed to load feeds:', e)
  }
}

async function loadArticles() {
  try {
    articles = await invoke<Article[]>('get_all_articles', { limit: 500 })
    selectedArticleIndex = articles.length > 0 ? 0 : -1
    renderArticleList()
  } catch (e) {
    console.error('Failed to load articles:', e)
  }
}

async function searchArticles(query: string) {
  try {
    if (!query.trim()) {
      await loadArticles()
      return
    }
    articles = await invoke<Article[]>('search_articles', { query, limit: 100 })
    selectedArticleIndex = articles.length > 0 ? 0 : -1
    renderArticleList()
  } catch (e) {
    console.error('Search failed:', e)
  }
}

function setupEventListeners() {
  document.querySelectorAll('.nav-item').forEach(el => {
    el.addEventListener('click', (e) => {
      e.preventDefault()
      const view = (e.currentTarget as HTMLElement).dataset.view as View
      if (view) {
        currentView = view
        renderApp()
      }
    })
  })

  if (currentView === 'books') {
    document.getElementById('add-books-btn')?.addEventListener('click', addBooks)
    document.getElementById('empty-add-btn')?.addEventListener('click', addBooks)
    document.getElementById('search-input')?.addEventListener('input', (e) => {
      const query = (e.target as HTMLInputElement).value.toLowerCase()
      filterBooks(query)
    })
  } else if (currentView === 'articles') {
    setupArticleListeners()
  } else if (currentView === 'feeds') {
    setupFeedsListeners()
  }

  document.addEventListener('keydown', handleKeyboard)
}

function setupArticleListeners() {
  const searchInput = document.getElementById('search-input') as HTMLInputElement
  searchInput?.addEventListener('input', (e) => {
    searchQuery = (e.target as HTMLInputElement).value
    if (searchQuery.trim()) {
      searchArticles(searchQuery)
    } else {
      loadArticles()
    }
  })

  document.getElementById('add-feed-btn')?.addEventListener('click', () => {
    const form = document.getElementById('feed-form')!
    form.style.display = form.style.display === 'none' ? 'flex' : 'none'
  })

  document.getElementById('cancel-feed-btn')?.addEventListener('click', () => {
    document.getElementById('feed-form')!.style.display = 'none'
  })

  document.getElementById('submit-feed-btn')?.addEventListener('click', addFeed)
  document.getElementById('feed-url-input')?.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') addFeed()
  })

  document.getElementById('refresh-feeds-btn')?.addEventListener('click', refreshFeeds)

  document.querySelectorAll('.filter-tab').forEach(el => {
    el.addEventListener('click', (e) => {
      articleFilter = (e.currentTarget as HTMLElement).dataset.filter as ArticleFilter
      selectedArticleIndex = -1
      renderApp()
    })
  })

  document.getElementById('article-list')?.addEventListener('click', (e) => {
    const target = e.target as HTMLElement
    const item = target.closest('.article-item') as HTMLElement
    if (!item) return

    const id = Number(item.dataset.id)
    const action = target.closest('[data-action]')?.getAttribute('data-action')

    if (action === 'star' || target.classList.contains('article-star')) {
      toggleStar(id)
    } else if (action === 'toggle-read') {
      toggleRead(id)
    } else if (action === 'open' || target.closest('.article-content')) {
      openArticle(id)
    }
  })
}

function setupFeedsListeners() {
  const searchInput = document.getElementById('search-input') as HTMLInputElement
  searchInput?.addEventListener('input', (e) => {
    const query = (e.target as HTMLInputElement).value.toLowerCase()
    filterFeeds(query)
  })

  document.getElementById('add-feed-btn')?.addEventListener('click', () => {
    const form = document.getElementById('feed-form')!
    form.style.display = form.style.display === 'none' ? 'flex' : 'none'
  })

  document.getElementById('cancel-feed-btn')?.addEventListener('click', () => {
    document.getElementById('feed-form')!.style.display = 'none'
  })

  document.getElementById('submit-feed-btn')?.addEventListener('click', addFeed)
  document.getElementById('feed-url-input')?.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') addFeed()
  })

  document.getElementById('refresh-feeds-btn')?.addEventListener('click', refreshFeeds)

  // Feed expand/collapse and article clicks
  document.getElementById('feed-list')?.addEventListener('click', (e) => {
    const target = e.target as HTMLElement
    const feedHeader = target.closest('.feed-header') as HTMLElement
    if (feedHeader) {
      const feedId = Number(feedHeader.dataset.id)
      if (expandedFeeds.has(feedId)) {
        expandedFeeds.delete(feedId)
      } else {
        expandedFeeds.add(feedId)
        loadArticlesForFeed(feedId)
      }
      renderFeedList()
      return
    }

    const articleEl = target.closest('.feed-article-item') as HTMLElement
    if (articleEl) {
      const articleId = Number(articleEl.dataset.id)
      openArticle(articleId)
      return
    }

    const starEl = target.closest('.feed-article-star') as HTMLElement
    if (starEl) {
      e.stopPropagation()
      const articleId = Number(starEl.dataset.id)
      toggleStar(articleId)
      return
    }

    const readEl = target.closest('.feed-article-read-btn') as HTMLElement
    if (readEl) {
      e.stopPropagation()
      const articleId = Number(readEl.dataset.id)
      toggleRead(articleId)
      return
    }
  })

  renderFeedList()
}

function filterFeeds(query: string) {
  const filtered = feeds.filter(f =>
    f.title.toLowerCase().includes(query) ||
    f.url.toLowerCase().includes(query)
  )
  renderFeedList(filtered)
}

async function loadArticlesForFeed(feedId: number) {
  try {
    const feedArticlesList = await invoke<Article[]>('get_articles', { feedId })
    feedArticles.set(feedId, feedArticlesList)
    renderFeedList()
  } catch (e) {
    console.error('Failed to load articles for feed:', e)
  }
}

function renderFeedList(feedList?: Feed[]) {
  const list = document.getElementById('feed-list')
  const emptyState = document.getElementById('empty-state')
  if (!list) return

  const displayFeeds = feedList || feeds

  if (displayFeeds.length === 0) {
    list.innerHTML = ''
    emptyState!.style.display = feeds.length === 0 ? 'flex' : 'none'
    return
  }

  emptyState!.style.display = 'none'
  list.innerHTML = displayFeeds.map(feed => {
    const isExpanded = expandedFeeds.has(feed.id)
    const articlesForFeed = feedArticles.get(feed.id) || []
    const unreadCount = articlesForFeed.filter(a => !a.read).length

    return `
      <div class="feed-group ${isExpanded ? 'expanded' : ''}" data-id="${feed.id}">
        <div class="feed-header" data-id="${feed.id}">
          <span class="feed-expand-icon">${isExpanded ? '▼' : '▶'}</span>
          <div class="feed-info">
            <div class="feed-title">${escapeHtml(feed.title)}</div>
            <div class="feed-url">${escapeHtml(feed.url)}</div>
          </div>
          <span class="feed-count">${articlesForFeed.length > 0 ? `${articlesForFeed.length} articles${unreadCount > 0 ? ` (${unreadCount} unread)` : ''}` : ''}</span>
        </div>
        ${isExpanded ? renderFeedArticles(articlesForFeed) : ''}
      </div>
    `
  }).join('')
}

function renderFeedArticles(articlesForFeed: Article[]): string {
  if (articlesForFeed.length === 0) {
    return `
      <div class="feed-articles-list">
        <div class="feed-articles-empty">No articles in this feed yet. Click ↻ Refresh to fetch.</div>
      </div>
    `
  }

  return `
    <div class="feed-articles-list">
      ${articlesForFeed.map(article => `
        <div class="feed-article-item ${article.read ? 'read' : ''}" data-id="${article.id}">
          <span class="feed-article-star ${article.starred ? 'starred' : ''}" data-id="${article.id}">
            ${article.starred ? '★' : '☆'}
          </span>
          <div class="feed-article-content">
            <div class="feed-article-title">${escapeHtml(article.title)}</div>
            <div class="feed-article-meta">
              ${article.author ? `<span class="author">by ${escapeHtml(article.author)}</span>` : ''}
              <span>${formatDate(article.published_at || article.created_at)}</span>
            </div>
            ${article.summary ? `<div class="feed-article-summary">${escapeHtml(article.summary)}</div>` : ''}
          </div>
          <button class="feed-article-read-btn" data-id="${article.id}">
            ${article.read ? 'Unread' : 'Read'}
          </button>
        </div>
      `).join('')}
    </div>
  `
}

function handleKeyboard(e: KeyboardEvent) {
  const searchInput = document.getElementById('search-input') as HTMLInputElement
  const isSearchFocused = document.activeElement === searchInput
  const readerOverlay = document.getElementById('reader-overlay')

  if (e.key === '/' && !isSearchFocused && !readerOverlay) {
    e.preventDefault()
    searchInput?.focus()
    return
  }

  if (e.key === 'Escape') {
    if (readerOverlay) {
      closeReader()
    } else if (isSearchFocused) {
      searchInput?.blur()
      searchQuery = ''
      searchInput.value = ''
      loadArticles()
    }
    return
  }

  if (readerOverlay) {
    if (e.key === 'o' || e.key === 'O') {
      closeReader()
    }
    return
  }

  if (isSearchFocused) return

  const filteredCount = getFilteredArticles().length

  switch (e.key) {
    case 'j':
    case 'J':
      e.preventDefault()
      if (selectedArticleIndex < filteredCount - 1) {
        selectedArticleIndex++
        renderArticleList()
        scrollToSelected()
      }
      break
    case 'k':
    case 'K':
      e.preventDefault()
      if (selectedArticleIndex > 0) {
        selectedArticleIndex--
        renderArticleList()
        scrollToSelected()
      }
      break
    case 'o':
    case 'O':
      e.preventDefault()
      if (selectedArticleIndex >= 0) {
        const filtered = getFilteredArticles()
        if (filtered[selectedArticleIndex]) {
          openArticle(filtered[selectedArticleIndex].id)
        }
      }
      break
    case 'r':
    case 'R':
      e.preventDefault()
      if (selectedArticleIndex >= 0) {
        const filtered = getFilteredArticles()
        if (filtered[selectedArticleIndex]) {
          toggleRead(filtered[selectedArticleIndex].id)
        }
      }
      break
    case 's':
    case 'S':
      e.preventDefault()
      if (selectedArticleIndex >= 0) {
        const filtered = getFilteredArticles()
        if (filtered[selectedArticleIndex]) {
          toggleStar(filtered[selectedArticleIndex].id)
        }
      }
      break
  }
}

function getFilteredArticles(): Article[] {
  let filtered = articles
  if (articleFilter === 'unread') {
    filtered = articles.filter(a => !a.read)
  } else if (articleFilter === 'starred') {
    filtered = articles.filter(a => a.starred)
  }
  if (searchQuery.trim()) {
    const q = searchQuery.toLowerCase()
    filtered = filtered.filter(a =>
      a.title.toLowerCase().includes(q) ||
      (a.summary && a.summary.toLowerCase().includes(q)) ||
      (a.author && a.author.toLowerCase().includes(q))
    )
  }
  return filtered
}

function scrollToSelected() {
  const selected = document.querySelector('.article-item.selected')
  selected?.scrollIntoView({ block: 'nearest', behavior: 'smooth' })
}

async function openArticle(id: number) {
  try {
    const article = await invoke<Article>('get_article', { articleId: id })
    if (!article) return

    currentArticle = article

    if (article.content) {
      extractedContent = {
        title: article.title,
        content: article.content,
        text_content: '',
        author: article.author,
      }
      showReader()
    } else {
      const extracted = await invoke<ExtractedContent>('extract_article_content', {
        articleId: id,
        url: article.url,
      })
      extractedContent = extracted
      showReader()
    }

    if (!article.read) {
      await toggleRead(id, false)
    }
  } catch (e) {
    console.error('Failed to open article:', e)
    alert('Failed to open article. Make sure the URL is accessible.')
  }
}

function showReader() {
  const main = document.getElementById('main-content')!
  const readerHtml = renderReaderView()
  if (readerHtml) {
    main.insertAdjacentHTML('beforeend', readerHtml)
    setupReaderListeners()
  }
}

function setupReaderListeners() {
  document.getElementById('reader-close-btn')?.addEventListener('click', closeReader)
  document.getElementById('reader-star-btn')?.addEventListener('click', () => {
    if (currentArticle) toggleStar(currentArticle.id)
  })
  document.getElementById('reader-mark-read-btn')?.addEventListener('click', () => {
    if (currentArticle) toggleRead(currentArticle.id)
  })
}

function closeReader() {
  document.getElementById('reader-overlay')?.remove()
  currentArticle = null
  extractedContent = null
}

async function toggleRead(id: number, shouldRefresh = true) {
  try {
    const article = articles.find(a => a.id === id)
    if (!article) return

    const newRead = !article.read
    await invoke('mark_article_read', { articleId: id, read: newRead })
    article.read = newRead

    if (shouldRefresh) renderArticleList()
  } catch (e) {
    console.error('Failed to toggle read:', e)
  }
}

async function toggleStar(id: number) {
  try {
    const article = articles.find(a => a.id === id)
    if (!article) return

    const newStarred = !article.starred
    await invoke('star_article', { articleId: id, starred: newStarred })
    article.starred = newStarred
    renderArticleList()
  } catch (e) {
    console.error('Failed to toggle star:', e)
  }
}

async function addFeed() {
  const input = document.getElementById('feed-url-input') as HTMLInputElement
  const url = input.value.trim()
  if (!url) return

  // Show loading state
  const submitBtn = document.getElementById('submit-feed-btn') as HTMLButtonElement
  const originalText = submitBtn?.textContent || 'Add'
  if (submitBtn) submitBtn.textContent = 'Discovering...'

  try {
    // Try to discover feeds from the URL
    const discovered = await invoke<Array<{ title: string; url: string; feed_type: string }>>('discover_feeds', { url })

    if (discovered.length === 0) {
      alert('No feeds found at that URL. Try entering the direct RSS/Atom feed URL.')
      if (submitBtn) submitBtn.textContent = originalText
      return
    }

    if (discovered.length === 1) {
      // Auto-add single discovered feed
      const feed = discovered[0]
      await invoke('add_feed', {
        title: feed.title,
        url: feed.url,
        siteUrl: url,
        description: null,
        folderId: null,
      })
      input.value = ''
      document.getElementById('feed-form')!.style.display = 'none'
      await loadFeeds()
      await refreshFeeds()
    } else {
      // Show feed picker
      showFeedPicker(discovered, url)
    }
  } catch (e) {
    console.error('Failed to discover/add feed:', e)
    // Fallback: try adding the URL directly as a feed
    try {
      await invoke('add_feed', {
        title: url,
        url,
        siteUrl: null,
        description: null,
        folderId: null,
      })
      input.value = ''
      document.getElementById('feed-form')!.style.display = 'none'
      await loadFeeds()
      await refreshFeeds()
    } catch (fallbackErr) {
      alert('Failed to add feed. Please check the URL and try again.')
    }
  } finally {
    if (submitBtn) submitBtn.textContent = originalText
  }
}

function showFeedPicker(discovered: Array<{ title: string; url: string; feed_type: string }>, siteUrl: string) {
  const form = document.getElementById('feed-form')!
  form.innerHTML = `
    <div class="feed-picker" style="display: flex; flex-direction: column; gap: 8px; width: 100%;">
      <div style="font-family: var(--font-display); font-size: 12px; color: var(--color-neon-purple); text-transform: uppercase; letter-spacing: 1px;">Multiple feeds found — select one:</div>
      <div class="feed-options" style="display: flex; flex-direction: column; gap: 4px; max-height: 200px; overflow-y: auto;">
        ${discovered.map((feed, i) => `
          <button class="feed-option-btn" data-index="${i}" style="text-align: left; padding: 10px 12px; background: var(--color-bg-tertiary); border: 1px solid var(--color-border); color: var(--color-text-primary); cursor: pointer; transition: all 150ms; border-radius: var(--radius-sm);">
            <div style="font-weight: 700; font-size: 13px;">${escapeHtml(feed.title)}</div>
            <div style="font-size: 11px; color: var(--color-text-muted); font-family: var(--font-mono);">${escapeHtml(feed.feed_type)} — ${escapeHtml(feed.url)}</div>
          </button>
        `).join('')}
      </div>
      <div style="display: flex; gap: 8px; margin-top: 4px;">
        <button class="reader-btn" id="cancel-feed-picker">Cancel</button>
      </div>
    </div>
  `

  form.querySelectorAll('.feed-option-btn').forEach(btn => {
    btn.addEventListener('click', async (e) => {
      const index = parseInt((e.currentTarget as HTMLElement).dataset.index!)
      const feed = discovered[index]
      try {
        await invoke('add_feed', {
          title: feed.title,
          url: feed.url,
          siteUrl: siteUrl,
          description: null,
          folderId: null,
        })
        // Restore form
        restoreFeedForm()
        await loadFeeds()
        await refreshFeeds()
      } catch (err) {
        alert('Failed to add feed: ' + err)
      }
    })
  })

  document.getElementById('cancel-feed-picker')?.addEventListener('click', () => {
    restoreFeedForm()
  })
}

function restoreFeedForm() {
  const form = document.getElementById('feed-form')!
  form.innerHTML = `
    <input type="text" id="feed-url-input" placeholder="Enter website or feed URL..." />
    <button class="btn-neon" id="submit-feed-btn">Add</button>
    <button class="reader-btn" id="cancel-feed-btn">Cancel</button>
  `
  form.style.display = 'none'

  // Re-attach listeners
  document.getElementById('submit-feed-btn')?.addEventListener('click', addFeed)
  document.getElementById('feed-url-input')?.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') addFeed()
  })
  document.getElementById('cancel-feed-btn')?.addEventListener('click', () => {
    document.getElementById('feed-form')!.style.display = 'none'
  })
}

async function refreshFeeds() {
  const btn = document.getElementById('refresh-feeds-btn') as HTMLButtonElement
  if (btn) btn.textContent = 'Refreshing...'

  try {
    await invoke('refresh_feeds')
    await loadArticles()
  } catch (e) {
    console.error('Failed to refresh feeds:', e)
  } finally {
    if (btn) btn.textContent = '↻ Refresh'
  }
}

async function loadBooks() {
  try {
    const result = await invoke<Book[]>('get_books')
    books = result
    updateBookCounts()
    renderBookGrid(books)
  } catch (e) {
    console.error('Failed to load books:', e)
  }
}

function updateBookCounts() {
  document.getElementById('count-all')!.textContent = String(books.length)
}

async function getCoverUrl(coverPath?: string): Promise<string | undefined> {
  if (!coverPath) return undefined
  try {
    const data = await invoke<number[]>('get_book_cover', { coverPath })
    const bytes = new Uint8Array(data)
    const blob = new Blob([bytes], { type: 'image/png' })
    return URL.createObjectURL(blob)
  } catch (e) {
    return undefined
  }
}

async function renderBookGrid(booksToRender: Book[]) {
  const grid = document.getElementById('book-grid')!
  const emptyState = document.getElementById('empty-state')!

  if (booksToRender.length === 0) {
    grid.style.display = 'none'
    emptyState.style.display = 'flex'
    return
  }

  grid.style.display = 'grid'
  emptyState.style.display = 'none'

  const covers = await Promise.all(booksToRender.map(b => getCoverUrl(b.coverPath)))

  grid.innerHTML = booksToRender.map((book, i) => {
    const coverUrl = covers[i]
    return `
    <div class="book-card" data-id="${book.id}">
      <div class="book-cover" data-book-id="${book.id}">
        ${coverUrl
          ? `<img src="${coverUrl}" alt="${escapeHtml(book.title)}" loading="lazy">`
          : `<div class="cover-placeholder">
              <span class="cover-letter">${book.title.charAt(0)}</span>
             </div>`
        }
        ${book.progress > 0 ? `
          <div class="progress-bar">
            <div class="progress-fill" style="width: ${book.progress * 100}%"></div>
          </div>
        ` : ''}
        <button class="book-delete-btn" data-book-id="${book.id}" title="Delete book">×</button>
      </div>
      <div class="book-info">
        <h3 class="book-title">${escapeHtml(book.title)}</h3>
        <p class="book-author">${escapeHtml(book.authors.join(', '))}</p>
      </div>
    </div>
  `}).join('')

  // Attach click handlers after rendering
  grid.querySelectorAll('.book-cover').forEach(el => {
    el.addEventListener('click', (e) => {
      if ((e.target as HTMLElement).classList.contains('book-delete-btn')) return
      const bookId = Number((el as HTMLElement).dataset.bookId)
      openBook(bookId)
    })
  })

  grid.querySelectorAll('.book-delete-btn').forEach(el => {
    el.addEventListener('click', (e) => {
      e.stopPropagation()
      const bookId = Number((el as HTMLElement).dataset.bookId)
      deleteBook(bookId)
    })
  })
}

function filterBooks(query: string) {
  const filtered = books.filter(b =>
    b.title.toLowerCase().includes(query) ||
    b.authors.some(a => a.toLowerCase().includes(query))
  )
  renderBookGrid(filtered)
}

async function addBooks() {
  try {
    const selected = await open({
      multiple: true,
      filters: [{
        name: 'Ebooks',
        extensions: ['epub', 'mobi', 'azw3', 'pdf', 'txt']
      }]
    })

    if (!selected || (Array.isArray(selected) && selected.length === 0)) return

    const paths = Array.isArray(selected) ? selected : [selected]
    const added = await invoke<Book[]>('add_books', { paths })

    books = [...books, ...added]
    updateBookCounts()
    renderBookGrid(books)
  } catch (e) {
    console.error('Failed to add books:', e)
  }
}

async function openBook(bookId: number) {
  currentView = 'reader'
  const main = document.getElementById('main-content')!
  main.innerHTML = ''

  const reader = document.createElement('epub-renderer') as any
  reader.style.cssText = 'display: flex; flex-direction: column; height: 100%;'
  await reader.loadEpub(bookId)

  main.appendChild(reader)
}

async function deleteBook(bookId: number) {
  if (!confirm('Delete this book from your library?')) return
  try {
    await invoke('delete_book', { bookId })
    books = books.filter(b => b.id !== bookId)
    updateBookCounts()
    renderBookGrid(books)
  } catch (e) {
    console.error('Failed to delete book:', e)
    alert('Failed to delete book')
  }
}

function escapeHtml(text: string): string {
  const div = document.createElement('div')
  div.textContent = text
  return div.innerHTML
}

function formatDate(dateStr: string): string {
  const date = new Date(dateStr)
  const now = new Date()
  const diff = now.getTime() - date.getTime()
  const days = Math.floor(diff / (1000 * 60 * 60 * 24))

  if (days === 0) {
    const hours = Math.floor(diff / (1000 * 60 * 60))
    if (hours === 0) {
      const mins = Math.floor(diff / (1000 * 60))
      return mins <= 1 ? 'just now' : `${mins}m ago`
    }
    return `${hours}h ago`
  } else if (days === 1) {
    return 'yesterday'
  } else if (days < 7) {
    return `${days}d ago`
  } else {
    return date.toLocaleDateString('en-US', { month: 'short', day: 'numeric' })
  }
}

// Boot sequence
async function boot() {
  const app = document.getElementById('app')!
  app.innerHTML = `
    <div class="boot-screen">
      <h1 class="logo">SYNTHREADER</h1>
      <p class="tagline">Synthwave '84 Ebook Reader</p>
      <div class="loading-bar">
        <div class="loading-fill"></div>
      </div>
      <p class="version">v0.2.0</p>
    </div>
  `

  await new Promise(r => setTimeout(r, 1200))
  renderApp()
}

boot()
