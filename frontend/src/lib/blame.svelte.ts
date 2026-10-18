// The annotate (blame) view: who last changed each line of a file, in the working
// tree or at a commit. Clicking a line's commit shows it in the History tab.
import { api, type Blame } from './api'
import { history } from './history.svelte'
import { perf } from './perf'
import { prefs } from './prefs.svelte'
import { app } from './state.svelte'

/** Lines that are not committed yet blame this all-zero hash. */
export const isUncommitted = (hash: string) => /^0+$/.test(hash)

class BlameState {
  /** The file shown; null while the view is closed. */
  path = $state<string | null>(null)
  /** The commit it is shown at; null for the working tree file. */
  rev = $state<string | null>(null)
  data = $state.raw<Blame | null>(null)

  async open(path: string, rev: string | null = null) {
    const repo = app.selectedPath
    if (!repo) return
    this.path = path
    this.rev = rev
    this.data = null
    await perf.view('blame.open', async () => {
      const data = await app.guard(() => api.getBlame(repo, rev, path, prefs.ignoreWhitespace))
      if (!data || this.path !== path || this.rev !== rev) return
      this.data = data
    })
    if (!this.data) this.close()
  }

  close() {
    this.path = null
    this.rev = null
    this.data = null
  }

  /** Shows a line's commit in the History tab. */
  async showCommit(hash: string) {
    if (isUncommitted(hash)) return
    this.close()
    await history.jumpTo(hash)
  }
}

export const blame = new BlameState()
