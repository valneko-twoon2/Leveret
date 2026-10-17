// Application state and actions. Components render from `app` and call its
// methods; capture scenes drive the UI through the same methods.
import { SvelteSet } from 'svelte/reactivity'
import {
  api,
  isAppError,
  type CommitDetail,
  type CommitFile,
  type FileChange,
  type FileDiff,
  type LogEntry,
  type LogScope,
  type RepoEntry,
  type RepoStatus,
} from './api'
import { t, type MessageKey } from './i18n/index.svelte'

export type Tab = 'changes' | 'history'

export interface Toast {
  id: number
  message: string
  kind: 'info' | 'error'
}

export interface ErrorInfo {
  message: string
  detail: string
}

const LOG_PAGE = 300

/** Turns any thrown value into a localized message plus raw detail. */
export function describeError(e: unknown): ErrorInfo {
  if (isAppError(e)) {
    const key = `errors.${e.code}` as MessageKey
    const message = t(key, { detail: e.detail })
    // Show the raw detail separately unless the message already embeds it.
    const detail = message.includes(e.detail) ? '' : e.detail
    return { message: message === key ? t('errors.other') : message, detail }
  }
  return { message: t('errors.other'), detail: e instanceof Error ? e.message : String(e) }
}

class AppState {
  repos = $state.raw<RepoEntry[]>([])
  selectedPath = $state<string | null>(null)
  loaded = $state(false)
  tab = $state<Tab>('changes')

  // changes
  status = $state.raw<RepoStatus | null>(null)
  /** Paths unchecked by the user; everything else is included in the commit. */
  excluded = new SvelteSet<string>()
  selectedFile = $state<string | null>(null)
  workingDiff = $state.raw<FileDiff | null>(null)
  summary = $state('')
  description = $state('')
  committing = $state(false)

  // history
  scope = $state<LogScope>('head')
  log = $state.raw<LogEntry[]>([])
  logLimit = $state(LOG_PAGE)
  logHasMore = $state(false)
  logLoading = $state(false)
  selectedCommit = $state<string | null>(null)
  detail = $state.raw<CommitDetail | null>(null)
  detailFile = $state<string | null>(null)
  commitDiff = $state.raw<FileDiff | null>(null)

  toasts = $state<Toast[]>([])
  error = $state<ErrorInfo | null>(null)
  /** Number of in-flight loads; capture waits for this to reach zero. */
  pending = $state(0)

  repo = $derived(this.repos.find((r) => r.path === this.selectedPath) ?? null)
  files = $derived(this.status?.files ?? [])
  includedFiles = $derived(this.files.filter((f) => !this.excluded.has(f.path)))
  selectedChange = $derived(this.files.find((f) => f.path === this.selectedFile) ?? null)

  private toastId = 0

  private async track<T>(work: Promise<T>): Promise<T> {
    this.pending++
    try {
      return await work
    } finally {
      this.pending--
    }
  }

  /** Runs `work`, reporting failures in the error dialog. Returns undefined on failure. */
  private async guard<T>(work: () => Promise<T>): Promise<T | undefined> {
    try {
      return await this.track(work())
    } catch (e) {
      this.error = describeError(e)
      return undefined
    }
  }

  toast(message: string, kind: Toast['kind'] = 'info') {
    const id = ++this.toastId
    this.toasts.push({ id, message, kind })
    setTimeout(() => this.dismissToast(id), 4000)
  }

  dismissToast(id: number) {
    this.toasts = this.toasts.filter((x) => x.id !== id)
  }

  async init() {
    const list = await this.guard(() => api.listRepos())
    this.loaded = true
    if (!list) return
    this.repos = list.repos
    const initial = list.repos.find((r) => r.path === list.selected) ?? list.repos[0]
    await this.selectRepo(initial?.path ?? null)
  }

  async addRepo(path: string) {
    const entry = await this.guard(() => api.addRepo(path))
    if (!entry) return
    const list = await this.guard(() => api.listRepos())
    if (list) this.repos = list.repos
    this.toast(t('repos.added', { name: entry.name }))
    await this.selectRepo(entry.path)
  }

  /** Reloads the repository list (e.g. after a clone) and selects `path`. */
  async reloadRepos(path?: string) {
    const list = await this.guard(() => api.listRepos())
    if (list) this.repos = list.repos
    if (path) await this.selectRepo(path)
  }

  async removeRepo(path: string) {
    if ((await this.guard(() => api.removeRepo(path))) === undefined) return
    this.repos = this.repos.filter((r) => r.path !== path)
    if (this.selectedPath === path) await this.selectRepo(this.repos[0]?.path ?? null)
  }

