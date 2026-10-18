// Stashes of the current repository: the list under the changed files, the stash
// shown in the main area (its files and diffs) and saving / restoring / deleting.
// Builds on `app` (state.svelte.ts); the stash message is the commit form's summary.
import { api, type CommitFile, type FileDiff, type StashDetail, type StashEntry } from './api'
import { changes } from './changes.svelte'
import { confirmDialog } from './dialogs'
import { t } from './i18n/index.svelte'
import { perf } from './perf'
import { prefs } from './prefs.svelte'
import { app } from './state.svelte'

class StashState {
  stashes = $state.raw<StashEntry[]>([])
  /** The stash shown instead of the working tree diff. */
  selected = $state.raw<StashEntry | null>(null)
  detail = $state.raw<StashDetail | null>(null)
  file = $state<string | null>(null)
  diff = $state.raw<FileDiff | null>(null)

  /** Forgets everything about the previous repository (called by `app.selectRepo`). */
  resetRepoView() {
    this.stashes = []
    this.close()
  }

  async load() {
    const repo = app.selectedPath
    if (!repo) return
    // Not worth an error dialog: the list just stays as it was.
    const stashes = await app.track(api.listStashes(repo)).catch(() => null)
    if (!stashes || repo !== app.selectedPath) return
    this.stashes = stashes
    if (this.selected && !stashes.some((s) => s.oid === this.selected?.oid)) this.close()
  }

  /** Shows a stash's files and diffs in the main area. */
  async open(entry: StashEntry) {
    const repo = app.selectedPath
    if (!repo) return
    this.selected = entry
    this.detail = null
    this.diff = null
    await perf.view('stash.open', async () => {
      const detail = await app.guard(() => api.getStash(repo, entry.oid))
      if (!detail || this.selected?.oid !== entry.oid) return
      this.detail = detail
      await this.selectFile(detail.files[0]?.path ?? null)
    })
  }

  close() {
    this.selected = null
    this.detail = null
    this.file = null
    this.diff = null
  }

  async selectFile(path: string | null) {
    this.file = path
    const repo = app.selectedPath
    const detail = this.detail
    const file = detail?.files.find((f: CommitFile) => f.path === path)
    if (!repo || !detail || !file) {
      this.diff = null
      return
    }
    const diff = await app.guard(() => api.getStashDiff(repo, detail, file))
    if (diff && this.file === path && this.detail === detail) this.diff = diff
  }

  /** Stashes every change, or only `paths` (renames with both paths). */
  async save(paths: string[] = []) {
    if (app.files.length === 0) {
      app.toast(t('stash.noChanges'))
      return
    }
    const message = changes.summary.trim()
    const done = await app.run('stash', (repo) => api.stashSave(repo, message, prefs.showUntracked, paths))
    if (done === undefined) return
    app.toast(t(paths.length ? 'stash.savedSome' : 'stash.saved'))
    await Promise.all([app.loadStatus(), this.load()])
  }

  /** Brings a stash's changes back; `keep` leaves it in the list (apply instead of pop). */
  async restore(entry: StashEntry, keep: boolean) {
    const outcome = await app.run('stash', (repo) => (keep ? api.stashApply(repo, entry) : api.stashPop(repo, entry)))
    if (!outcome) return
    if (outcome === 'conflicts') app.toast(t('operation.stashConflicts'), 'error')
    else app.toast(t(keep ? 'stash.applied' : 'stash.popped'))
    if (!keep) this.close()
    await app.setTab('changes')
    await Promise.all([app.loadStatus(), this.load()])
  }

  async drop(entry: StashEntry) {
    if (!(await confirmDialog(t('stash.dropConfirm', { message: entry.message }), t('stash.drop')))) return
    const done = await app.run('stash', (repo) => api.stashDrop(repo, entry))
    if (done === undefined) return
    app.toast(t('stash.dropped'))
    await this.load()
  }
}

export const stash = new StashState()
