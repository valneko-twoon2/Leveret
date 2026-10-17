// The changes tab: file selection and working-tree diff, the commit form,
// discard / resolve / ignore and stashes. Builds on `app` (state.svelte.ts),
// which loads the status these act on.
import { SvelteSet } from 'svelte/reactivity'
import { api, type FileChange, type FileDiff, type RepoStatus, type StashEntry } from './api'
import { confirmDialog } from './dialogs'
import { t } from './i18n/index.svelte'
import { perf } from './perf'
import { app } from './state.svelte'

class ChangesState {
  /** Paths unchecked by the user; everything else is included in the commit. */
  excluded = new SvelteSet<string>()
  selectedFile = $state<string | null>(null)
  workingDiff = $state.raw<FileDiff | null>(null)
  summary = $state('')
  description = $state('')
  amend = $state(false)
  stashes = $state.raw<StashEntry[]>([])

  includedFiles = $derived(app.files.filter((f) => !this.excluded.has(f.path)))
  selectedChange = $derived(app.files.find((f) => f.path === this.selectedFile) ?? null)

  canCommit = $derived(
    app.busy === null &&
      !app.commitBlocked &&
      this.summary.trim() !== '' &&
      (this.amend ? app.canRewriteHead : this.includedFiles.length > 0),
  )

  /** Message put into the form by "amend", so turning amend off can clear it again. */
  private amendPrefill: { summary: string; description: string } | null = null

  /** Forgets everything about the previous repository (called by `app.selectRepo`). */
  resetRepoView() {
    this.excluded.clear()
    this.selectedFile = null
    this.workingDiff = null
    this.summary = ''
    this.description = ''
    this.amend = false
    this.amendPrefill = null
    this.stashes = []
  }

  /** Brings the tab in line with a freshly loaded status (called by `app.loadStatus`). */
  async syncWithStatus(status: RepoStatus) {
    if (this.amend && !app.canRewriteHead) this.setAmend(false)
    // Forget exclusions for files that are gone.
    const present = new Set(status.files.map((f) => f.path))
    for (const p of [...this.excluded]) if (!present.has(p)) this.excluded.delete(p)
    const keep = this.selectedFile && present.has(this.selectedFile) ? this.selectedFile : (status.files[0]?.path ?? null)
    await this.selectFile(keep)
  }

  async loadStashes() {
    const repo = app.selectedPath
    if (!repo) return
    // Not worth an error dialog: the list just stays as it was.
    const stashes = await app.track(api.listStashes(repo)).catch(() => null)
    if (stashes && repo === app.selectedPath) this.stashes = stashes
  }

  async selectFile(path: string | null) {
    this.selectedFile = path
    const repo = app.selectedPath
    const file = app.files.find((f) => f.path === path)
    if (!repo || !file) {
      this.workingDiff = null
      return
    }
    await perf.view('diff.file', async () => {
      const diff = await app.guard(() => api.getWorkingDiff(repo, file))
      if (diff && this.selectedFile === path && repo === app.selectedPath) this.workingDiff = diff
    })
  }

  toggleFile(path: string) {
    if (this.excluded.has(path)) this.excluded.delete(path)
    else this.excluded.add(path)
  }

  toggleAll() {
    if (this.includedFiles.length === app.files.length) {
      for (const f of app.files) this.excluded.add(f.path)
    } else {
      this.excluded.clear()
    }
  }

  /** Turns "amend the last commit" on (pre-filling its message) or off. */
  async setAmend(on: boolean) {
    this.amend = on
    const repo = app.selectedPath
    const head = app.status?.headOid
    if (on && repo && head && !this.summary.trim() && !this.description.trim()) {
      const detail = await app.guard(() => api.getCommit(repo, head))
      if (!detail || !this.amend) return
      const [summary, ...rest] = detail.message.split('\n')
      this.summary = summary
      this.description = rest.join('\n').trim()
      this.amendPrefill = { summary: this.summary, description: this.description }
    } else if (!on && this.amendPrefill) {
      if (this.summary === this.amendPrefill.summary && this.description === this.amendPrefill.description) {
        this.summary = ''
        this.description = ''
      }
      this.amendPrefill = null
    }
  }

  async commit() {
    if (!this.canCommit) return
    const paths = this.includedFiles.flatMap((f: FileChange) => (f.origPath ? [f.path, f.origPath] : [f.path]))
    const summary = this.summary.trim()
    const amend = this.amend
    const hash = await app.run('commit', (repo) => api.commit(repo, { summary, description: this.description, paths, amend }))
    if (!hash) return
    this.summary = ''
    this.description = ''
    this.amend = false
    this.amendPrefill = null
    app.toast(amend ? t('commit.amended') : t('commit.done', { summary }))
    await app.refreshAfterChange()
  }

  /** Removes the last commit, keeping its changes and putting its message back into the form. */
  async undoLastCommit() {
    const message = await app.run('commit', (repo) => api.undoLastCommit(repo))
    if (message === undefined) return
    if (!this.summary.trim() && !this.description.trim()) {
      const [summary, ...rest] = message.split('\n')
      this.summary = summary
      this.description = rest.join('\n').trim()
    }
    app.toast(t('commit.undone'))
    await app.setTab('changes')
    await app.refreshAfterChange()
  }

  // --- discard, resolve, ignore ---------------------------------------------

  async discard(paths: string[]) {
    if (paths.length === 0) return
    const question = paths.length === 1 ? t('discard.confirmOne', { path: paths[0] }) : t('discard.confirmMany', { count: paths.length })
    if (!(await confirmDialog(`${question}\n${t('discard.confirmNote')}`, t('discard.title')))) return
    const done = await app.run('discard', (repo) => api.discardChanges(repo, paths))
    if (done === undefined) return
    app.toast(t('discard.done', { count: paths.length }))
    await app.loadStatus()
  }

  async markResolved(paths: string[]) {
    if (paths.length === 0) return
    const done = await app.run('operation', (repo) => api.markResolved(repo, paths))
    if (done === undefined) return
    app.toast(t('discard.resolvedDone', { count: paths.length }))
    await app.loadStatus()
  }

  async ignore(path: string, byExtension: boolean) {
    const done = await app.run('discard', (repo) => api.ignorePath(repo, path, byExtension))
    if (done === undefined) return
    app.toast(t('discard.ignored'))
    await app.loadStatus()
  }

  // --- stash ----------------------------------------------------------------

  async stashSave() {
    if (app.files.length === 0) {
      app.toast(t('stash.noChanges'))
      return
    }
    const done = await app.run('stash', (repo) => api.stashSave(repo, this.summary.trim()))
    if (done === undefined) return
    app.toast(t('stash.saved'))
    await Promise.all([app.loadStatus(), this.loadStashes()])
  }

  async stashPop(index: number) {
    const outcome = await app.run('stash', (repo) => api.stashPop(repo, index))
    if (!outcome) return
    if (outcome === 'conflicts') app.toast(t('operation.stashConflicts'), 'error')
    else app.toast(t('stash.popped'))
    await app.setTab('changes')
    await Promise.all([app.loadStatus(), this.loadStashes()])
  }

  async stashDrop(entry: StashEntry) {
    if (!(await confirmDialog(t('stash.dropConfirm', { message: entry.message }), t('stash.drop')))) return
    const done = await app.run('stash', (repo) => api.stashDrop(repo, entry.index))
    if (done === undefined) return
    app.toast(t('stash.dropped'))
    await this.loadStashes()
  }
}

export const changes = new ChangesState()
