// The history tab: the commit log with paging, searching (in git, not just the
// loaded commits) and a file's history, the selected commit's details and diff,
// and actions on a commit (revert, cherry-pick, checkout). Builds on `app`
// (state.svelte.ts).
import { api, type CommitDetail, type CommitFile, type FileDiff, type LogEntry, type LogQuery, type LogScope } from './api'
import { confirmDialog } from './dialogs'
import { shortHash } from './format'
import { t } from './i18n/index.svelte'
import { perf } from './perf'
import { app } from './state.svelte'

/** Commits loaded per page; `resetPaging` goes back to one page. */
const LOG_PAGE = 300
/** Typing pauses this long before the search runs. */
const SEARCH_DELAY_MS = 350

/** What the search box looks in. */
export type SearchField = 'message' | 'author' | 'content' | 'path'

class HistoryState {
  scope = $state<LogScope>('head')
  log = $state.raw<LogEntry[]>([])
  logLimit = $state(LOG_PAGE)
  logHasMore = $state(false)
  logLoading = $state(false)
  /** The search box. */
  logFilter = $state('')
  searchField = $state<SearchField>('message')
  /** The file whose history is shown (the log is limited to it). */
  file = $state<string | null>(null)
  selectedCommit = $state<string | null>(null)
  detail = $state.raw<CommitDetail | null>(null)
  detailFile = $state<string | null>(null)
  commitDiff = $state.raw<FileDiff | null>(null)

  /** The search in effect, or null for the whole log (with its graph). */
  query = $derived.by<LogQuery | null>(() => {
    const text = this.logFilter.trim()
    if (!text && !this.file) return null
    const q: LogQuery = this.file ? { file: this.file } : {}
    if (text) q[this.searchField] = text
    return q
  })

  private searchTimer: ReturnType<typeof setTimeout> | null = null

  /** Forgets everything about the previous repository (called by `app.selectRepo`). */
  resetRepoView() {
    this.log = []
    this.logLimit = LOG_PAGE
    this.logHasMore = false
    this.logFilter = ''
    this.file = null
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
    const query = this.query
    const log = await app.guard(() => api.getLog(repo, this.scope, limit, query))
    this.logLoading = false
    // A newer search may have started meanwhile.
    if (!log || repo !== app.selectedPath || query !== this.query) return
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

  /** The search box changed: searches once typing pauses (at once with `now`). */
  setFilter(text: string, now = false) {
    this.logFilter = text
    if (this.searchTimer) clearTimeout(this.searchTimer)
    const run = () => {
      this.searchTimer = null
      this.logLimit = LOG_PAGE
      return perf.view('history.search', () => this.loadLog())
    }
    if (now) return run()
    this.searchTimer = setTimeout(() => void run(), SEARCH_DELAY_MS)
  }

  async setSearchField(field: SearchField) {
    this.searchField = field
    if (this.logFilter.trim()) await this.setFilter(this.logFilter, true)
  }

  /** Shows the history of one file (from any file list). */
  async showFileHistory(path: string) {
    this.file = path
    this.logLimit = LOG_PAGE
    this.selectedCommit = null
    if (app.tab !== 'history') await app.setTab('history')
    else await perf.view('history.file', () => this.loadLog())
  }

  async closeFileHistory() {
    this.file = null
    this.logLimit = LOG_PAGE
    await perf.view('history.scope', () => this.loadLog())
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
      // In a file's history, open that file (under its name in this commit).
      const wanted = this.log.find((c) => c.hash === hash)?.filePath ?? this.file
      const file = detail.files.find((f) => f.path === wanted) ?? detail.files[0]
      await this.selectCommitFile(file?.path ?? null)
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

  /** Shows the selected file's diff again (after a diff setting changed). */
  async reloadCommitFile() {
    if (this.detailFile) await this.selectCommitFile(this.detailFile)
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
