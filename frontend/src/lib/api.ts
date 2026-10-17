// Typed wrappers around the backend commands. Outside of Tauri (plain browser,
// e.g. `scripts/preview.bat`) or with `--mock`, calls go to the in-memory mock.
import { invoke, isTauri } from '@tauri-apps/api/core'
import { perf, type PerfSample } from './perf'

export type ChangeKind =
  | 'modified'
  | 'added'
  | 'deleted'
  | 'renamed'
  | 'copied'
  | 'typeChanged'
  | 'untracked'
  | 'conflicted'

export interface FileChange {
  path: string
  origPath: string | null
  kind: ChangeKind
}

export type OperationKind = 'merge' | 'rebase' | 'revert' | 'cherryPick'

/** A merge, rebase, revert or cherry-pick that stopped (usually on conflicts). */
export interface Operation {
  kind: OperationKind
  /** Rebase: the branch being rebased. */
  branch: string | null
  /** Rebase: current step / number of steps. */
  step: number | null
  total: number | null
}

/** Result of a command that may stop on conflicts. */
export type Outcome = 'done' | 'conflicts'

export interface RepoStatus {
  branch: string | null
  headOid: string | null
  upstream: string | null
  ahead: number
  behind: number
  operation: Operation | null
  hasRemote: boolean
  /** Unix seconds of the last fetch. */
  lastFetched: number | null
  files: FileChange[]
}

export interface Branch {
  /** `main`, `origin/main` */
  name: string
  /** `refs/heads/main`, `refs/remotes/origin/main` */
  fullRef: string
  kind: 'local' | 'remote'
  current: boolean
  upstream: string | null
  upstreamGone: boolean
  ahead: number
  behind: number
  oid: string
  time: number
  subject: string
}

export interface StashEntry {
  index: number
  message: string
  time: number
}

export type RemoteOp = 'fetch' | 'pull' | 'push' | 'sync'

export interface RemoteProgress {
  repo: string
  operation: RemoteOp
  phase: string
  percent: number
}

export interface RepoEntry {
  path: string
  name: string
}

export interface RepoList {
  repos: RepoEntry[]
  selected: string | null
}

export type LineKind = 'context' | 'add' | 'delete' | 'noNewline'

export interface DiffLine {
  kind: LineKind
  oldNo: number | null
  newNo: number | null
  text: string
}

export interface Hunk {
  header: string
  lines: DiffLine[]
}

export interface FileDiff {
  binary: boolean
  tooLarge: boolean
  truncated: boolean
  hunks: Hunk[]
}

export type RefKind = 'head' | 'branch' | 'remote' | 'tag'

export interface RefLabel {
  name: string
  kind: RefKind
  current: boolean
}

export interface Edge {
  from: number
  to: number
  kind: 'up' | 'down'
  color: number
}

export interface GraphRow {
  col: number
  color: number
  width: number
  edges: Edge[]
}

export interface LogEntry {
  hash: string
  parents: string[]
  authorName: string
  authorEmail: string
  authorTime: number
  refs: RefLabel[]
  subject: string
  graph: GraphRow
}

export type LogScope = 'head' | 'all'

export interface CommitFile {
  path: string
  origPath: string | null
  kind: ChangeKind
}

export interface CommitDetail {
  hash: string
  parents: string[]
  authorName: string
  authorEmail: string
  authorTime: number
  committerName: string
  committerEmail: string
  commitTime: number
  message: string
  files: CommitFile[]
}

export interface CommitRequest {
  summary: string
  description: string
  paths: string[]
  /** Replace the last commit. */
  amend?: boolean
}

/** What AI agents may do (backend/src/agent/mod.rs). */
export type AgentAccess = 'off' | 'readOnly' | 'standard' | 'full'
export type AgentLevel = 'read' | 'write' | 'destructive'

export interface AgentTool {
  name: string
  title: string
  level: AgentLevel
}

/** One change made by an AI agent. */
export interface AgentActivity {
  /** Unix milliseconds. */
  time: number
  client: string
  tool: string
  repo: string | null
  ok: boolean
  message: string
}

export interface AgentInfo {
  /** This executable; MCP clients run `<exePath> --mcp`. */
  exePath: string
  access: AgentAccess
  tools: AgentTool[]
  activity: AgentActivity[]
}

export interface AgentCallResult {
  text: string
  isError: boolean
}

export interface CaptureRequest {
  dir: string
  scenes: string[] | null
}

/** `--bench <dir> --runs N` (scripts/benchmark.ps1). */
export interface BenchRequest {
  dir: string
  runs: number
}

