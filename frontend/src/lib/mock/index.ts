// In-memory backend used in a plain browser (UI preview) and for deterministic
// screenshots (`leveret --capture <dir> --mock`). It mimics the Tauri commands
// in backend/src/commands/ closely enough to exercise every screen.
//
// Loaded lazily (`import('./mock/index')`) so it stays out of the main bundle.
// data.ts holds the static data, repo.ts the repository model, state.ts what the
// commands change, and the *Commands.ts files the command handlers by area.
import type { AgentActivity, CloneProgress, RemoteProgress } from '../api'
import { appCommands } from './appCommands'
import { branchCommands } from './branchCommands'
import { MOCK_ACCOUNT } from './data'
import { makeRepos } from './repo'
import { repoCommands } from './repoCommands'
import { agentListeners, cloneListeners, db, delay, fail, remoteListeners, UNHANDLED } from './state'

export function onAgentActivity(handler: (entries: AgentActivity[]) => void): () => void {
  agentListeners.add(handler)
  return () => agentListeners.delete(handler)
}

/** Resets mock state. `empty` starts with no registered repositories. */
export function reset(options: { empty?: boolean; signedIn?: boolean } = {}): void {
  db.repos = options.empty ? [] : makeRepos()
  db.selected = options.empty ? null : 'E:\\Code\\Leveret'
  db.account = options.signedIn ? MOCK_ACCOUNT : null
  db.pollCount = 0
}

export function onCloneProgress(handler: (p: CloneProgress) => void): () => void {
  cloneListeners.add(handler)
  return () => cloneListeners.delete(handler)
}

export function onRemoteProgress(handler: (p: RemoteProgress) => void): () => void {
  remoteListeners.add(handler)
  return () => remoteListeners.delete(handler)
}

export async function handle(cmd: string, args: Record<string, unknown>): Promise<unknown> {
  await delay(30)
  for (const commands of [repoCommands, branchCommands, appCommands]) {
    const result = await commands(cmd, args)
    if (result !== UNHANDLED) return result
  }
  fail('other', `mock: unknown command ${cmd}`)
}
