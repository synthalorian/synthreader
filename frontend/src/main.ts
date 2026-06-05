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

let books: Book[] = []

function renderApp() {
  const app = document.getElementById('app')!
  app.innerHTML = `
    <div class="app-layout">
      <aside class="sidebar">
        <div class="sidebar-header">
          <h2 class="logo-small">SYNTH</h2>
        </div>
        <nav class="sidebar-nav">
          <div class="nav-section">
            <h3 class="nav-title">Library</h3>
            <a href="#" class="nav-item active" data-filter="all">
              <span>📚</span> All Books <span class="count" id="count-all">0</span>
            </a>
            <a href="#" class="nav-item" data-filter="reading">
              <span>📖</span> Reading <span class="count" id="count-reading">0</span>
            </a>
            <a href="#" class="nav-item" data-filter="finished">
              <span>✅</span> Finished <span class="count" id="count-finished">0</span>
            </a>
            <a href="#" class="nav-item" data-filter="want">
              <span>🔖</span> Want to Read <span class="count" id="count-want">0</span>
            </a>
          </div>
        </nav>
      </aside>
      <main class="main-content">
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
      </main>
    </div>
  `

  setupEventListeners()
  loadBooks()
}

function setupEventListeners() {
  document.getElementById('add-books-btn')?.addEventListener('click', addBooks)
  document.getElementById('empty-add-btn')?.addEventListener('click', addBooks)

  document.getElementById('search-input')?.addEventListener('input', (e) => {
    const query = (e.target as HTMLInputElement).value.toLowerCase()
    filterBooks(query)
  })

  const contentArea = document.getElementById('content-area')!
  contentArea.addEventListener('dragover', (e) => {
    e.preventDefault()
    contentArea.classList.add('drag-over')
  })
  contentArea.addEventListener('dragleave', () => {
    contentArea.classList.remove('drag-over')
  })
  contentArea.addEventListener('drop', (e) => {
    e.preventDefault()
    contentArea.classList.remove('drag-over')
    // TODO: handle file drop
  })
}

async function loadBooks() {
  try {
    const result = await invoke<Book[]>('get_books')
    books = result
    updateCounts()
    renderBookGrid(books)
  } catch (e) {
    console.error('Failed to load books:', e)
  }
}

function updateCounts() {
  document.getElementById('count-all')!.textContent = String(books.length)
  document.getElementById('count-reading')!.textContent = String(books.filter(b => b.progress > 0 && b.progress < 1).length)
  document.getElementById('count-finished')!.textContent = String(books.filter(b => b.progress >= 1).length)
  document.getElementById('count-want')!.textContent = String(books.filter(b => b.progress === 0).length)
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

  // Load covers in parallel
  const covers = await Promise.all(
    booksToRender.map(b => getCoverUrl(b.coverPath))
  )

  grid.innerHTML = booksToRender.map((book, i) => {
    const coverUrl = covers[i]
    return `
    <div class="book-card" data-id="${book.id}">
      <div class="book-cover" onclick="openBook(${book.id})">
        ${coverUrl
          ? `<img src="${coverUrl}" alt="${book.title}" loading="lazy">`
          : `<div class="cover-placeholder">
              <span class="cover-letter">${book.title.charAt(0)}</span>
             </div>`
        }
        ${book.progress > 0 ? `
          <div class="progress-bar">
            <div class="progress-fill" style="width: ${book.progress * 100}%"></div>
          </div>
        ` : ''}
      </div>
      <div class="book-info">
        <h3 class="book-title">${book.title}</h3>
        <p class="book-author">${book.authors.join(', ')}</p>
      </div>
    </div>
  `}).join('')
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
    updateCounts()
    renderBookGrid(books)
  } catch (e) {
    console.error('Failed to add books:', e)
  }
}

function openBook(bookId: number) {
  const app = document.getElementById('app')!
  app.innerHTML = ''

  const reader = document.createElement('epub-renderer') as any
  reader.setChapters([
    { href: 'chapter1', title: 'Chapter 1', content: '<h1>Chapter 1</h1><p>This is a test chapter. The synthwave reader is working.</p>' },
    { href: 'chapter2', title: 'Chapter 2', content: '<h1>Chapter 2</h1><p>Another chapter with <a href="#">a link</a> and some text.</p>' }
  ])

  app.appendChild(reader)
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
      <p class="version">v0.1.0</p>
    </div>
  `

  await new Promise(r => setTimeout(r, 1200))
  renderApp()
}

boot()