export interface PerfSnapshot {
  samples: PerfSample[]
  /** Milliseconds since the backend process started. */
  uptimeMs: number
}

/** Command line options (`--mock`, `--theme light|dark`, `--capture <dir>`, `--scenes a,b`, `--bench <dir>`). */
export interface StartupOptions {
  mock: boolean
  theme: 'light' | 'dark' | null
  capture: CaptureRequest | null
  bench: BenchRequest | null
  /** `LEVERET_PERF=1`: forward timings to the backend's perf.jsonl. */
  perfLog: boolean
  /** Devtools explicitly enabled (debug builds with LEVERET_DEVTOOLS=1). */
  devtools: boolean
}

export interface GithubAccount {
  login: string
  name: string | null
  htmlUrl: string
  /** data: URL */
  avatar: string | null
}

export interface GithubState {
  account: GithubAccount | null
  /** Whether sign-in through the browser (OAuth device flow) is available. */
  deviceFlow: boolean
}

export interface DeviceCode {
  deviceCode: string
  userCode: string
  verificationUri: string
  expiresIn: number
  interval: number
}

export type PollResult =
  | { status: 'pending' }
  | { status: 'slowDown'; interval: number }
  | { status: 'denied' }
  | { status: 'expired' }
  | { status: 'success'; account: GithubAccount }

export interface GithubRepo {
  fullName: string
  owner: string
  name: string
  description: string | null
  private: boolean
  fork: boolean
  cloneUrl: string
  htmlUrl: string
  pushedAt: string | null
}

export interface CloneProgress {
  phase: string
  percent: number
}

/** Error shape returned by the backend (see backend/src/error.rs). */
export interface AppError {
  code: string
  detail: string
}

export function isAppError(e: unknown): e is AppError {
  return typeof e === 'object' && e !== null && 'code' in e && 'detail' in e
}

export const inTauri = isTauri()
let mock: typeof import('./mock/index') | null = null

/** Routes all following calls to the mock backend. */
export async function useMockBackend(): Promise<void> {
  mock ??= await import('./mock/index')
}

export function usingMock(): boolean {
  return mock !== null
}

/** Commands that are part of the measuring itself. */
const UNTIMED = new Set(['perf_samples', 'perf_report'])

/** Calls a backend command, timing the round trip. */
function call<T>(cmd: string, args: Record<string, unknown> = {}): Promise<T> {
  const send = () => (mock ? (mock.handle(cmd, args) as Promise<T>) : invoke<T>(cmd, args))
  return UNTIMED.has(cmd) ? send() : perf.time('ipc', cmd, send)
}

/** Pulls the backend's new timing samples into `perf`. Returns the backend uptime. */
export async function syncBackendPerf(): Promise<number> {
  const snapshot = await call<PerfSnapshot>('perf_samples', { after: perf.lastSeq })
  perf.addBackend(snapshot.samples)
  return snapshot.uptimeMs
}

