// Types and backend calls for the app itself: settings, AI agents, GitHub,
// cloning, timings and the capture / benchmark modes. Combined into `api` by api.ts.
import { invoke } from '@tauri-apps/api/core'
import { call, inTauri, listenTo, mockBackend } from './ipc'
import type { PerfSample } from './perf'
import type { RepoEntry } from './repoApi'

export interface GitSettings {
  version: string
  userName: string
  userEmail: string
  defaultBranch: string
  pullRebase: string
  autocrlf: string
}

export type GitConfigKey = 'user.name' | 'user.email' | 'init.defaultBranch' | 'pull.rebase' | 'core.autocrlf'

export interface AppSettings {
  version: string
  git: GitSettings
  cloneDir: string
  configDir: string
  logsDir: string
  diffTool: boolean
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

export const appApi = {
  getSettings: () => call<AppSettings>('get_settings'),
  setGitConfig: (key: GitConfigKey, value: string) => call<void>('set_git_config', { key, value }),
  setCloneDir: (dir: string | null) => call<void>('set_clone_dir', { dir }),
  openAppFolder: (which: 'config' | 'logs') => call<void>('open_app_folder', { which }),

  agentInfo: () => call<AgentInfo>('agent_info'),
  setAgentAccess: (access: AgentAccess) => call<void>('set_agent_access', { access }),
  /** Runs an agent tool in-process (for agents running inside Leveret). */
  agentCall: (name: string, args: Record<string, unknown>) => call<AgentCallResult>('agent_call', { name, arguments: args }),
  /** Subscribes to changes made by AI agents. Returns an unsubscribe function. */
  onAgentActivity: async (handler: (entries: AgentActivity[]) => void): Promise<() => void> => {
    const mock = mockBackend()
    return mock ? mock.onAgentActivity(handler) : listenTo('agent-activity', handler)
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
    const mock = mockBackend()
    return mock ? mock.onCloneProgress(handler) : listenTo('clone-progress', handler)
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
