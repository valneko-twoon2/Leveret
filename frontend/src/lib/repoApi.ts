// Types and backend calls for repositories, the working tree and commits
// (backend/src/commands/{repos,changes,difftool}.rs). Combined into `api` by api.ts.
import type { CommitFile } from './historyApi'
import { call } from './ipc'
import { prefs } from './prefs.svelte'

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
  /** An interactive rebase (skipping a step isn't offered). */
  interactive: boolean
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
  /** Lines can be picked for a partial commit or discard. */
  selectable: boolean
  /** Identifies this exact diff; picked lines are sent with it. */
  token: string
}

/** Some lines of a file: `[hunk, line]` indices into the diff with `token`. */
export interface LineSelection {
  path: string
  untracked: boolean
  token: string
  lines: [number, number][]
}

export interface CommitRequest {
  summary: string
  description: string
  paths: string[]
  /** Replace the last commit. */
  amend?: boolean
  /** Files of which only some lines are committed (not also in `paths`). */
  partial?: LineSelection[]
}

/** A picture of the repository, for the image diff. */
export interface ImageData {
  mime: string
  size: number
  /** null when the picture is too large to show */
  dataUrl: string | null
}

/** Where the two sides of an image diff are: a commit (`rev`) or the working tree (`rev` null); a side is null when the file is not there. */
export interface ImageSides {
  old: { rev: string | null; path: string } | null
  new: { rev: string | null; path: string } | null
}

/** Picture file names (the backend serves the same formats). */
export const isImagePath = (path: string) => /\.(png|jpe?g|gif|bmp|webp|ico)$/i.test(path)

export const repoApi = {
  /** A picture at `path` in commit `rev`, or in the working tree when `rev` is null. */
  getImage: (repo: string, rev: string | null, path: string) => call<ImageData | null>('get_image', { repo, rev, path }),
  listRepos: () => call<RepoList>('list_repos'),
  addRepo: (path: string) => call<RepoEntry>('add_repo', { path }),
  removeRepo: (path: string) => call<void>('remove_repo', { path }),
  selectRepo: (path: string | null) => call<void>('select_repo', { path }),

  getStatus: (repo: string, untracked = true) => call<RepoStatus>('get_status', { repo, untracked }),
  getWorkingDiff: (repo: string, file: FileChange) =>
    call<FileDiff>('get_working_diff', {
      repo,
      path: file.path,
      origPath: file.origPath,
      untracked: file.kind === 'untracked',
      options: prefs.diffOptions,
    }),
  commit: (repo: string, request: CommitRequest) => call<string>('commit', { repo, request }),
  undoLastCommit: (repo: string) => call<string>('undo_last_commit', { repo }),

  discardChanges: (repo: string, paths: string[]) => call<void>('discard_changes', { repo, paths }),
  discardLines: (repo: string, selection: LineSelection) => call<void>('discard_lines', { repo, selection }),
  markResolved: (repo: string, paths: string[], force: boolean) => call<void>('mark_resolved', { repo, paths, force }),
  resolveConflict: (repo: string, paths: string[], side: 'ours' | 'theirs') => call<void>('resolve_conflict', { repo, paths, side }),
  ignorePath: (repo: string, path: string, byExtension: boolean) => call<void>('ignore_path', { repo, path, byExtension }),
  showInExplorer: (repo: string, path?: string) => call<void>('show_in_explorer', { repo, path: path ?? null }),

  getDiffTool: () => call<boolean>('get_diff_tool'),
  openWorkingInDiffTool: (repo: string, file: FileChange) =>
    call<void>('open_working_in_diff_tool', { repo, path: file.path, origPath: file.origPath }),
  /** A file in `hash` against `parent` (null: against nothing). */
  openCommitInDiffTool: (repo: string, hash: string, parent: string | null, file: CommitFile) =>
    call<void>('open_commit_in_diff_tool', { repo, hash, parent, path: file.path, origPath: file.origPath }),
}
