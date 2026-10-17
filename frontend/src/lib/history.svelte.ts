// The history tab: the commit log with paging and filtering, the selected
// commit's details and diff, and actions on a commit (revert, cherry-pick,
// checkout). Builds on `app` (state.svelte.ts).
import { api, type CommitDetail, type CommitFile, type FileDiff, type LogEntry, type LogScope } from './api'
import { confirmDialog } from './dialogs'
import { shortHash } from './format'
import { t } from './i18n/index.svelte'
import { perf } from './perf'
import { app } from './state.svelte'

/** Commits loaded per page; `resetPaging` goes back to one page. */
const LOG_PAGE = 300

class HistoryState {
  scope = $state<LogScope>('head')
  log = $state.raw<LogEntry[]>([])
  logLimit = $state(LOG_PAGE)
  logHasMore = $state(false)
  logLoading = $state(false)
  logFilter = $state('')
  selectedCommit = $state<string | null>(null)
  detail = $state.raw<CommitDetail | null>(null)
  detailFile = $state<string | null>(null)
  commitDiff = $state.raw<FileDiff | null>(null)

  filteredLog = $derived.by(() => {
    const q = this.logFilter.trim().toLowerCase()
    if (!q) return this.log
    return this.log.filter(
      (c) => c.subject.toLowerCase().includes(q) || c.authorName.toLowerCase().includes(q) || c.hash.startsWith(q),
    )
  })

  /** Forgets everything about the previous repository (called by `app.selectRepo`). */
  resetRepoView() {
    this.log = []
    this.logLimit = LOG_PAGE
    this.logHasMore = false
    this.logFilter = ''
    this.selectedCommit = null
    this.detail = null
    this.detailFile = null
    this.commitDiff = null
  }

  /** Goes back to one page of commits (called by `app.refreshAfterChange`). */
  resetPaging() {
    this.logLimit = LOG_PAGE
  }

  async setScope(scope: LogScope) {
    this.scope = scope
    this.logLimit = LOG_PAGE
    await perf.view('history.scope', () => this.loadLog())
  }

  async loadLog() {
    const repo = app.selectedPath
    if (!repo) return
    this.logLoading = true
    const limit = this.logLimit
    const log = await app.guard(() => api.getLog(repo, this.scope, limit))
    this.logLoading = false
    if (!log || repo !== app.selectedPath) return
    this.log = log
    this.logHasMore = log.length >= limit
    const keep = this.selectedCommit && log.some((c) => c.hash === this.selectedCommit) ? this.selectedCommit : (log[0]?.hash ?? null)
    if (keep !== this.selectedCommit || !this.detail) await this.selectCommit(keep)
  }

  async loadMoreLog() {
    if (!this.logHasMore || this.logLoading) return
    this.logLimit += LOG_PAGE
    await perf.view('history.more', () => this.loadLog())
  }

  async selectCommit(hash: string | null) {
    this.selectedCommit = hash
    const repo = app.selectedPath
    if (!repo || !hash) {
      this.detail = null
      this.commitDiff = null
      return
    }
    await perf.view('commit.open', async () => {
      const detail = await app.guard(() => api.getCommit(repo, hash))
      if (!detail || this.selectedCommit !== hash) return
      this.detail = detail
      await this.selectCommitFile(detail.files[0]?.path ?? null)
    })
  }

  async selectCommitFile(path: string | null) {
    this.detailFile = path
    const repo = app.selectedPath
    const detail = this.detail
    const file = detail?.files.find((f: CommitFile) => f.path === path)
    if (!repo || !detail || !file) {
      this.commitDiff = null
      return
    }
    await perf.view('diff.commit', async () => {
      const diff = await app.guard(() => api.getCommitDiff(repo, detail, file))
      if (diff && this.detailFile === path && this.detail === detail) this.commitDiff = diff
    })
  }

  // --- actions on a commit --------------------------------------------------

  async revertCommit(c: LogEntry) {
    if (!(await confirmDialog(t('history.revertConfirm', { subject: c.subject }), t('history.revert')))) return
    const outcome = await app.run('operation', (repo) => api.revertCommit(repo, c.hash))
    if (outcome) await app.handleOutcome(outcome, t('history.reverted'))
  }

  async cherryPick(c: LogEntry) {
    const outcome = await app.run('operation', (repo) => api.cherryPick(repo, c.hash))
    if (outcome) await app.handleOutcome(outcome, t('history.cherryPicked'))
  }

  async checkoutCommit(c: LogEntry) {
    const hash = shortHash(c.hash)
    if (!(await confirmDialog(t('history.checkoutConfirm', { hash }), t('history.checkout')))) return
    const done = await app.run('branch', (repo) => api.checkoutCommit(repo, c.hash))
    if (done === undefined) return
    app.toast(t('history.checkedOut', { hash }))
    await app.refreshAfterChange()
  }
}

export const history = new HistoryState()
