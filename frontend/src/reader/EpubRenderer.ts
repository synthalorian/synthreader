interface EpubChapter {
  href: string
  title: string
  content: string
}

interface SystemFont {
  name: string
  path: string
  family: string
}

interface BookProgress {
  bookId: number
  chapterIndex: number
  scrollPosition: number
}

export class EpubRenderer extends HTMLElement {
  private chapters: EpubChapter[] = []
  private currentChapter = 0
  private tocVisible = false
  private bookId: number = 0
  private fonts: SystemFont[] = []
  private selectedFontPath: string = ''
  private fontPickerVisible = false
  private fontBase64Map: Map<string, string> = new Map()
  private boundKeyHandler = this.handleKeydown.bind(this)
  private scrollPosition: number = 0

  constructor() {
    super()
    this.attachShadow({ mode: 'open' })
  }

  async loadEpub(bookId: number) {
    this.bookId = bookId
    await this.loadFonts()
    await this.loadChapters()
    await this.loadProgress()
    this.render()
    document.addEventListener('keydown', this.boundKeyHandler)
  }

  disconnectedCallback() {
    document.removeEventListener('keydown', this.boundKeyHandler)
    this.saveProgress()
  }

  setChapters(chapters: EpubChapter[]) {
    this.chapters = chapters
    this.render()
  }

  private async loadFonts() {
    try {
      const { invoke } = await import('@tauri-apps/api/core')
      this.fonts = await invoke<SystemFont[]>('get_system_fonts')

      const saved = await invoke<string | null>('get_font_preference')
      if (saved) {
        this.selectedFontPath = saved
      } else if (this.fonts.length > 0) {
        const nerdFont = this.fonts.find(f =>
          f.name.toLowerCase().includes('3270') ||
          f.family.toLowerCase().includes('3270')
        )
        this.selectedFontPath = nerdFont?.path || this.fonts[0].path
      }

      await this.preloadFontData()
    } catch (e) {
      console.error('Failed to load fonts:', e)
    }
  }

  private async preloadFontData() {
    const { readFile } = await import('@tauri-apps/plugin-fs')
    for (const font of this.fonts) {
      try {
        const data = await readFile(font.path)
        const bytes = new Uint8Array(data)
        let binary = ''
        for (let i = 0; i < bytes.byteLength; i++) {
          binary += String.fromCharCode(bytes[i])
        }
        const base64 = btoa(binary)
        this.fontBase64Map.set(font.path, base64)
        console.log(`[Font] Preloaded: ${font.name} (${font.path}) -> ${base64.length} chars`)
      } catch (e) {
        console.warn(`[Font] Failed to preload ${font.name}:`, e)
      }
    }
    console.log(`[Font] Total preloaded: ${this.fontBase64Map.size}/${this.fonts.length}`)
  }

  private async loadChapters() {
    if (!this.bookId) return
    try {
      const { invoke } = await import('@tauri-apps/api/core')
      const chapters = await invoke<{ href: string; title: string }[]>('get_book_chapters', {
        bookId: this.bookId
      })
      this.chapters = chapters.map(ch => ({
        ...ch,
        content: ''
      }))
    } catch (e) {
      console.error('Failed to load chapters:', e)
    }
  }

  private async loadChapterContentAsync(href: string): Promise<string> {
    if (!this.bookId) return ''
    try {
      const { invoke } = await import('@tauri-apps/api/core')
      const content = await invoke<string>('get_chapter_content', {
        bookId: this.bookId,
        href
      })
      return content
    } catch (e) {
      console.error('Failed to load chapter content:', e)
      return `<p>Error loading chapter: ${e}</p>`
    }
  }

  private async saveProgress() {
    if (!this.bookId) return
    try {
      const { invoke } = await import('@tauri-apps/api/core')
      const progress: BookProgress = {
        bookId: this.bookId,
        chapterIndex: this.currentChapter,
        scrollPosition: this.scrollPosition
      }
      await invoke('save_book_progress', { progress })
    } catch (e) {
      console.error('Failed to save progress:', e)
    }
  }

