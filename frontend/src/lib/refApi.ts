// Types and backend calls for branches, tags, stashes, remotes and stopped
// operations (backend/src/commands/{branches,tags,stash,remote,operations}.rs).
// Combined into `api` by api.ts.
import { call, listenTo, mockBackend } from './ipc'
import type { Outcome } from './repoApi'

export interface Tag {
  name: string
  /** The commit the tag points at. */
  target: string
  annotated: boolean
  /** First line of the tag message (or of the commit message for lightweight tags). */
  subject: string
  time: number
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

export const refApi = {
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

  listTags: (repo: string) => call<Tag[]>('list_tags', { repo }),
  createTag: (repo: string, name: string, target: string, message: string) => call<void>('create_tag', { repo, name, target, message }),
  deleteTag: (repo: string, name: string, remote: boolean) => call<void>('delete_tag', { repo, name, remote }),
  pushTag: (repo: string, name: string) => call<void>('push_tag', { repo, name }),

  listStashes: (repo: string) => call<StashEntry[]>('list_stashes', { repo }),
  stashSave: (repo: string, message: string, untracked: boolean) => call<void>('stash_save', { repo, message, untracked }),
  stashPop: (repo: string, index: number) => call<Outcome>('stash_pop', { repo, index }),
  stashDrop: (repo: string, index: number) => call<void>('stash_drop', { repo, index }),

  remote: (repo: string, operation: RemoteOp) => call<Outcome>('remote', { repo, operation }),
  /** Subscribes to fetch / pull / push progress. Returns an unsubscribe function. */
  onRemoteProgress: async (handler: (p: RemoteProgress) => void): Promise<() => void> => {
    const mock = mockBackend()
    return mock ? mock.onRemoteProgress(handler) : listenTo('remote-progress', handler)
  },
}
