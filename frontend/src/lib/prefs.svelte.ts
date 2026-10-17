// Display preferences, remembered per user in localStorage (like the pane sizes).

const STORAGE_KEY = 'leveret.prefs'

interface Saved {
  showUntracked?: boolean
}

function load(): Saved {
  try {
    const value: unknown = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? '{}')
    return value && typeof value === 'object' ? (value as Saved) : {}
  } catch {
    return {}
  }
}

class Prefs {
  /** Untracked files are listed in the Changes tab. */
  showUntracked = $state(load().showUntracked !== false)

  setShowUntracked(show: boolean) {
    this.showUntracked = show
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify({ showUntracked: show } satisfies Saved))
    } catch {
      // Not remembered, but still applied for this session.
    }
  }
}

export const prefs = new Prefs()
