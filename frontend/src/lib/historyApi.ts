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
  /** In a file's history: the file's path in this commit (it may have been renamed since). */
  filePath?: string
}

export type LogScope = 'head' | 'all'

/** Searching the history in git; every given condition must match. */
export interface LogQuery {
  /** Text in the message (a pasted hash finds its commit too). */
  message?: string
  author?: string
  /** Commits that add or remove this text. */
  content?: string
  /** Commits that touch this file or folder. */
  path?: string
  /** One file's history, following renames. */
  file?: string
}

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

export interface BlameCommit {
  /** All zeros for lines that are not committed yet. */
  hash: string
  authorName: string
  authorTime: number
  summary: string
}

/** Who last changed each line of a file. */
export interface Blame {
  commits: BlameCommit[]
  /** Line `i` is number `i + 1`; `commit` indexes `commits`. */
  lines: { commit: number; text: string }[]
  binary: boolean
  truncated: boolean
}

/** What resetting the current branch to a commit would do. */
export interface ResetInfo {
  /** Commits that leave the branch. */
  count: number
  /** Of those, commits on a remote (pushing then needs a force push). */
  pushed: number
  /** Uncommitted changes to tracked files (a discarding reset is refused then). */
  dirty: boolean
}

/** `keep`: the removed commits' changes stay as uncommitted changes. */
export type ResetMode = 'keep' | 'discard'
/** `version`: the file as in the commit; `undo`: the current file without the commit's change. */
export type RestoreMode = 'version' | 'undo'

export const historyApi = {
  getLog: (repo: string, scope: LogScope, limit: number, query: LogQuery | null = null) =>
    call<LogEntry[]>('get_log', { repo, scope, limit, query }),
  getCommit: (repo: string, hash: string) => call<CommitDetail>('get_commit', { repo, hash }),
  /** `rev`: the file at that commit; null: the working tree file. */
  getBlame: (repo: string, rev: string | null, path: string, ignoreWhitespace: boolean) =>
    call<Blame>('get_blame', { repo, rev, path, ignoreWhitespace }),
  getCommitDiff: (repo: string, detail: CommitDetail, file: CommitFile) =>
    call<FileDiff>('get_commit_diff', {
      repo,
      hash: detail.hash,
      parent: detail.parents[0] ?? null,
      path: file.path,
      origPath: file.origPath,
      ignoreWhitespace: prefs.ignoreWhitespace,
    }),

  /** Files that differ between any two commits. */
  compareCommits: (repo: string, base: string, target: string) => call<CommitFile[]>('compare_commits', { repo, base, target }),
  /** A file's diff from `base` to `target`. */
  getRangeDiff: (repo: string, base: string, target: string, file: CommitFile) =>
    call<FileDiff>('get_commit_diff', {
      repo,
      hash: target,
      parent: base,
      path: file.path,
      origPath: file.origPath,
      ignoreWhitespace: prefs.ignoreWhitespace,
    }),

  resetInfo: (repo: string, target: string) => call<ResetInfo>('reset_info', { repo, target }),
  resetBranch: (repo: string, target: string, mode: ResetMode) => call<void>('reset_branch', { repo, target, mode }),
  restoreFile: (repo: string, hash: string, file: CommitFile, mode: RestoreMode) =>
    call<void>('restore_file', { repo, hash, path: file.path, origPath: file.origPath, mode }),

  revertCommit: (repo: string, hash: string) => call<Outcome>('revert_commit', { repo, hash }),
  cherryPick: (repo: string, hash: string) => call<Outcome>('cherry_pick', { repo, hash }),
}
