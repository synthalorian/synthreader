(function(){const s=document.createElement("link").relList;if(s&&s.supports&&s.supports("modulepreload"))return;for(const e of document.querySelectorAll('link[rel="modulepreload"]'))i(e);new MutationObserver(e=>{for(const a of e)if(a.type==="childList")for(const r of a.addedNodes)r.tagName==="LINK"&&r.rel==="modulepreload"&&i(r)}).observe(document,{childList:!0,subtree:!0});function t(e){const a={};return e.integrity&&(a.integrity=e.integrity),e.referrerPolicy&&(a.referrerPolicy=e.referrerPolicy),e.crossOrigin==="use-credentials"?a.credentials="include":e.crossOrigin==="anonymous"?a.credentials="omit":a.credentials="same-origin",a}function i(e){if(e.ep)return;e.ep=!0;const a=t(e);fetch(e.href,a)}})();async function o(n,s={},t){return window.__TAURI_INTERNALS__.invoke(n,s,t)}async function c(){const n=document.getElementById("app");n.innerHTML=`
    <div class="boot-screen">
      <h1 class="logo">SYNTHREADER</h1>
      <p class="tagline">Synthwave '84 Ebook Reader</p>
      <div class="loading-bar">
        <div class="loading-fill"></div>
      </div>
      <p class="version">v0.1.0</p>
    </div>
  `,await new Promise(s=>setTimeout(s,1500));try{const s=await o("greet",{name:"synth"});console.log(s)}catch(s){console.log("Tauri not available:",s)}n.innerHTML=`
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
  `}c();
//# sourceMappingURL=index-CydIN2OW.js.map
