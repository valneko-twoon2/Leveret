// Display preferences, remembered per user in localStorage (like the pane sizes).
// Settings that other tools share (git's own config) or the backend needs live
// in the backend instead (see settings.svelte.ts).

const STORAGE_KEY = 'leveret.prefs'

export type Theme = 'system' | 'light' | 'dark'

interface Saved {
  showUntracked: boolean
  theme: Theme
  ignoreWhitespace: boolean
  tabSize: number
  /** Unchanged lines shown around each change in a diff. */
  contextLines: number
  /** Minutes between background fetches; 0 turns them off. */
  autoFetchMinutes: number
  /** Push right after each commit. */
  pushAfterCommit: boolean
}

const DEFAULTS: Saved = {
  showUntracked: true,
  theme: 'system',
  ignoreWhitespace: false,
  tabSize: 4,
  contextLines: 3,
  autoFetchMinutes: 10,
  pushAfterCommit: false,
}

export const TAB_SIZES = [2, 4, 8] as const
export const CONTEXT_LINES = [0, 1, 3, 5, 10, 25] as const
export const AUTO_FETCH_MINUTES = [0, 5, 10, 30, 60] as const

/** Saved values that are still valid; anything else falls back to the default. */
function load(): Saved {
  let raw: Record<string, unknown> = {}
  try {
    const value: unknown = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? '{}')
    if (value && typeof value === 'object') raw = value as Record<string, unknown>
  } catch {
    // Unreadable: defaults.
  }
  const pick = <K extends keyof Saved>(key: K, ok: (v: unknown) => boolean): Saved[K] =>
    ok(raw[key]) ? (raw[key] as Saved[K]) : DEFAULTS[key]
  return {
    showUntracked: pick('showUntracked', (v) => typeof v === 'boolean'),
    theme: pick('theme', (v) => v === 'system' || v === 'light' || v === 'dark'),
    ignoreWhitespace: pick('ignoreWhitespace', (v) => typeof v === 'boolean'),
    tabSize: pick('tabSize', (v) => (TAB_SIZES as readonly unknown[]).includes(v)),
    contextLines: pick('contextLines', (v) => (CONTEXT_LINES as readonly unknown[]).includes(v)),
    autoFetchMinutes: pick('autoFetchMinutes', (v) => (AUTO_FETCH_MINUTES as readonly unknown[]).includes(v)),
    pushAfterCommit: pick('pushAfterCommit', (v) => typeof v === 'boolean'),
  }
}

class Prefs {
  private saved = load()
  showUntracked = $state(this.saved.showUntracked)
  theme = $state<Theme>(this.saved.theme)
  ignoreWhitespace = $state(this.saved.ignoreWhitespace)
  tabSize = $state(this.saved.tabSize)
  contextLines = $state(this.saved.contextLines)
  autoFetchMinutes = $state(this.saved.autoFetchMinutes)
  pushAfterCommit = $state(this.saved.pushAfterCommit)

  /** Changes one preference and remembers it. */
  set<K extends keyof Saved>(key: K, value: Saved[K]) {
    ;(this as unknown as Saved)[key] = value
    this.save()
  }

  /** What changes the text of a diff: sent along with every diff request. */
  get diffOptions() {
    return { ignoreWhitespace: this.ignoreWhitespace, context: this.contextLines }
  }

  setShowUntracked(show: boolean) {
    this.set('showUntracked', show)
  }

  reset() {
    for (const key of Object.keys(DEFAULTS) as (keyof Saved)[]) (this as unknown as Record<string, unknown>)[key] = DEFAULTS[key]
    this.save()
  }

  private save() {
    const value: Saved = {
      showUntracked: this.showUntracked,
      theme: this.theme,
      ignoreWhitespace: this.ignoreWhitespace,
      tabSize: this.tabSize,
      contextLines: this.contextLines,
      autoFetchMinutes: this.autoFetchMinutes,
      pushAfterCommit: this.pushAfterCommit,
    }
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(value))
    } catch {
      // Not remembered, but still applied for this session.
    }
  }
}

export const prefs = new Prefs()
