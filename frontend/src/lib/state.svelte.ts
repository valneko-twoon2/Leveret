// Application state and actions. Components render from `app` and call its
// methods; capture scenes drive the UI through the same methods.
//
// `app` holds what every screen shares (repositories, selection, status, busy
// state, toasts, errors) plus remote actions and stopped operations. The tabs and
// dialogs have their own stores that build on it: changes.svelte.ts,
// history.svelte.ts, issues.svelte.ts, branches.svelte.ts and agents.svelte.ts.
import { api, isAppError, type Outcome, type RemoteOp, type RemoteProgress, type RepoEntry, type RepoStatus } from './api'
import { agents } from './agents.svelte'
import { branches } from './branches.svelte'
import { changes } from './changes.svelte'
import { confirmDialog } from './dialogs'
import { history } from './history.svelte'
import { issues } from './issues.svelte'
import { tags } from './tags.svelte'
import { t, type MessageKey } from './i18n/index.svelte'
import { perf } from './perf'
import { prefs } from './prefs.svelte'

export type Tab = 'changes' | 'history' | 'issues'

/** A long-running action; while one runs, other git actions are disabled. */
export type Busy = RemoteOp | 'commit' | 'branch' | 'operation' | 'discard' | 'stash' | 'tag'

export interface Toast {
  id: number
  message: string
  kind: 'info' | 'error'
}

export interface ErrorInfo {
  message: string
  detail: string
}