  async selectRepo(path: string | null) {
    if (path !== this.selectedPath) {
      this.selectedPath = path
      this.resetRepoView()
      void api.selectRepo(path).catch(() => {})
    }
    await this.refresh()
  }

  private resetRepoView() {
    this.status = null
    this.excluded.clear()
    this.selectedFile = null
    this.workingDiff = null
    this.summary = ''
    this.description = ''
    this.log = []
    this.logLimit = LOG_PAGE
    this.logHasMore = false
    this.selectedCommit = null
    this.detail = null
    this.detailFile = null
    this.commitDiff = null
  }

  /** Reloads status (and history when it is visible) for the current repository. */
  async refresh() {
    await Promise.all([this.loadStatus(), this.tab === 'history' ? this.loadLog() : Promise.resolve()])
  }

  async loadStatus() {
    const repo = this.selectedPath
    if (!repo) return
    const status = await this.guard(() => api.getStatus(repo))
    if (!status || repo !== this.selectedPath) return
    this.status = status
    // Forget exclusions for files that are gone.
    const present = new Set(status.files.map((f) => f.path))
    for (const p of [...this.excluded]) if (!present.has(p)) this.excluded.delete(p)
    const keep = this.selectedFile && present.has(this.selectedFile) ? this.selectedFile : (status.files[0]?.path ?? null)
    await this.selectFile(keep)
  }

  async selectFile(path: string | null) {
    this.selectedFile = path
    const repo = this.selectedPath
    const file = this.files.find((f) => f.path === path)
    if (!repo || !file) {
      this.workingDiff = null
      return
    }
    const diff = await this.guard(() => api.getWorkingDiff(repo, file))
    if (diff && this.selectedFile === path && repo === this.selectedPath) this.workingDiff = diff
  }

  toggleFile(path: string) {
    if (this.excluded.has(path)) this.excluded.delete(path)
    else this.excluded.add(path)
  }

  toggleAll() {
    if (this.includedFiles.length === this.files.length) {
      for (const f of this.files) this.excluded.add(f.path)
    } else {
      this.excluded.clear()
    }
  }

  canCommit = $derived(!this.committing && this.summary.trim() !== '' && this.includedFiles.length > 0)

  async commit() {
    const repo = this.selectedPath
    if (!repo || !this.canCommit) return
    const paths = this.includedFiles.flatMap((f: FileChange) => (f.origPath ? [f.path, f.origPath] : [f.path]))
    const summary = this.summary.trim()
    this.committing = true
    const hash = await this.guard(() => api.commit(repo, { summary, description: this.description, paths }))
    this.committing = false
    if (!hash) return
    this.summary = ''
    this.description = ''
    this.toast(t('commit.done', { summary }))
    this.logLimit = LOG_PAGE
    await Promise.all([this.loadStatus(), this.log.length || this.tab === 'history' ? this.loadLog() : Promise.resolve()])
  }

  async setTab(tab: Tab) {
    this.tab = tab
    if (tab === 'history' && this.log.length === 0) await this.loadLog()
  }

  async setScope(scope: LogScope) {
    this.scope = scope
    this.logLimit = LOG_PAGE
    await this.loadLog()
  }

  async loadLog() {
    const repo = this.selectedPath
    if (!repo) return
    this.logLoading = true
    const limit = this.logLimit
    const log = await this.guard(() => api.getLog(repo, this.scope, limit))
    this.logLoading = false
    if (!log || repo !== this.selectedPath) return
    this.log = log
    this.logHasMore = log.length >= limit
    const keep = this.selectedCommit && log.some((c) => c.hash === this.selectedCommit) ? this.selectedCommit : (log[0]?.hash ?? null)
    if (keep !== this.selectedCommit || !this.detail) await this.selectCommit(keep)
  }

  async loadMoreLog() {
    if (!this.logHasMore || this.logLoading) return
    this.logLimit += LOG_PAGE
    await this.loadLog()
  }

  async selectCommit(hash: string | null) {
    this.selectedCommit = hash
    const repo = this.selectedPath
    if (!repo || !hash) {
      this.detail = null
      this.commitDiff = null
      return
    }
    const detail = await this.guard(() => api.getCommit(repo, hash))
    if (!detail || this.selectedCommit !== hash) return
    this.detail = detail
    await this.selectCommitFile(detail.files[0]?.path ?? null)
  }

  async selectCommitFile(path: string | null) {
    this.detailFile = path
    const repo = this.selectedPath
    const detail = this.detail
    const file = detail?.files.find((f: CommitFile) => f.path === path)
    if (!repo || !detail || !file) {
      this.commitDiff = null
      return
    }
    const diff = await this.guard(() => api.getCommitDiff(repo, detail, file))
    if (diff && this.detailFile === path && this.detail === detail) this.commitDiff = diff
  }
}

export const app = new AppState()