  private async loadProgress() {
    if (!this.bookId) return
    try {
      const { invoke } = await import('@tauri-apps/api/core')
      const progress = await invoke<BookProgress | null>('get_book_progress', {
        bookId: this.bookId
      })
      if (progress) {
        this.currentChapter = Math.min(progress.chapterIndex, this.chapters.length - 1)
        this.scrollPosition = progress.scrollPosition || 0
      }
    } catch (e) {
      console.error('Failed to load progress:', e)
    }
  }

  private async saveFontPreference(path: string) {
    try {
      const { invoke } = await import('@tauri-apps/api/core')
      await invoke('set_font_preference', { fontPath: path })
    } catch (e) {
      console.error('Failed to save font preference:', e)
    }
  }

  private getSelectedFont(): SystemFont | undefined {
    return this.fonts.find(f => f.path === this.selectedFontPath)
  }

  private handleKeydown(e: KeyboardEvent) {
    if (!this.shadowRoot || this.chapters.length === 0) return

    const target = e.target as HTMLElement
    if (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable) return

    const frame = this.shadowRoot!.getElementById('content-frame') as HTMLIFrameElement
    const frameDoc = frame?.contentDocument || frame?.contentWindow?.document

    switch (e.key) {
      case 'ArrowRight':
      case 'PageDown':
        e.preventDefault()
        if (frameDoc) {
          const scrollHeight = frameDoc.documentElement.scrollHeight
          const clientHeight = frameDoc.documentElement.clientHeight
          const currentScroll = frameDoc.documentElement.scrollTop || frameDoc.body.scrollTop
          const maxScroll = scrollHeight - clientHeight

          if (currentScroll >= maxScroll - 10) {
            this.goNext()
          } else {
            frameDoc.documentElement.scrollTop = currentScroll + clientHeight * 0.9
          }
        } else {
          this.goNext()
        }
        break
      case 'ArrowLeft':
      case 'PageUp':
        e.preventDefault()
        if (frameDoc) {
          const currentScroll = frameDoc.documentElement.scrollTop || frameDoc.body.scrollTop

          if (currentScroll <= 10) {
            this.goPrev()
          } else {
            const clientHeight = frameDoc.documentElement.clientHeight
            frameDoc.documentElement.scrollTop = currentScroll - clientHeight * 0.9
          }
        } else {
          this.goPrev()
        }
        break
      case 'ArrowDown':
        e.preventDefault()
        if (frameDoc) {
          frameDoc.documentElement.scrollTop += 40
        }
        break
      case 'ArrowUp':
        e.preventDefault()
        if (frameDoc) {
          frameDoc.documentElement.scrollTop -= 40
        }
        break
      case 'Escape':
        if (this.tocVisible || this.fontPickerVisible) {
          e.preventDefault()
          this.tocVisible = false
          this.fontPickerVisible = false
          this.render()
        }
        break
    }
  }

  private goNext() {
    if (this.currentChapter < this.chapters.length - 1) {
      this.saveProgress()
      this.currentChapter++
      this.scrollPosition = 0
      this.render()
    }
  }

  private goPrev() {
    if (this.currentChapter > 0) {
      this.saveProgress()
      this.currentChapter--
      this.scrollPosition = 0
      this.render()
    }
  }

  private render() {
    this.renderAsync().catch(e => console.error('Render error:', e))
  }