/** Fetch in the background when the last fetch is older than this. */
const AUTO_FETCH_AFTER_MS = 10 * 60 * 1000
const AUTO_FETCH_CHECK_MS = 60 * 1000

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
  busy = $state<Busy | null>(null)
  remoteProgress = $state.raw<RemoteProgress | null>(null)
  status = $state.raw<RepoStatus | null>(null)

  /** Performance panel (Ctrl+Shift+D). */
  perfOpen = $state(false)

  toasts = $state<Toast[]>([])
  error = $state<ErrorInfo | null>(null)
  /** Number of in-flight loads; capture waits for this to reach zero. */
  pending = $state(0)

  repo = $derived(this.repos.find((r) => r.path === this.selectedPath) ?? null)
  /** Changed files, untracked ones last (git lists them last too; this keeps the mock and agents' changes in line). */
  files = $derived.by(() => {
    const files = this.status?.files ?? []
    return [...files.filter((f) => f.kind !== 'untracked'), ...files.filter((f) => f.kind === 'untracked')]
  })
  conflicted = $derived(this.files.filter((f) => f.kind === 'conflicted'))
  operation = $derived(this.status?.operation ?? null)
  /** A rebase, revert or cherry-pick is stopped: commits go through "continue" instead. */
  commitBlocked = $derived(this.operation !== null && this.operation.kind !== 'merge')
  /** The last commit exists and hasn't been pushed. */
  canRewriteHead = $derived(
    !!this.status?.headOid && !(this.status.upstream && this.status.ahead === 0) && this.operation === null,
  )

  private toastId = 0
  private progressSubscribed = false
  private autoFetchTimer: ReturnType<typeof setInterval> | null = null

  /** Counts `work` as an in-flight load. Used by the other stores too. */
  async track<T>(work: Promise<T>): Promise<T> {
    this.pending++
    try {
      return await work
    } finally {
      this.pending--
    }
  }

  /** Runs `work`, reporting failures in the error dialog. Returns undefined on failure. */
  async guard<T>(work: () => Promise<T>): Promise<T | undefined> {
    try {
      return await this.track(work())
    } catch (e) {
      this.error = describeError(e)
      return undefined
    }
  }

  /**
   * Runs a git action that changes the repository. Only one runs at a time, so
   * double clicks and shortcuts can't start overlapping git commands.
   * Every store's repository actions go through here.
   */
  async run<T>(
    busy: Busy,
    work: (repo: string) => Promise<T>,
    options: { quiet?: boolean; name?: string } = {},
  ): Promise<T | undefined> {
    const repo = this.selectedPath
    if (!repo || this.busy !== null) return undefined
    this.busy = busy
    try {
      return await this.track(perf.time('action', options.name ?? busy, () => work(repo)))
    } catch (e) {
      if (!options.quiet) this.error = describeError(e)
      return undefined
    } finally {
      this.busy = null
      this.remoteProgress = null
    }
  }

  toast(message: string, kind: Toast['kind'] = 'info') {
    const id = ++this.toastId
    this.toasts.push({ id, message, kind })
    setTimeout(() => this.dismissToast(id), kind === 'error' ? 8000 : 4000)
  }

  dismissToast(id: number) {
    this.toasts = this.toasts.filter((x) => x.id !== id)
  }

  async copy(text: string) {
    try {
      await navigator.clipboard.writeText(text)
      this.toast(t('common.copied'))
    } catch (e) {
      this.error = describeError(e)
    }
  }

  async init() {
    if (!this.progressSubscribed) {
      this.progressSubscribed = true
      void api
        .onRemoteProgress((p) => {
          if (p.repo === this.selectedPath && this.busy === p.operation) this.remoteProgress = p
        })
        .catch(() => {})
      void api
        .onAgentActivity((entries) => void agents.onAgentActivity(entries))
        .catch(() => {})
    }
    const list = await this.guard(() => api.listRepos())
    this.loaded = true
    if (!list) return
    this.repos = list.repos
    const initial = list.repos.find((r) => r.path === list.selected) ?? list.repos[0]
    await this.selectRepo(initial?.path ?? null)
  }

  /** Fetches in the background now and then, so ahead/behind counts stay current. */
  startAutoFetch() {
    if (this.autoFetchTimer) return
    this.autoFetchTimer = setInterval(() => {
      const st = this.status
      if (!st?.hasRemote || this.busy !== null || document.visibilityState !== 'visible') return
      if (st.lastFetched !== null && Date.now() - st.lastFetched * 1000 < AUTO_FETCH_AFTER_MS) return
      void this.remote('fetch', { quiet: true })
    }, AUTO_FETCH_CHECK_MS)
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
    const open = async () => {
      if (path !== this.selectedPath) {
        this.selectedPath = path
        this.resetRepoView()
        void api.selectRepo(path).catch(() => {})
      }
      await this.refresh()
    }
    await (path ? perf.view('repo.open', open) : open())
  }

  private resetRepoView() {
    this.status = null
    changes.resetRepoView()
    history.resetRepoView()
    branches.resetRepoView()
    issues.resetRepoView()
    tags.resetRepoView()
  }

  /** Reloads status (and history when it is visible) for the current repository. */
  async refresh() {
    if (!this.selectedPath) return
    await perf.view('refresh', async () => {
      await Promise.all([
        this.loadStatus(),
        changes.loadStashes(),
        this.tab === 'history' ? history.loadLog() : Promise.resolve(),
        branches.branchesOpen ? branches.loadBranches() : Promise.resolve(),
        // Issues reload only after a repository switch cleared them (GitHub rate limits).
        this.tab === 'issues' && !issues.loaded ? issues.load() : Promise.resolve(),
      ])
    })
  }

  /** Refresh after an action that may have changed commits or branches. Used by the other stores too. */
  async refreshAfterChange() {
    history.resetPaging()
    await Promise.all([
      this.loadStatus(),
      changes.loadStashes(),
      history.log.length || this.tab === 'history' ? history.loadLog() : Promise.resolve(),
      branches.branchesOpen ? branches.loadBranches() : Promise.resolve(),
    ])
  }

  async loadStatus() {
    const repo = this.selectedPath
    if (!repo) return
    const status = await this.guard(() => api.getStatus(repo, prefs.showUntracked))
    if (!status || repo !== this.selectedPath) return
    this.status = status
    await changes.syncWithStatus(status)
  }

  async showInExplorer(path?: string) {
    const repo = this.selectedPath
    if (repo) await this.guard(() => api.showInExplorer(repo, path))
  }

  async setTab(tab: Tab) {
    await perf.view(`tab.${tab}`, async () => {
      this.tab = tab
      if (tab === 'history' && history.log.length === 0) await history.loadLog()
      if (tab === 'issues' && !issues.loaded) await issues.load()
    })
  }

  // --- remote ---------------------------------------------------------------

  async remote(op: RemoteOp, options: { quiet?: boolean } = {}) {
    const branch = this.status?.branch
    const publishing = (op === 'push' || op === 'sync') && !this.status?.upstream
    const outcome = await this.run(op, (repo) => api.remote(repo, op), { ...options, name: `remote.${op}` })
    if (outcome === undefined) {
      // A failed quiet fetch still updates nothing; a failed visible one showed its error.
      return
    }
    if (outcome === 'conflicts') {
      await this.handleOutcome(outcome, '')
      return
    }
    if (!options.quiet) {
      const done = { fetch: t('remote.fetched'), pull: t('remote.pulled'), push: t('remote.pushed'), sync: t('remote.synced') }[op]
      this.toast(publishing && branch ? t('remote.published', { branch }) : done)
    }
    await this.refreshAfterChange()
  }

  // --- operations -----------------------------------------------------------

  /** Reports how an operation ended and refreshes. Used by the other stores too. */
  async handleOutcome(outcome: Outcome, doneMessage: string) {
    if (outcome === 'conflicts') {
      this.toast(t('operation.stoppedOnConflicts'), 'error')
      await this.setTab('changes')
    } else if (doneMessage) {
      this.toast(doneMessage)
    }
    await this.refreshAfterChange()
  }

  operationName(): string {
    return this.operation ? t(`operation.${this.operation.kind}`) : ''
  }

  async continueOperation() {
    const outcome = await this.run('operation', (repo) => api.continueOperation(repo))
    if (outcome) await this.handleOutcome(outcome, t('operation.completed'))
  }

  async skipOperation() {
    const outcome = await this.run('operation', (repo) => api.skipOperation(repo))
    if (outcome) await this.handleOutcome(outcome, t('operation.completed'))
  }

  async abortOperation() {
    const name = this.operationName()
    if (!(await confirmDialog(t('operation.abortConfirm', { operation: name }), t('operation.abort')))) return
    const done = await this.run('operation', (repo) => api.abortOperation(repo))
    if (done === undefined) return
    this.toast(t('operation.aborted', { operation: name }))
    await this.refreshAfterChange()
  }
}

export const app = new AppState()
