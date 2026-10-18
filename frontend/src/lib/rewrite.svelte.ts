// Rewriting from the History tab: interactive rebase (reorder, reword, squash,
// drop), resetting the current branch to a commit (with a confirmation dialog)
// and restoring one file from a commit. Builds on `app`.
import {
  api,
  type CommitDetail,
  type CommitFile,
  type LogEntry,
  type PlanCommit,
  type RebaseAction,
  type ResetInfo,
  type ResetMode,
  type RestoreMode,
} from './api'
import { confirmDialog } from './dialogs'
import { shortHash } from './format'
import { t } from './i18n/index.svelte'
import { perf } from './perf'
import { app } from './state.svelte'

/** A commit in the rebase dialog: newest first, as in the history. */
export interface RebaseRow {
  commit: PlanCommit
  action: RebaseAction
  /** The message for `reword` (starts as the current one). */
  message: string
}

class RewriteState {
  /** The interactive rebase being planned (the dialog is open). */
  rebase = $state<{ base: string | null; rows: RebaseRow[] } | null>(null)

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

  /** Plans an interactive rebase of `from` and every commit after it. */
  async openRebase(from: LogEntry) {
    const repo = app.selectedPath
    if (!repo) return
    const base = from.parents[0] ?? null
    await perf.view('rebase.open', async () => {
      const plan = await app.guard(() => api.rebasePlan(repo, base))
      if (!plan || repo !== app.selectedPath) return
      const rows = [...plan].reverse().map((commit): RebaseRow => ({ commit, action: 'pick', message: commit.message }))
      this.rebase = { base, rows }
    })
  }

  closeRebase() {
    this.rebase = null
  }

  moveRow(index: number, delta: number) {
    const rows = this.rebase?.rows
    const to = index + delta
    if (!rows || to < 0 || to >= rows.length) return
    const [row] = rows.splice(index, 1)
    rows.splice(to, 0, row)
  }

  setAction(index: number, action: RebaseAction) {
    const row = this.rebase?.rows[index]
    if (row) row.action = action
  }

  setMessage(index: number, message: string) {
    const row = this.rebase?.rows[index]
    if (row) row.message = message
  }

  async runRebase() {
    const plan = this.rebase
    if (!plan) return
    // Git applies them oldest first.
    const steps = [...plan.rows]
      .reverse()
      .map((r) => ({ hash: r.commit.hash, action: r.action, message: r.action === 'reword' ? r.message : null }))
    this.rebase = null
    const outcome = await app.run('operation', (repo) => api.interactiveRebase(repo, plan.base, steps))
    if (outcome) await app.handleOutcome(outcome, t('rebase.done'))
    // A rebase can also stop without conflicts (e.g. a hook rejected a message).
    else await app.refreshAfterChange()
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
