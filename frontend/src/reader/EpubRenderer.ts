interface EpubChapter {
  href: string
  title: string
  content: string
}

export class EpubRenderer extends HTMLElement {
  private chapters: EpubChapter[] = []
  private currentChapter = 0
  private tocVisible = false

  constructor() {
    super()
    this.attachShadow({ mode: 'open' })
  }

  async loadEpub(bookId: number) {
    // TODO: Fetch chapters from backend
    this.render()
  }

  setChapters(chapters: EpubChapter[]) {
    this.chapters = chapters
    this.render()
  }

  private render() {
    const chapter = this.chapters[this.currentChapter]
    const progress = this.chapters.length > 0
      ? (this.currentChapter + 1) / this.chapters.length
      : 0

    this.shadowRoot!.innerHTML = `
      <style>
        :host {
          display: flex;
          flex-direction: column;
          height: 100%;
          background: #0a0a1a;
          color: #e0e0ff;
        }
        .reader-header {
          display: flex;
          align-items: center;
          justify-content: space-between;
          padding: 12px 24px;
          border-bottom: 1px solid #2a2a5a;
          background: #12122a;
        }
        .chapter-title {
          font-family: 'Orbitron', sans-serif;
          font-size: 14px;
          color: #05d9e8;
        }
        .reader-controls {
          display: flex;
          gap: 12px;
        }
        .btn-icon {
          background: transparent;
          border: 1px solid #2a2a5a;
          color: #a0a0d0;
          padding: 6px 12px;
          cursor: pointer;
          font-family: 'Orbitron', sans-serif;
          font-size: 11px;
          text-transform: uppercase;
          letter-spacing: 1px;
          transition: all 150ms ease;
        }
        .btn-icon:hover {
          border-color: #05d9e8;
          color: #05d9e8;
          box-shadow: 0 0 8px rgba(5, 217, 232, 0.3);
        }
        .reader-body {
          flex: 1;
          overflow-y: auto;
          position: relative;
        }
        .progress-bar {
          position: fixed;
          top: 0;
          left: 0;
          right: 0;
          height: 2px;
          background: #222250;
          z-index: 100;
        }
        .progress-fill {
          height: 100%;
          background: linear-gradient(90deg, #ff2a6d, #05d9e8);
          box-shadow: 0 0 10px #05d9e8;
          width: ${progress * 100}%;
          transition: width 0.3s ease;
        }
        .content-frame {
          width: 100%;
          height: 100%;
          border: none;
          background: #0a0a1a;
        }
        .nav-overlay {
          position: fixed;
          top: 0;
          left: 0;
          bottom: 0;
          width: 300px;
          background: #12122a;
          border-right: 1px solid #2a2a5a;
          transform: translateX(-100%);
          transition: transform 250ms ease;
          z-index: 200;
          overflow-y: auto;
          padding: 16px;
        }
        .nav-overlay.visible {
          transform: translateX(0);
        }
        .nav-title {
          font-family: 'Orbitron', sans-serif;
          font-size: 14px;
          color: #05d9e8;
          margin-bottom: 16px;
          text-transform: uppercase;
          letter-spacing: 2px;
        }
        .nav-item {
          padding: 8px 0;
          color: #a0a0d0;
          cursor: pointer;
          font-size: 13px;
          border-bottom: 1px solid #1a1a3e;
          transition: color 150ms ease;
        }
        .nav-item:hover {
          color: #ff2a6d;
        }
        .nav-item.active {
          color: #05d9e8;
        }
        .overlay-backdrop {
          position: fixed;
          inset: 0;
          background: rgba(10, 10, 26, 0.8);
          opacity: 0;
          pointer-events: none;
          transition: opacity 250ms ease;
          z-index: 150;
        }
        .overlay-backdrop.visible {
          opacity: 1;
          pointer-events: auto;
        }
      </style>

      <div class="progress-bar">
        <div class="progress-fill"></div>
      </div>

      <div class="overlay-backdrop ${this.tocVisible ? 'visible' : ''}"
           id="backdrop"></div>

      <div class="nav-overlay ${this.tocVisible ? 'visible' : ''}" id="nav-overlay">
        <div class="nav-title">Contents</div>
        ${this.chapters.map((ch, i) => `
          <div class="nav-item ${i === this.currentChapter ? 'active' : ''}"
               data-index="${i}">
            ${ch.title || `Chapter ${i + 1}`}
          </div>
        `).join('')}
      </div>

      <div class="reader-header">
        <span class="chapter-title">${chapter?.title || 'Untitled'}</span>
        <div class="reader-controls">
          <button class="btn-icon" id="btn-toc">Contents</button>
          <button class="btn-icon" id="btn-prev">← Prev</button>
          <button class="btn-icon" id="btn-next">Next →</button>
        </div>
      </div>

      <div class="reader-body">
        <iframe class="content-frame" id="content-frame"
                sandbox="allow-same-origin"></iframe>
      </div>
    `

    this.setupEventListeners()
    if (chapter) {
      this.loadChapterContent(chapter)
    }
  }

  private setupEventListeners() {
    this.shadowRoot!.getElementById('btn-toc')?.addEventListener('click', () => {
      this.tocVisible = !this.tocVisible
      this.render()
    })

    this.shadowRoot!.getElementById('backdrop')?.addEventListener('click', () => {
      this.tocVisible = false
      this.render()
    })

    this.shadowRoot!.getElementById('btn-prev')?.addEventListener('click', () => {
      if (this.currentChapter > 0) {
        this.currentChapter--
        this.render()
      }
    })

    this.shadowRoot!.getElementById('btn-next')?.addEventListener('click', () => {
      if (this.currentChapter < this.chapters.length - 1) {
        this.currentChapter++
        this.render()
      }
    })

    this.shadowRoot!.querySelectorAll('.nav-item').forEach(item => {
      item.addEventListener('click', (e) => {
        const idx = parseInt((e.currentTarget as HTMLElement).dataset.index!)
        this.currentChapter = idx
        this.tocVisible = false
        this.render()
      })
    })
  }

  private loadChapterContent(chapter: EpubChapter) {
    const frame = this.shadowRoot!.getElementById('content-frame') as HTMLIFrameElement
    if (!frame) return

    const html = `
      <!DOCTYPE html>
      <html>
      <head>
        <meta charset="UTF-8">
        <style>
          @import url('https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700&display=swap');
          :root {
            --reader-bg: #0a0a1a;
            --reader-text: #e0e0ff;
            --reader-accent: #05d9e8;
            --reader-font-size: 18px;
            --reader-line-height: 1.8;
            --reader-margin: 40px;
          }
          body {
            background: var(--reader-bg) !important;
            color: var(--reader-text) !important;
            font-family: 'Inter', Georgia, serif !important;
            font-size: var(--reader-font-size) !important;
            line-height: var(--reader-line-height) !important;
            max-width: 700px;
            margin: 0 auto;
            padding: var(--reader-margin);
          }
          * { background: transparent !important; color: var(--reader-text) !important; }
          a { color: var(--reader-accent) !important; }
          img { max-width: 100%; height: auto; filter: brightness(0.9) contrast(1.1); }
          ::selection { background: rgba(5, 217, 232, 0.3); color: #fff; }
        </style>
      </head>
      <body>${chapter.content}</body>
      </html>
    `

    frame.srcdoc = html
  }
}

customElements.define('epub-renderer', EpubRenderer)
