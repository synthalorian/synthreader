import './styles/theme.css'
import { invoke } from '@tauri-apps/api/core'

interface Book {
  id: number
  title: string
  authors: string[]
  coverUrl?: string
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
  updateCounts()
}

function setupEventListeners() {
  document.getElementById('add-books-btn')?.addEventListener('click', addBooks)
  document.getElementById('empty-add-btn')?.addEventListener('click', addBooks)

  document.getElementById('search-input')?.addEventListener('input', (e) => {
    const query = (e.target as HTMLInputElement).value.toLowerCase()
    filterBooks(query)
  })

  // Drag and drop
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

function updateCounts() {
  document.getElementById('count-all')!.textContent = String(books.length)
  document.getElementById('count-reading')!.textContent = String(books.filter(b => b.progress > 0 && b.progress < 1).length)
  document.getElementById('count-finished')!.textContent = String(books.filter(b => b.progress >= 1).length)
  document.getElementById('count-want')!.textContent = String(books.filter(b => b.progress === 0).length)
}

function renderBookGrid(booksToRender: Book[]) {
  const grid = document.getElementById('book-grid')!
  const emptyState = document.getElementById('empty-state')!

  if (booksToRender.length === 0) {
    grid.style.display = 'none'
    emptyState.style.display = 'flex'
    return
  }

  grid.style.display = 'grid'
  emptyState.style.display = 'none'

  grid.innerHTML = booksToRender.map(book => `
    <div class="book-card" data-id="${book.id}">
      <div class="book-cover">
        ${book.coverUrl
          ? `<img src="${book.coverUrl}" alt="${book.title}" loading="lazy">`
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
  `).join('')
}

function filterBooks(query: string) {
  const filtered = books.filter(b =>
    b.title.toLowerCase().includes(query) ||
    b.authors.some(a => a.toLowerCase().includes(query))
  )
  renderBookGrid(filtered)
}

async function addBooks() {
  // TODO: open file dialog via Tauri
  console.log('Add books clicked')
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
