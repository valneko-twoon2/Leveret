// Mock commands for merge / rebase and stopped operations, branches and remote
// actions (fetch, pull, push, sync).
import type { Outcome, RemoteOp } from '../api'
import { addCommit, finishOperation, listBranches, moveBranchLabel } from './repo'
import { delay, fail, find, remoteListeners, UNHANDLED } from './state'

/** Handles one command of this group; `UNHANDLED` for any other command. */
export async function branchCommands(cmd: string, args: Record<string, unknown>): Promise<unknown> {
  switch (cmd) {
    case 'merge_branch':
    case 'rebase_branch': {
      const r = find(args.repo)
      const ref = String(args.branch ?? args.onto)
      if (r.operation) fail('gitFailed', 'another operation is in progress')
      // feature/rebase touches the same lines as main: show the conflict flow.
      if (ref.endsWith('/feature/rebase')) {
        const isRebase = cmd === 'rebase_branch'
        r.operation = { kind: isRebase ? 'rebase' : 'merge', branch: isRebase ? r.branch : null, step: isRebase ? 1 : null, total: isRebase ? 3 : null }
        if (isRebase) r.branch = null
        r.files = [
          { path: 'src/git/rebase.rs', origPath: null, kind: 'conflicted' },
          { path: 'src/ui/history.rs', origPath: null, kind: 'conflicted' },
          { path: 'Cargo.toml', origPath: null, kind: 'modified' },
        ]
        return 'conflicts' satisfies Outcome
      }
      if (cmd === 'merge_branch') addCommit(r, `Merge branch '${ref.replace(/^refs\/(heads|remotes)\//, '')}'`, '', [])
      return 'done' satisfies Outcome
    }
    case 'continue_operation': {
      const r = find(args.repo)
      if (!r.operation) fail('noOperation')
      if (r.files.some((f) => f.kind === 'conflicted')) fail('conflictsRemain')
      finishOperation(r, true)
      return 'done' satisfies Outcome
    }
    case 'skip_operation':
    case 'abort_operation': {
      const r = find(args.repo)
      if (!r.operation) fail('noOperation')
      finishOperation(r, false)
      return cmd === 'skip_operation' ? ('done' satisfies Outcome) : null
    }
    case 'list_branches':
      return listBranches(find(args.repo))
    case 'create_branch': {
      const r = find(args.repo)
      const name = String(args.name)
      if (r.localBranches.includes(name)) fail('branchExists', name)
      r.localBranches.push(name)
      if (args.checkout) {
        r.branch = name
        r.upstream = null
        r.ahead = 0
        r.behind = 0
      }
      return null
    }
    case 'switch_branch': {
      const r = find(args.repo)
      const ref = String(args.branch)
      const name = ref.startsWith('refs/heads/') ? ref.slice('refs/heads/'.length) : ref.replace(/^refs\/remotes\/[^/]+\//, '')
      if (!r.localBranches.includes(name)) r.localBranches.push(name)
      const tracked = r.remoteBranches.includes(`origin/${name}`)
      Object.assign(r, { branch: name, upstream: tracked ? `origin/${name}` : null, ahead: 0, behind: 0 })
      moveBranchLabel(r, r.commits[0]?.hash, r.commits[0]?.hash)
      return null
    }
    case 'checkout_commit': {
      const r = find(args.repo)
      Object.assign(r, { branch: null, upstream: null, ahead: 0, behind: 0 })
      return null
    }
    case 'delete_branch': {
      const r = find(args.repo)
      const name = String(args.name)
      if (r.unmerged.includes(name) && !args.force) fail('branchNotMerged', name)
      r.localBranches = r.localBranches.filter((b) => b !== name)
      return null
    }
    case 'remote': {
      const r = find(args.repo)
      const op = args.operation as RemoteOp
      if (!r.remoteBranches.length) fail('noRemote')
      if (op !== 'fetch' && !r.branch) fail('detachedHead')
      if (op === 'pull' && !r.upstream) fail('noUpstream')
      const phases: [string, number][] =
        op === 'push'
          ? [['Counting objects', 10], ['Writing objects', 45], ['Writing objects', 100]]
          : [['Receiving objects', 30], ['Receiving objects', 70], ['Resolving deltas', 100]]
      for (const [phase, percent] of phases) {
        await delay(300)
        for (const l of remoteListeners) l({ repo: r.entry.path, operation: op, phase, percent })
      }
      r.lastFetched = Math.floor(Date.now() / 1000)
      if (op === 'pull' || op === 'sync') r.behind = 0
      if (op === 'push' || op === 'sync') {
        r.ahead = 0
        if (!r.upstream && r.branch) {
          r.upstream = `origin/${r.branch}`
          if (!r.remoteBranches.includes(r.upstream)) r.remoteBranches.push(r.upstream)
        }
      }
      return 'done' satisfies Outcome
    }
    default:
      return UNHANDLED
  }
}