  private async renderAsync() {
    const chapter = this.chapters[this.currentChapter]
    const progress = this.chapters.length > 0
      ? (this.currentChapter + 1) / this.chapters.length
      : 0

    const selectedFont = this.getSelectedFont()

    this.shadowRoot!.innerHTML = `
      <style>
        :host {
          display: flex;
          flex-direction: column;
          height: 100%;
          background: #0a0014;
          color: #f0e6ff;
        }
        .reader-header {
          display: flex;
          align-items: center;
          justify-content: space-between;
          padding: 12px 24px;
          border-bottom: 1px solid #3d0066;
          background: #140024;
          flex-shrink: 0;
        }
        .chapter-title {
          font-family: '3270 Nerd Font', 'Orbitron', sans-serif;
          font-size: 14px;
          color: #b300ff;
          white-space: nowrap;
          overflow: hidden;
          text-overflow: ellipsis;
          max-width: 50%;
        }
        .reader-controls {
          display: flex;
          gap: 12px;
          align-items: center;
        }
        .btn-icon {
          background: transparent;
          border: 1px solid #3d0066;
          color: #b899d9;
          padding: 6px 12px;
          cursor: pointer;
          font-family: '3270 Nerd Font', 'Orbitron', sans-serif;
          font-size: 11px;
          text-transform: uppercase;
          letter-spacing: 1px;
          transition: all 150ms ease;
        }
        .btn-icon:hover {
          border-color: #b300ff;
          color: #b300ff;
          box-shadow: 0 0 12px rgba(179, 0, 255, 0.4);
        }
        .btn-icon:disabled {
          opacity: 0.4;
          cursor: not-allowed;
          border-color: #1e0036;
          color: #6b4c85;
        }
        .font-picker-wrapper {
          position: relative;
        }
        .font-picker-dropdown {
          position: absolute;
          top: 100%;
          right: 0;
          margin-top: 8px;
          background: #140024;
          border: 1px solid #3d0066;
          border-radius: 4px;
          max-height: 300px;
          overflow-y: auto;
          min-width: 220px;
          z-index: 300;
          display: none;
          box-shadow: 0 4px 20px rgba(0, 0, 0, 0.6);
        }
        .font-picker-dropdown.visible {
          display: block;
        }
        .font-picker-header {
          padding: 10px 14px;
          font-family: '3270 Nerd Font', 'Orbitron', sans-serif;
          font-size: 11px;
          color: #b300ff;
          border-bottom: 1px solid #3d0066;
          text-transform: uppercase;
          letter-spacing: 1px;
        }
        .font-option {
          padding: 10px 14px;
          cursor: pointer;
          font-size: 13px;
          color: #b899d9;
          transition: all 150ms ease;
          border-bottom: 1px solid #1e0036;
        }
        .font-option:hover {
          background: rgba(179, 0, 255, 0.12);
          color: #b300ff;
        }
        .font-option.active {
          color: #ff00ff;
          background: rgba(255, 0, 255, 0.08);
        }
        .font-option .font-family {
          font-size: 11px;
          color: #6b4c85;
          margin-top: 2px;
        }
        .reader-body {
          flex: 1;
          overflow: hidden;
          position: relative;
        }
        .progress-bar {
          position: fixed;
          top: 0;
          left: 0;
          right: 0;
          height: 2px;
          background: #1e0036;
          z-index: 100;
        }
        .progress-fill {
          height: 100%;
          background: linear-gradient(90deg, #ff00ff, #b300ff);
          box-shadow: 0 0 10px #b300ff;
          width: ${progress * 100}%;
          transition: width 0.3s ease;
        }
        .content-frame {
          width: 100%;
          height: 100%;
          border: none;
          background: #0a0014;
        }
        .nav-overlay {
          position: fixed;
          top: 0;
          left: 0;
          bottom: 0;
          width: 300px;
          background: #140024;
          border-right: 1px solid #3d0066;
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
          font-family: '3270 Nerd Font', 'Orbitron', sans-serif;
          font-size: 14px;
          color: #b300ff;
          margin-bottom: 16px;
          text-transform: uppercase;
          letter-spacing: 2px;
        }
        .nav-item {
          padding: 8px 0;
          color: #b899d9;
          cursor: pointer;
          font-size: 13px;
          border-bottom: 1px solid #1e0036;
          transition: color 150ms ease;
        }
        .nav-item:hover {
          color: #ff00ff;
        }
        .nav-item.active {
          color: #b300ff;
        }
        .overlay-backdrop {
          position: fixed;
          inset: 0;
          background: rgba(10, 0, 20, 0.85);
          opacity: 0;
          pointer-events: none;
          transition: opacity 250ms ease;
          z-index: 150;
        }
        .overlay-backdrop.visible {
          opacity: 1;
          pointer-events: auto;
        }
        .page-hint {
          position: fixed;
          bottom: 20px;
          left: 50%;
          transform: translateX(-50%);
          background: rgba(20, 0, 36, 0.95);
          border: 1px solid #3d0066;
          padding: 6px 16px;
          border-radius: 4px;
          font-family: '3270 Nerd Font', 'Orbitron', sans-serif;
          font-size: 11px;
          color: #6b4c85;
          z-index: 50;
          pointer-events: none;
        }
        .page-hint kbd {
          background: #1e0036;
          border: 1px solid #3d0066;
          padding: 2px 6px;
          border-radius: 3px;
          color: #b899d9;
          font-family: monospace;
          font-size: 10px;
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
          <div class="font-picker-wrapper">
            <button class="btn-icon" id="btn-font">Font: ${selectedFont?.name || 'Default'}</button>
            <div class="font-picker-dropdown ${this.fontPickerVisible ? 'visible' : ''}" id="font-dropdown">
              <div class="font-picker-header">Select Font</div>
              ${this.fonts.map(f => `
                <div class="font-option ${f.path === this.selectedFontPath ? 'active' : ''}"
                     data-path="${f.path}">
                  ${f.name}
                  <div class="font-family">${f.family}</div>
                </div>
              `).join('')}
            </div>
          </div>
          <button class="btn-icon" id="btn-toc">Contents</button>
          <button class="btn-icon" id="btn-prev" ${this.currentChapter <= 0 ? 'disabled' : ''}>← Prev</button>
          <button class="btn-icon" id="btn-next" ${this.currentChapter >= this.chapters.length - 1 ? 'disabled' : ''}>Next →</button>
        </div>
      </div>

      <div class="reader-body">
        <iframe class="content-frame" id="content-frame"
                sandbox="allow-same-origin"></iframe>
      </div>

      <div class="page-hint">
        <kbd>←</kbd> prev <kbd>→</kbd> next <kbd>esc</kbd> close menu
      </div>
    `

    this.setupEventListeners()
    if (chapter) {
      await this.loadChapterContent(chapter)
    }
  }

