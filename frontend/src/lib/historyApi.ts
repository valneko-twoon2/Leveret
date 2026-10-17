// Types and backend calls for the history: the log, commit details and diffs,
// and actions on a commit (backend/src/commands/{history,operations}.rs).
// Combined into `api` by api.ts.
import { call } from './ipc'
import { prefs } from './prefs.svelte'
import type { ChangeKind, FileDiff, Outcome } from './repoApi'

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

export const historyApi = {
  getLog: (repo: string, scope: LogScope, limit: number) => call<LogEntry[]>('get_log', { repo, scope, limit }),
  getCommit: (repo: string, hash: string) => call<CommitDetail>('get_commit', { repo, hash }),
  getCommitDiff: (repo: string, detail: CommitDetail, file: CommitFile) =>
    call<FileDiff>('get_commit_diff', {
      repo,
      hash: detail.hash,
      parent: detail.parents[0] ?? null,
      path: file.path,
      origPath: file.origPath,
      ignoreWhitespace: prefs.ignoreWhitespace,
    }),

  revertCommit: (repo: string, hash: string) => call<Outcome>('revert_commit', { repo, hash }),
  cherryPick: (repo: string, hash: string) => call<Outcome>('cherry_pick', { repo, hash }),
}
