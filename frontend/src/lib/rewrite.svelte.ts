// Rewriting from the History tab: resetting the current branch to a commit (with
// a confirmation dialog) and restoring one file from a commit. Builds on `app`.
import { api, type CommitDetail, type CommitFile, type LogEntry, type ResetInfo, type ResetMode, type RestoreMode } from './api'
import { confirmDialog } from './dialogs'
import { shortHash } from './format'
import { t } from './i18n/index.svelte'
import { perf } from './perf'
import { app } from './state.svelte'

class RewriteState {
  /** The commit the branch is about to be reset to (the dialog is open). */
  reset = $state.raw<{ entry: LogEntry; info: ResetInfo } | null>(null)

  async openReset(entry: LogEntry) {
    const repo = app.selectedPath
    if (!repo) return
    await perf.view('reset.open', async () => {
      const info = await app.guard(() => api.resetInfo(repo, entry.hash))
      if (info && repo === app.selectedPath) this.reset = { entry, info }
    })
  }

  closeReset() {
    this.reset = null
  }

  async resetBranch(mode: ResetMode) {
    const target = this.reset?.entry
    if (!target) return
    this.reset = null
    const done = await app.run('branch', (repo) => api.resetBranch(repo, target.hash, mode))
    if (done === undefined) return
    app.toast(t('reset.done', { hash: shortHash(target.hash) }))
    await app.refreshAfterChange()
  }

  /** Puts a file back to a commit's version, or undoes the commit's change to it. */
  async restoreFile(detail: CommitDetail, file: CommitFile, mode: RestoreMode) {
    // A file the commit deleted comes back as it was before (in the parent).
    const hash = mode === 'version' && file.kind === 'deleted' ? detail.parents[0] : detail.hash
    if (!hash) return
    const path = mode === 'version' && file.kind === 'deleted' ? (file.origPath ?? file.path) : file.path
    const question = t(mode === 'version' ? 'restore.versionConfirm' : 'restore.undoConfirm', { path, hash: shortHash(detail.hash) })
    const title = t(mode === 'version' ? (file.kind === 'deleted' ? 'restore.versionBefore' : 'restore.version') : 'restore.undo')
    if (!(await confirmDialog(question, title))) return
    const done = await app.run('discard', (repo) => api.restoreFile(repo, hash, { ...file, path }, mode))
    if (done === undefined) return
    app.toast(t('restore.done', { path }))
    await app.loadStatus()
  }
}

export const rewrite = new RewriteState()