  private setupEventListeners() {
    this.shadowRoot!.getElementById('btn-toc')?.addEventListener('click', () => {
      this.tocVisible = !this.tocVisible
      this.fontPickerVisible = false
      this.render()
    })

    this.shadowRoot!.getElementById('btn-font')?.addEventListener('click', () => {
      this.fontPickerVisible = !this.fontPickerVisible
      this.tocVisible = false
      this.render()
    })

    this.shadowRoot!.querySelectorAll('.font-option').forEach(item => {
      item.addEventListener('click', (e) => {
        const path = (e.currentTarget as HTMLElement).dataset.path!
        this.selectedFontPath = path
        this.fontPickerVisible = false
        this.saveFontPreference(path)
        this.render()
      })
    })

    this.shadowRoot!.getElementById('backdrop')?.addEventListener('click', () => {
      this.tocVisible = false
      this.fontPickerVisible = false
      this.render()
    })

    this.shadowRoot!.getElementById('btn-prev')?.addEventListener('click', () => this.goPrev())
    this.shadowRoot!.getElementById('btn-next')?.addEventListener('click', () => this.goNext())

    this.shadowRoot!.querySelectorAll('.nav-item').forEach(item => {
      item.addEventListener('click', (e) => {
        const idx = parseInt((e.currentTarget as HTMLElement).dataset.index!)
        this.currentChapter = idx
        this.tocVisible = false
        this.render()
      })
    })
  }

