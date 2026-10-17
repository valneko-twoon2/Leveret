// Sizes of the resizable panes. Remembered per user in localStorage (a
// convenience only: everything works with the defaults when storage is unavailable).

export interface PaneSpec {
  default: number
  min: number
  max: number
}

export const PANES = {
  /** Repository list. */
  sidebar: { default: 220, min: 160, max: 420 },
  /** Changes / History column. */
  column: { default: 340, min: 240, max: 720 },
  /** File list of a commit in the History tab. */
  commitFiles: { default: 280, min: 160, max: 640 },
} satisfies Record<string, PaneSpec>

export type PaneName = keyof typeof PANES

const STORAGE_KEY = 'leveret.layout'

function load(): Partial<Record<PaneName, number>> {
  try {
    const value: unknown = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? '{}')
    return typeof value === 'object' && value !== null ? (value as Partial<Record<PaneName, number>>) : {}
  } catch {
    return {}
  }
}

function clamp(name: PaneName, value: number): number {
  const spec = PANES[name]
  return Number.isFinite(value) ? Math.round(Math.min(spec.max, Math.max(spec.min, value))) : spec.default
}

class Layout {
  sizes = $state<Record<PaneName, number>>({
    sidebar: PANES.sidebar.default,
    column: PANES.column.default,
    commitFiles: PANES.commitFiles.default,
  })

  constructor() {
    const saved = load()
    for (const name of Object.keys(PANES) as PaneName[]) {
      if (saved[name] !== undefined) this.sizes[name] = clamp(name, saved[name])
    }
  }

  set(name: PaneName, value: number) {
    this.sizes[name] = clamp(name, value)
  }

  reset(name: PaneName) {
    this.sizes[name] = PANES[name].default
    this.save()
  }

  /**
   * Shrinks panes that don't fit the window, so the diff keeps a usable width
   * (at the minimum window size too). Not saved: the remembered sizes come back
   * when the window is wide enough again.
   */
  fit(windowWidth: number) {
    const saved = load()
    const want = (name: PaneName) => clamp(name, saved[name] ?? PANES[name].default)
    const fitted = (name: PaneName, room: number) =>
      Math.round(Math.max(PANES[name].min, Math.min(want(name), room)))
    this.sizes.sidebar = fitted('sidebar', windowWidth * 0.22)
    this.sizes.column = fitted('column', windowWidth - this.sizes.sidebar - 400)
    this.sizes.commitFiles = fitted('commitFiles', windowWidth - this.sizes.sidebar - this.sizes.column - 320)
  }

  resetAll() {
    for (const name of Object.keys(PANES) as PaneName[]) this.sizes[name] = PANES[name].default
    this.save()
  }

  save() {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(this.sizes))
    } catch {
      // Storage unavailable (private mode, blocked): sizes just aren't remembered.
    }
  }
}

export const layout = new Layout()
