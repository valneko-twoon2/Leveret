// Mock commands for merge / rebase and stopped operations, branches and remote
// actions (fetch, pull, push, sync).
import type { Outcome, RemoteOp, ResetInfo, Tag } from '../api'
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
    case 'list_tags': {
      const r = find(args.repo)
      return r.commits.flatMap((c) =>
        (r.refs.get(c.hash) ?? [])
          .filter((x) => x.kind === 'tag')
          .map((x) => ({ name: x.name, target: c.hash, annotated: x.name.startsWith('v'), subject: c.subject, time: c.authorTime }) satisfies Tag),
      )
    }
    case 'create_tag': {
      const r = find(args.repo)
      const name = String(args.name)
      if ([...r.refs.values()].flat().some((x) => x.kind === 'tag' && x.name === name)) fail('tagExists', name)
      const target = String(args.target)
      r.refs.set(target, [...(r.refs.get(target) ?? []), { name, kind: 'tag', current: false }])
      return null
    }
    case 'delete_tag': {
      const r = find(args.repo)
      for (const [hash, labels] of r.refs) r.refs.set(hash, labels.filter((x) => !(x.kind === 'tag' && x.name === args.name)))
      return null
    }
    case 'push_tag':
      await delay(400)
      return null
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
    case 'reset_info': {
      const r = find(args.repo)
      const count = Math.max(0, r.commits.findIndex((c) => c.hash === args.target))
      return { count, pushed: Math.max(0, count - r.ahead), dirty: r.files.some((f) => f.kind !== 'untracked') } satisfies ResetInfo
    }
    case 'reset_branch': {
      const r = find(args.repo)
      const index = r.commits.findIndex((c) => c.hash === args.target)
      if (index < 0) fail('gitFailed', 'bad revision')
      if (args.mode === 'discard' && r.files.some((f) => f.kind !== 'untracked')) fail('localChangesConflict', 'commit or stash the changes first')
      const oldHead = r.commits[0]?.hash
      const removed = r.commits.splice(0, index)
      if (args.mode === 'keep') for (const c of removed) for (const f of c.files) if (!r.files.some((x) => x.path === f.path)) r.files.push({ ...f })
      moveBranchLabel(r, oldHead, r.commits[0]?.hash)
      r.ahead = Math.max(0, r.ahead - removed.length)
      return null
    }
    case 'restore_file': {
      const r = find(args.repo)
      if (!r.files.some((f) => f.path === args.path)) r.files.push({ path: String(args.path), origPath: null, kind: 'modified' })
      return null
    }
    default:
      return UNHANDLED
  }
}
