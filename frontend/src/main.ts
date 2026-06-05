import './styles/theme.css'
import { invoke } from '@tauri-apps/api/core'

async function init() {
  const app = document.getElementById('app')!

  // Boot screen
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

  // Simulate boot
  await new Promise(r => setTimeout(r, 1500))

  // Test Tauri invoke
  try {
    const greeting = await invoke<string>('greet', { name: 'synth' })
    console.log(greeting)
  } catch (e) {
    console.log('Tauri not available:', e)
  }

  // Main app
  app.innerHTML = `
    <div class="app-layout">
      <aside class="sidebar">
        <div class="sidebar-header">
          <h2 class="logo-small">SYNTH</h2>
        </div>
        <nav class="sidebar-nav">
          <div class="nav-section">
            <h3 class="nav-title">Library</h3>
            <a href="#" class="nav-item active">
              <span>📚</span> All Books <span class="count">0</span>
            </a>
            <a href="#" class="nav-item">
              <span>📖</span> Reading <span class="count">0</span>
            </a>
            <a href="#" class="nav-item">
              <span>✅</span> Finished <span class="count">0</span>
            </a>
            <a href="#" class="nav-item">
              <span>🔖</span> Want to Read <span class="count">0</span>
            </a>
          </div>
          <div class="nav-section">
            <h3 class="nav-title">Collections</h3>
            <a href="#" class="nav-item">
              <span>📁</span> Sci-Fi
            </a>
            <a href="#" class="nav-item">
              <span>📁</span> Technical
            </a>
          </div>
        </nav>
      </aside>
      <main class="main-content">
        <header class="top-bar">
          <div class="search-box">
            <input type="text" placeholder="Search your library..." />
          </div>
          <div class="actions">
            <button class="btn-neon">+ Add Books</button>
          </div>
        </header>
        <div class="content-area">
          <div class="empty-state">
            <div class="empty-icon">📚</div>
            <h2>Your library is empty</h2>
            <p>Drop ebook files here or click "Add Books" to get started</p>
            <button class="btn-neon">Add Books</button>
          </div>
        </div>
      </main>
    </div>
  `
}

init()