  private async loadChapterContent(chapter: EpubChapter) {
    const frame = this.shadowRoot!.getElementById('content-frame') as HTMLIFrameElement
    if (!frame) return

    let content = chapter.content
    if (!content && this.bookId) {
      content = await this.loadChapterContentAsync(chapter.href)
    }

    const selectedFont = this.getSelectedFont()
    const fontPath = selectedFont?.path || ''
    const fontName = selectedFont?.name || '3270 Nerd Font'

    // Build font-face rule - try local() first, fall back to base64
    let fontFaceRule = ''
    let fontFamilyValue = 'Georgia, serif'

    if (fontPath) {
      const base64 = this.fontBase64Map.get(fontPath)
      console.log(`[Font] Selected: ${fontName} at ${fontPath}, base64 found: ${!!base64}`)
      
      // Use local() reference to system font - works in WebKitGTK if font is installed
      fontFaceRule = `@font-face {
        font-family: 'ReaderFont';
        src: local('3270 Nerd Font'), local('3270NF'), local('${fontName}'), local('${fontName.replace(/\s/g, '')}');
        font-weight: 400;
        font-style: normal;
      }`
      fontFamilyValue = "'ReaderFont', monospace"
      console.log(`[Font] Using local() reference for: ${fontName}`)
    } else {
      console.warn(`[Font] No fontPath selected`)
    }

    const html = `
      <!DOCTYPE html>
      <html>
      <head>
        <meta charset="UTF-8">
        <style>
          ${fontFaceRule}
          :root {
            --reader-bg: #0a0014;
            --reader-text: #f0e6ff;
            --reader-accent: #b300ff;
            --reader-font-size: 18px;
            --reader-line-height: 1.8;
            --reader-margin: 40px;
          }
          body {
            background: var(--reader-bg) !important;
            color: var(--reader-text) !important;
            font-family: ${fontFamilyValue} !important;
            font-size: var(--reader-font-size) !important;
            line-height: var(--reader-line-height) !important;
            font-weight: 400 !important;
            max-width: 700px;
            margin: 0 auto;
            padding: var(--reader-margin);
          }
          * { background: transparent !important; color: var(--reader-text) !important; font-family: inherit !important; font-weight: inherit !important; }
          a { color: var(--reader-accent) !important; text-decoration: underline; }
          a:hover { color: #ff00ff !important; }
          img { max-width: 100%; height: auto; filter: brightness(0.95) contrast(1.05); }
          ::selection { background: rgba(179, 0, 255, 0.35); color: #fff; }
          h1, h2, h3, h4, h5, h6 { color: #ff00ff !important; margin-top: 1.5em; margin-bottom: 0.5em; font-family: ${fontFamilyValue} !important; font-weight: 700 !important; }
          p { margin-bottom: 1em; }
          blockquote { border-left: 3px solid #b300ff; padding-left: 16px; margin-left: 0; color: #b899d9 !important; }
          code { background: #140024 !important; padding: 2px 6px; border-radius: 3px; font-family: ${fontFamilyValue} !important; font-size: 0.9em; }
          pre { background: #140024 !important; padding: 16px; border-radius: 6px; overflow-x: auto; font-family: ${fontFamilyValue} !important; }
        </style>
      </head>
      <body>${content}</body>
      </html>
    `

    frame.srcdoc = html

    // Restore scroll position after content loads
    if (this.scrollPosition > 0) {
      frame.onload = () => {
        const frameDoc = frame.contentDocument || frame.contentWindow?.document
        if (frameDoc) {
          frameDoc.documentElement.scrollTop = this.scrollPosition
        }
      }
    }
  }
}

customElements.define('epub-renderer', EpubRenderer)
