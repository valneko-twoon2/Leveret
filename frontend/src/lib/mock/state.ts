// Mutable state of the mock backend (repositories, GitHub account, agent access,
// event listeners) and small helpers shared by the command handlers.
import type { AgentAccess, AgentActivity, AppError, CloneProgress, GithubAccount, RemoteProgress } from '../api'
import { makeRepos, type MockRepo } from './repo'

/** Everything the mock commands change; `reset` (index.ts) puts it back. */
export const db: {
  repos: MockRepo[]
  selected: string | null
  account: GithubAccount | null
  pollCount: number
  agentAccess: AgentAccess
} = {
  repos: makeRepos(),
  selected: 'E:\\Code\\Leveret',
  account: null,
  pollCount: 0,
  agentAccess: 'standard',
}

export const cloneListeners = new Set<(p: CloneProgress) => void>()
export const remoteListeners = new Set<(p: RemoteProgress) => void>()
export const agentListeners = new Set<(entries: AgentActivity[]) => void>()

/** Returned by a group of command handlers for a command it doesn't know. */
export const UNHANDLED = Symbol('unhandled')

export function fail(code: string, detail = ''): never {
  throw { code, detail } satisfies AppError
}

export function find(path: unknown): MockRepo {
  const repo = db.repos.find((r) => r.entry.path === path)
  if (!repo) fail('notARepository', String(path))
  return repo
}

export const delay = (ms: number) => new Promise((r) => setTimeout(r, ms))