export const api = {
  listRepos: () => call<RepoList>('list_repos'),
  addRepo: (path: string) => call<RepoEntry>('add_repo', { path }),
  removeRepo: (path: string) => call<void>('remove_repo', { path }),
  selectRepo: (path: string | null) => call<void>('select_repo', { path }),

  getStatus: (repo: string) => call<RepoStatus>('get_status', { repo }),
  getWorkingDiff: (repo: string, file: FileChange) =>
    call<FileDiff>('get_working_diff', {
      repo,
      path: file.path,
      origPath: file.origPath,
      untracked: file.kind === 'untracked',
    }),
  commit: (repo: string, request: CommitRequest) => call<string>('commit', { repo, request }),

  discardChanges: (repo: string, paths: string[]) => call<void>('discard_changes', { repo, paths }),
  markResolved: (repo: string, paths: string[]) => call<void>('mark_resolved', { repo, paths }),
  ignorePath: (repo: string, path: string, byExtension: boolean) => call<void>('ignore_path', { repo, path, byExtension }),
  showInExplorer: (repo: string, path?: string) => call<void>('show_in_explorer', { repo, path: path ?? null }),
  undoLastCommit: (repo: string) => call<string>('undo_last_commit', { repo }),

  revertCommit: (repo: string, hash: string) => call<Outcome>('revert_commit', { repo, hash }),
  cherryPick: (repo: string, hash: string) => call<Outcome>('cherry_pick', { repo, hash }),
  rebaseBranch: (repo: string, onto: string) => call<Outcome>('rebase_branch', { repo, onto }),
  mergeBranch: (repo: string, branch: string) => call<Outcome>('merge_branch', { repo, branch }),
  continueOperation: (repo: string) => call<Outcome>('continue_operation', { repo }),
  skipOperation: (repo: string) => call<Outcome>('skip_operation', { repo }),
  abortOperation: (repo: string) => call<void>('abort_operation', { repo }),

  listBranches: (repo: string) => call<Branch[]>('list_branches', { repo }),
  createBranch: (repo: string, name: string, start: string | null, checkout: boolean) =>
    call<void>('create_branch', { repo, name, start, checkout }),
  switchBranch: (repo: string, branch: string) => call<void>('switch_branch', { repo, branch }),
  checkoutCommit: (repo: string, hash: string) => call<void>('checkout_commit', { repo, hash }),
  deleteBranch: (repo: string, name: string, force: boolean) => call<void>('delete_branch', { repo, name, force }),

  listStashes: (repo: string) => call<StashEntry[]>('list_stashes', { repo }),
  stashSave: (repo: string, message: string) => call<void>('stash_save', { repo, message }),
  stashPop: (repo: string, index: number) => call<Outcome>('stash_pop', { repo, index }),
  stashDrop: (repo: string, index: number) => call<void>('stash_drop', { repo, index }),

  remote: (repo: string, operation: RemoteOp) => call<Outcome>('remote', { repo, operation }),
  /** Subscribes to fetch / pull / push progress. Returns an unsubscribe function. */
  onRemoteProgress: async (handler: (p: RemoteProgress) => void): Promise<() => void> => {
    if (mock) return mock.onRemoteProgress(handler)
    const { listen } = await import('@tauri-apps/api/event')
    return listen<RemoteProgress>('remote-progress', (e) => handler(e.payload))
  },

  getLog: (repo: string, scope: LogScope, limit: number) => call<LogEntry[]>('get_log', { repo, scope, limit }),
  getCommit: (repo: string, hash: string) => call<CommitDetail>('get_commit', { repo, hash }),
  getCommitDiff: (repo: string, detail: CommitDetail, file: CommitFile) =>
    call<FileDiff>('get_commit_diff', {
      repo,
      hash: detail.hash,
      parent: detail.parents[0] ?? null,
      path: file.path,
      origPath: file.origPath,
    }),

  agentInfo: () => call<AgentInfo>('agent_info'),
  setAgentAccess: (access: AgentAccess) => call<void>('set_agent_access', { access }),
  /** Runs an agent tool in-process (for agents running inside Leveret). */
  agentCall: (name: string, args: Record<string, unknown>) => call<AgentCallResult>('agent_call', { name, arguments: args }),
  /** Subscribes to changes made by AI agents. Returns an unsubscribe function. */
  onAgentActivity: async (handler: (entries: AgentActivity[]) => void): Promise<() => void> => {
    if (mock) return mock.onAgentActivity(handler)
    const { listen } = await import('@tauri-apps/api/event')
    return listen<AgentActivity[]>('agent-activity', (e) => handler(e.payload))
  },

  githubState: () => call<GithubState>('github_state'),
  githubStartSignIn: () => call<DeviceCode>('github_start_sign_in'),
  githubPollSignIn: (deviceCode: string) => call<PollResult>('github_poll_sign_in', { deviceCode }),
  githubSignInWithToken: (token: string) => call<GithubAccount>('github_sign_in_with_token', { token }),
  githubSignOut: () => call<void>('github_sign_out'),
  githubListRepos: () => call<GithubRepo[]>('github_list_repos'),
  openGithubUrl: (url: string) => call<void>('open_github_url', { url }),

  defaultCloneDir: () => call<string>('default_clone_dir'),
  cloneRepo: (url: string, parent: string, name: string) => call<RepoEntry>('clone_repo', { url, parent, name }),
  /** Subscribes to clone progress. Returns an unsubscribe function. */
  onCloneProgress: async (handler: (p: CloneProgress) => void): Promise<() => void> => {
    if (mock) return mock.onCloneProgress(handler)
    const { listen } = await import('@tauri-apps/api/event')
    return listen<CloneProgress>('clone-progress', (e) => handler(e.payload))
  },

  perfReport: (samples: PerfSample[]) => call<void>('perf_report', { samples }),

  // Capture commands always go to the real backend: they act on the window.
  startupOptions: () =>
    inTauri
      ? invoke<StartupOptions>('startup_options')
      : Promise.resolve<StartupOptions>({ mock: true, theme: null, capture: null, bench: null, perfLog: false, devtools: true }),
  captureScreen: (path?: string) => invoke<string>('capture_screen', { path: path ?? null }),
  benchSave: (report: string) => invoke<string>('bench_save', { report }),
  exitApp: (code = 0) => invoke<void>('exit_app', { code }),
}
