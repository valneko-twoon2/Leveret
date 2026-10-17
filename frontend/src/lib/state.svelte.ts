// Application state and actions. Components render from `app` and call its
// methods; capture scenes drive the UI through the same methods.
import { SvelteSet } from 'svelte/reactivity'
import {
  api,
  isAppError,
  type AgentAccess,
  type AgentActivity,
  type AgentInfo,
  type Branch,
  type CommitDetail,
  type CommitFile,
  type FileChange,
  type FileDiff,
  type LogEntry,
  type LogScope,
  type Outcome,
  type RemoteOp,
  type RemoteProgress,
  type RepoEntry,
  type RepoStatus,
  type StashEntry,
} from './api'
import { confirmDialog } from './dialogs'
import { shortHash } from './format'
import { t, type MessageKey } from './i18n/index.svelte'
import { perf } from './perf'

export type Tab = 'changes' | 'history'

/** A long-running action; while one runs, other git actions are disabled. */
export type Busy = RemoteOp | 'commit' | 'branch' | 'operation' | 'discard' | 'stash'

export interface Toast {
  id: number
  message: string
  kind: 'info' | 'error'
}

export interface ErrorInfo {
  message: string
  detail: string
}

export interface NewBranchRequest {
  /** Commit hash or full ref to start from; null for HEAD. */
  start: string | null
  /** What to show as the start point. */
  label: string | null
}

const LOG_PAGE = 300
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

