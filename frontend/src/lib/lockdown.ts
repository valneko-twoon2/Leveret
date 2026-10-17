// Second layer of the browser lockdown (the first is in backend/src/lockdown.rs):
// suppresses browser shortcuts, zoom, the context menu, back/forward buttons,
// file drops and link navigation, so the app behaves like a desktop app even
// when WebView2 settings are unavailable (or in the browser preview).

const CTRL_KEYS = new Set([
  'KeyR', // reload
  'KeyP', // print
  'KeyF', // find
  'KeyG', // find next
  'KeyU', // view source
  'KeyS', // save page
  'KeyO', // open file
  'KeyN', // new window
  'KeyT', // new tab
  'KeyW', // close tab
  'KeyJ', // downloads
  'KeyH', // history
  'KeyD', // bookmark
  'KeyL', // address bar
  'KeyE', // search
  'Equal',
  'Minus',
  'NumpadAdd',
  'NumpadSubtract',
  'Digit0',
  'Numpad0',
])

const CTRL_SHIFT_KEYS = new Set(['KeyI', 'KeyJ', 'KeyC', 'KeyR', 'KeyN', 'KeyT', 'KeyB', 'KeyO', 'KeyM', 'Delete'])

/** Function keys the app handles itself (see App.svelte). */
const APP_KEYS = new Set(['F5'])

function isEditable(target: EventTarget | null): boolean {
  return target instanceof HTMLElement && (target.isContentEditable || target.matches('input, textarea, select'))
}

/** Whether a key event would trigger browser behaviour. App shortcuts are still delivered. */
export function isBrowserShortcut(e: KeyboardEvent, devtools = false): boolean {
  if (/^F\d{1,2}$/.test(e.key)) {
    if (devtools && e.key === 'F12') return false
    return !APP_KEYS.has(e.key) || e.ctrlKey || e.shiftKey || e.altKey // Ctrl+F5 etc. reload
  }
  if (e.key === 'BrowserBack' || e.key === 'BrowserForward' || e.key === 'BrowserRefresh' || e.key === 'BrowserSearch') return true
  if (e.altKey && (e.key === 'ArrowLeft' || e.key === 'ArrowRight' || e.key === 'Home')) return true
  const ctrl = e.ctrlKey || e.metaKey
  if (ctrl && e.shiftKey) return !(devtools && (e.code === 'KeyI' || e.code === 'KeyJ' || e.code === 'KeyC')) && CTRL_SHIFT_KEYS.has(e.code)
  if (ctrl && !e.altKey) {
    // Ctrl+A/C/V/X/Z/Y keep working; Ctrl+F etc. are blocked even in text fields.
    return CTRL_KEYS.has(e.code)
  }
  if (e.key === 'Backspace' && !isEditable(e.target)) return true // legacy "back"
  return false
}

/** Installs the guards. Returns a function that removes them. */
export function installLockdown(options: { devtools?: boolean } = {}): () => void {
  const devtools = options.devtools ?? false
  const ac = new AbortController()
  const opts = { capture: true, signal: ac.signal } as const

  window.addEventListener(
    'keydown',
    (e) => {
      // preventDefault only: app shortcuts on the same keys still run.
      if (isBrowserShortcut(e, devtools)) e.preventDefault()
    },
    opts,
  )
  window.addEventListener('contextmenu', (e) => e.preventDefault(), opts)
  window.addEventListener('wheel', (e) => e.ctrlKey && e.preventDefault(), { ...opts, passive: false })
  for (const type of ['gesturestart', 'gesturechange'] as const) window.addEventListener(type, (e) => e.preventDefault(), opts)
  // Mouse back/forward buttons.
  for (const type of ['mouseup', 'mousedown', 'auxclick'] as const) {
    window.addEventListener(type, (e) => (e as MouseEvent).button > 2 && e.preventDefault(), opts)
  }
  // Dropped files/links must not navigate the page (Tauri delivers file drops natively).
  for (const type of ['dragover', 'drop'] as const) window.addEventListener(type, (e) => e.preventDefault(), opts)
  // Links never navigate the app window.
  window.addEventListener(
    'click',
    (e) => {
      const link = (e.target as Element | null)?.closest?.('a[href]')
      if (link) e.preventDefault()
    },
    opts,
  )
  // No popups.
  const originalOpen = window.open
  window.open = () => null

  return () => {
    ac.abort()
    window.open = originalOpen
  }
}