/** Same rules as `security::branch_name` in the backend (git check-ref-format). */
export function isValidBranchName(name: string): boolean {
  if (!name || name.length > 255 || name === '@' || name === 'HEAD') return false
  if (name.startsWith('-') || name.endsWith('.') || name.includes('..') || name.includes('@{')) return false
  if (/[\s~^:?*[\\\u0000-\u001f\u007f]/.test(name)) return false
  return name.split('/').every((part) => part !== '' && !part.startsWith('.') && !part.endsWith('.lock'))
}

/** Local branch name a remote branch would be checked out as (`origin/feature/x` -> `feature/x`). */
export function localName(branch: Branch): string {
  return branch.kind === 'remote' ? branch.name.slice(branch.name.indexOf('/') + 1) : branch.name
}

class AppState {
  repos = $state.raw<RepoEntry[]>([])
  selectedPath = $state<string | null>(null)
  loaded = $state(false)
  tab = $state<Tab>('changes')
  busy = $state<Busy | null>(null)
  remoteProgress = $state.raw<RemoteProgress | null>(null)

  // changes
  status = $state.raw<RepoStatus | null>(null)
  /** Paths unchecked by the user; everything else is included in the commit. */
  excluded = new SvelteSet<string>()
  selectedFile = $state<string | null>(null)
  workingDiff = $state.raw<FileDiff | null>(null)
  summary = $state('')
  description = $state('')
  amend = $state(false)
  stashes = $state.raw<StashEntry[]>([])

  // history
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

  // branches
  branchesOpen = $state(false)
  branches = $state.raw<Branch[]>([])
  branchFilter = $state('')
  newBranch = $state.raw<NewBranchRequest | null>(null)
  newBranchName = $state('')
  newBranchCheckout = $state(true)

  // AI agents
  agentOpen = $state(false)
  agentInfo = $state.raw<AgentInfo | null>(null)

  /** Performance panel (Ctrl+Shift+D). */
  perfOpen = $state(false)

  toasts = $state<Toast[]>([])
  error = $state<ErrorInfo | null>(null)
  /** Number of in-flight loads; capture waits for this to reach zero. */
  pending = $state(0)

  repo = $derived(this.repos.find((r) => r.path === this.selectedPath) ?? null)
  files = $derived(this.status?.files ?? [])
  includedFiles = $derived(this.files.filter((f) => !this.excluded.has(f.path)))
  selectedChange = $derived(this.files.find((f) => f.path === this.selectedFile) ?? null)
  conflicted = $derived(this.files.filter((f) => f.kind === 'conflicted'))
  operation = $derived(this.status?.operation ?? null)
  /** A rebase, revert or cherry-pick is stopped: commits go through "continue" instead. */
  commitBlocked = $derived(this.operation !== null && this.operation.kind !== 'merge')
  /** The last commit exists and hasn't been pushed. */
  canRewriteHead = $derived(
    !!this.status?.headOid && !(this.status.upstream && this.status.ahead === 0) && this.operation === null,
  )

  filteredLog = $derived.by(() => {
    const q = this.logFilter.trim().toLowerCase()
    if (!q) return this.log
    return this.log.filter(
      (c) => c.subject.toLowerCase().includes(q) || c.authorName.toLowerCase().includes(q) || c.hash.startsWith(q),
    )
  })

  canCommit = $derived(
    this.busy === null &&
      !this.commitBlocked &&
      this.summary.trim() !== '' &&
      (this.amend ? this.canRewriteHead : this.includedFiles.length > 0),
  )

  private toastId = 0
  private progressSubscribed = false
  private autoFetchTimer: ReturnType<typeof setInterval> | null = null
  /** Message put into the form by "amend", so turning amend off can clear it again. */
  private amendPrefill: { summary: string; description: string } | null = null

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

  /**
   * Runs a git action that changes the repository. Only one runs at a time, so
   * double clicks and shortcuts can't start overlapping git commands.
   */
  private async run<T>(
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
        .onAgentActivity((entries) => void this.onAgentActivity(entries))
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
    this.excluded.clear()
    this.selectedFile = null
    this.workingDiff = null
    this.summary = ''
    this.description = ''
    this.amend = false
    this.amendPrefill = null
    this.stashes = []
    this.log = []
    this.logLimit = LOG_PAGE
    this.logHasMore = false
    this.logFilter = ''
    this.selectedCommit = null
    this.detail = null
    this.detailFile = null
    this.commitDiff = null
    this.branchesOpen = false
    this.branches = []
    this.newBranch = null
  }

  /** Reloads status (and history when it is visible) for the current repository. */
  async refresh() {
    if (!this.selectedPath) return
    await perf.view('refresh', async () => {
      await Promise.all([
        this.loadStatus(),
        this.loadStashes(),
        this.tab === 'history' ? this.loadLog() : Promise.resolve(),
        this.branchesOpen ? this.loadBranches() : Promise.resolve(),
      ])
    })
  }

  /** Refresh after an action that may have changed commits or branches. */
  private async refreshAfterChange() {
    this.logLimit = LOG_PAGE
    await Promise.all([
      this.loadStatus(),
      this.loadStashes(),
      this.log.length || this.tab === 'history' ? this.loadLog() : Promise.resolve(),
      this.branchesOpen ? this.loadBranches() : Promise.resolve(),
    ])
  }

  async loadStatus() {
    const repo = this.selectedPath
    if (!repo) return
    const status = await this.guard(() => api.getStatus(repo))
    if (!status || repo !== this.selectedPath) return
    this.status = status
    if (this.amend && !this.canRewriteHead) this.setAmend(false)
    // Forget exclusions for files that are gone.
    const present = new Set(status.files.map((f) => f.path))
    for (const p of [...this.excluded]) if (!present.has(p)) this.excluded.delete(p)
    const keep = this.selectedFile && present.has(this.selectedFile) ? this.selectedFile : (status.files[0]?.path ?? null)
    await this.selectFile(keep)
  }

  async loadStashes() {
    const repo = this.selectedPath
    if (!repo) return
    // Not worth an error dialog: the list just stays as it was.
    const stashes = await this.track(api.listStashes(repo)).catch(() => null)
    if (stashes && repo === this.selectedPath) this.stashes = stashes
  }

  async selectFile(path: string | null) {
    this.selectedFile = path
    const repo = this.selectedPath
    const file = this.files.find((f) => f.path === path)
    if (!repo || !file) {
      this.workingDiff = null
      return
    }
    await perf.view('diff.file', async () => {
      const diff = await this.guard(() => api.getWorkingDiff(repo, file))
      if (diff && this.selectedFile === path && repo === this.selectedPath) this.workingDiff = diff
    })
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

  /** Turns "amend the last commit" on (pre-filling its message) or off. */
  async setAmend(on: boolean) {
    this.amend = on
    const repo = this.selectedPath
    const head = this.status?.headOid
    if (on && repo && head && !this.summary.trim() && !this.description.trim()) {
      const detail = await this.guard(() => api.getCommit(repo, head))
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
    const hash = await this.run('commit', (repo) => api.commit(repo, { summary, description: this.description, paths, amend }))
    if (!hash) return
    this.summary = ''
    this.description = ''
    this.amend = false
    this.amendPrefill = null
    this.toast(amend ? t('commit.amended') : t('commit.done', { summary }))
    await this.refreshAfterChange()
  }

  /** Removes the last commit, keeping its changes and putting its message back into the form. */
  async undoLastCommit() {
    const message = await this.run('commit', (repo) => api.undoLastCommit(repo))
    if (message === undefined) return
    if (!this.summary.trim() && !this.description.trim()) {
      const [summary, ...rest] = message.split('\n')
      this.summary = summary
      this.description = rest.join('\n').trim()
    }
    this.toast(t('commit.undone'))
    await this.setTab('changes')
    await this.refreshAfterChange()
  }

  // --- discard, resolve, ignore ---------------------------------------------

  async discard(paths: string[]) {
    if (paths.length === 0) return
    const question = paths.length === 1 ? t('discard.confirmOne', { path: paths[0] }) : t('discard.confirmMany', { count: paths.length })
    if (!(await confirmDialog(`${question}\n${t('discard.confirmNote')}`, t('discard.title')))) return
    const done = await this.run('discard', (repo) => api.discardChanges(repo, paths))
    if (done === undefined) return
    this.toast(t('discard.done', { count: paths.length }))
    await this.loadStatus()
  }

  async markResolved(paths: string[]) {
    if (paths.length === 0) return
    const done = await this.run('operation', (repo) => api.markResolved(repo, paths))
    if (done === undefined) return
    this.toast(t('discard.resolvedDone', { count: paths.length }))
    await this.loadStatus()
  }

  async ignore(path: string, byExtension: boolean) {
    const done = await this.run('discard', (repo) => api.ignorePath(repo, path, byExtension))
    if (done === undefined) return
    this.toast(t('discard.ignored'))
    await this.loadStatus()
  }

  async showInExplorer(path?: string) {
    const repo = this.selectedPath
    if (repo) await this.guard(() => api.showInExplorer(repo, path))
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

  private async handleOutcome(outcome: Outcome, doneMessage: string) {
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

  async revertCommit(c: LogEntry) {
    if (!(await confirmDialog(t('history.revertConfirm', { subject: c.subject }), t('history.revert')))) return
    const outcome = await this.run('operation', (repo) => api.revertCommit(repo, c.hash))
    if (outcome) await this.handleOutcome(outcome, t('history.reverted'))
  }

  async cherryPick(c: LogEntry) {
    const outcome = await this.run('operation', (repo) => api.cherryPick(repo, c.hash))
    if (outcome) await this.handleOutcome(outcome, t('history.cherryPicked'))
  }

  async checkoutCommit(c: LogEntry) {
    const hash = shortHash(c.hash)
    if (!(await confirmDialog(t('history.checkoutConfirm', { hash }), t('history.checkout')))) return
    const done = await this.run('branch', (repo) => api.checkoutCommit(repo, c.hash))
    if (done === undefined) return
    this.toast(t('history.checkedOut', { hash }))
    await this.refreshAfterChange()
  }

  // --- stash ----------------------------------------------------------------

  async stashSave() {
    if (this.files.length === 0) {
      this.toast(t('stash.noChanges'))
      return
    }
    const done = await this.run('stash', (repo) => api.stashSave(repo, this.summary.trim()))
    if (done === undefined) return
    this.toast(t('stash.saved'))
    await Promise.all([this.loadStatus(), this.loadStashes()])
  }

  async stashPop(index: number) {
    const outcome = await this.run('stash', (repo) => api.stashPop(repo, index))
    if (!outcome) return
    if (outcome === 'conflicts') this.toast(t('operation.stashConflicts'), 'error')
    else this.toast(t('stash.popped'))
    await this.setTab('changes')
    await Promise.all([this.loadStatus(), this.loadStashes()])
  }

  async stashDrop(entry: StashEntry) {
    if (!(await confirmDialog(t('stash.dropConfirm', { message: entry.message }), t('stash.drop')))) return
    const done = await this.run('stash', (repo) => api.stashDrop(repo, entry.index))
    if (done === undefined) return
    this.toast(t('stash.dropped'))
    await this.loadStashes()
  }

  // --- branches -------------------------------------------------------------

  async openBranches() {
    if (!this.selectedPath) return
    this.branchFilter = ''
    this.branchesOpen = true
    await perf.view('branches.open', () => this.loadBranches())
  }

  closeBranches() {
    this.branchesOpen = false
  }

  async loadBranches() {
    const repo = this.selectedPath
    if (!repo) return
    const branches = await this.guard(() => api.listBranches(repo))
    if (branches && repo === this.selectedPath) this.branches = branches
  }

  async switchBranch(b: Branch) {
    if (b.current) {
      this.closeBranches()
      return
    }
    const done = await this.run('branch', (repo) => api.switchBranch(repo, b.fullRef))
    if (done === undefined) return
    this.closeBranches()
    this.toast(t('branches.switched', { branch: localName(b) }))
    await this.refreshAfterChange()
  }

  async mergeBranch(b: Branch) {
    const current = this.status?.branch ?? t('repo.detached')
    if (!(await confirmDialog(t('branches.mergeConfirm', { branch: b.name, current }), t('branches.merge')))) return
    const outcome = await this.run('operation', (repo) => api.mergeBranch(repo, b.fullRef))
    if (!outcome) return
    this.closeBranches()
    await this.handleOutcome(outcome, t('branches.merged', { branch: b.name }))
  }

  async rebaseOnto(b: Branch) {
    const current = this.status?.branch ?? t('repo.detached')
    if (!(await confirmDialog(t('branches.rebaseConfirm', { branch: b.name, current }), t('branches.rebase')))) return
    const outcome = await this.run('operation', (repo) => api.rebaseBranch(repo, b.fullRef))
    if (!outcome) return
    this.closeBranches()
    await this.handleOutcome(outcome, t('branches.rebased', { branch: b.name }))
  }

  async deleteBranch(b: Branch) {
    if (b.kind !== 'local' || b.current) return
    if (!(await confirmDialog(t('branches.deleteConfirm', { branch: b.name }), t('branches.delete')))) return
    const repo = this.selectedPath
    if (!repo || this.busy !== null) return
    this.busy = 'branch'
    try {
      try {
        await this.track(api.deleteBranch(repo, b.name, false))
      } catch (e) {
        if (!isAppError(e) || e.code !== 'branchNotMerged') throw e
        if (!(await confirmDialog(t('branches.forceDeleteConfirm', { branch: b.name }), t('branches.delete')))) return
        await this.track(api.deleteBranch(repo, b.name, true))
      }
      this.toast(t('branches.deleted', { branch: b.name }))
    } catch (e) {
      this.error = describeError(e)
    } finally {
      this.busy = null
    }
    await this.refreshAfterChange()
  }

  /** Opens the "new branch" dialog, starting from HEAD or from `start`. */
  openNewBranch(start: string | null = null, label: string | null = null) {
    if (!this.selectedPath) return
    this.newBranchName = ''
    this.newBranchCheckout = true
    this.newBranch = { start, label }
  }

  closeNewBranch() {
    this.newBranch = null
  }

  newBranchError = $derived.by(() => {
    const name = this.newBranchName.trim()
    if (!name) return null
    if (!isValidBranchName(name)) return t('newBranch.invalid')
    if (this.branches.some((b) => b.kind === 'local' && b.name === name)) return t('newBranch.exists')
    return null
  })

  async createBranch() {
    const req = this.newBranch
    const name = this.newBranchName.trim()
    if (!req || !name || this.newBranchError) return
    const checkout = this.newBranchCheckout
    const done = await this.run('branch', (repo) => api.createBranch(repo, name, req.start, checkout))
    if (done === undefined) return
    this.newBranch = null
    this.closeBranches()
    this.toast(t('newBranch.created', { branch: name }))
    await this.refreshAfterChange()
  }

  // --- AI agents ------------------------------------------------------------

  async openAgent() {
    this.agentOpen = true
    await perf.view('agent.open', () => this.loadAgentInfo())
  }

  closeAgent() {
    this.agentOpen = false
  }

  async loadAgentInfo() {
    const info = await this.guard(() => api.agentInfo())
    if (info) this.agentInfo = info
  }

  async setAgentAccess(access: AgentAccess) {
    if ((await this.guard(() => api.setAgentAccess(access))) === undefined) return
    const key = { off: 'agent.accessOff', readOnly: 'agent.accessReadOnly', standard: 'agent.accessStandard', full: 'agent.accessFull' } as const
    this.toast(t('agent.accessSaved', { level: t(key[access]) }))
    await this.loadAgentInfo()
  }

  /** An agent changed something (usually through `leveret --mcp`): tell the user and show the result. */
  private async onAgentActivity(entries: AgentActivity[]) {
    for (const a of entries.slice(-3)) {
      const params = { client: a.client, tool: a.tool }
      this.toast(a.ok ? t('agent.didTool', params) : t('agent.toolFailed', params), a.ok ? 'info' : 'error')
    }
    if (this.agentOpen) void this.loadAgentInfo()
    // Agents may have registered repositories or changed the current one.
    if (this.busy === null) {
      await this.reloadRepos()
      await this.refreshAfterChange()
    }
  }

  // --- history --------------------------------------------------------------

  async setTab(tab: Tab) {
    await perf.view(`tab.${tab}`, async () => {
      this.tab = tab
      if (tab === 'history' && this.log.length === 0) await this.loadLog()
    })
  }

  async setScope(scope: LogScope) {
    this.scope = scope
    this.logLimit = LOG_PAGE
    await perf.view('history.scope', () => this.loadLog())
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
    await perf.view('history.more', () => this.loadLog())
  }

  async selectCommit(hash: string | null) {
    this.selectedCommit = hash
    const repo = this.selectedPath
    if (!repo || !hash) {
      this.detail = null
      this.commitDiff = null
      return
    }
    await perf.view('commit.open', async () => {
      const detail = await this.guard(() => api.getCommit(repo, hash))
      if (!detail || this.selectedCommit !== hash) return
      this.detail = detail
      await this.selectCommitFile(detail.files[0]?.path ?? null)
    })
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
    await perf.view('diff.commit', async () => {
      const diff = await this.guard(() => api.getCommitDiff(repo, detail, file))
      if (diff && this.detailFile === path && this.detail === detail) this.commitDiff = diff
    })
  }
}

export const app = new AppState()
