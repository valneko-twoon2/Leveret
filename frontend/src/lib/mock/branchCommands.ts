// Mock commands for merge / rebase and stopped operations, branches and remote
// actions (fetch, pull, push, sync).
import type { Outcome, PlanCommit, RebaseStep, Remote, RemoteOp, ResetInfo, Tag } from '../api'
import { addCommit, finishOperation, listBranches, moveBranchLabel, remotesOf } from './repo'
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
        r.operation = { kind: isRebase ? 'rebase' : 'merge', branch: isRebase ? r.branch : null, step: isRebase ? 1 : null, total: isRebase ? 3 : null, interactive: false }
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
    case 'rename_branch': {
      const r = find(args.repo)
      const [from, to] = [String(args.name), String(args.newName)]
      if (r.localBranches.includes(to) && from !== to) fail('branchExists', to)
      r.localBranches = r.localBranches.map((b) => (b === from ? to : b))
      if (r.branch === from) r.branch = to
      return null
    }
    case 'set_upstream': {
      const r = find(args.repo)
      const name = String(args.name)
      const upstream = args.upstream ? String(args.upstream).replace(/^refs\/remotes\//, '') : null
      ;(r.upstreamOf ??= {})[name] = upstream
      if (r.branch === name) r.upstream = upstream
      return null
    }
    case 'delete_remote_branch': {
      const r = find(args.repo)
      await delay(300)
      const name = String(args.branch).replace(/^refs\/remotes\//, '')
      r.remoteBranches = r.remoteBranches.filter((b) => b !== name)
      return null
    }
    case 'list_remotes':
      return remotesOf(find(args.repo)) satisfies Remote[]
    case 'add_remote': {
      const r = find(args.repo)
      const remotes = remotesOf(r)
      if (remotes.some((x) => x.name === args.name)) fail('remoteExists', String(args.name))
      remotes.push({ name: String(args.name), fetchUrl: String(args.url), pushUrl: null })
      return null
    }
    case 'rename_remote': {
      const r = find(args.repo)
      const [from, to] = [String(args.name), String(args.newName)]
      const remotes = remotesOf(r)
      if (remotes.some((x) => x.name === to) && from !== to) fail('remoteExists', to)
      for (const x of remotes) if (x.name === from) x.name = to
      r.remoteBranches = r.remoteBranches.map((b) => (b.startsWith(`${from}/`) ? `${to}/${b.slice(from.length + 1)}` : b))
      return null
    }
    case 'remove_remote': {
      const r = find(args.repo)
      const name = String(args.name)
      r.remotes = remotesOf(r).filter((x) => x.name !== name)
      r.remoteBranches = r.remoteBranches.filter((b) => !b.startsWith(`${name}/`))
      return null
    }
    case 'set_remote_url': {
      const r = find(args.repo)
      for (const x of remotesOf(r)) if (x.name === args.name) x[args.push ? 'pushUrl' : 'fetchUrl'] = String(args.url)
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
        op === 'push' || op === 'forcePush'
          ? [['Counting objects', 10], ['Writing objects', 45], ['Writing objects', 100]]
          : [['Receiving objects', 30], ['Receiving objects', 70], ['Resolving deltas', 100]]
      for (const [phase, percent] of phases) {
        await delay(300)
        for (const l of remoteListeners) l({ repo: r.entry.path, operation: op, phase, percent })
      }
      r.lastFetched = Math.floor(Date.now() / 1000)
      if (op === 'pull' || op === 'sync') r.behind = 0
      if (op === 'push' || op === 'forcePush' || op === 'sync') {
        r.ahead = 0
        if (!r.upstream && r.branch) {
          r.upstream = `origin/${r.branch}`
          if (!r.remoteBranches.includes(r.upstream)) r.remoteBranches.push(r.upstream)
        }
      }
      return 'done' satisfies Outcome
    }
    case 'rebase_plan': {
      const r = find(args.repo)
      const end = args.base ? r.commits.findIndex((c) => c.hash === args.base) : r.commits.length
      if (end < 0) fail('invalidInput', 'the commit is not in the current branch')
      const range = r.commits.slice(0, end)
      if (range.some((c) => c.parents.length > 1)) fail('mergesInRange')
      return range
        .map((c, i) => ({ hash: c.hash, subject: c.subject, message: c.body ? `${c.subject}\n\n${c.body}` : c.subject, authorName: c.authorName, authorTime: c.authorTime, pushed: i >= r.ahead }))
        .reverse() satisfies PlanCommit[]
    }
    case 'interactive_rebase': {
      const r = find(args.repo)
      const steps = args.steps as RebaseStep[]
      const byHash = new Map(r.commits.map((c) => [c.hash, c]))
      const kept: typeof r.commits = []
      for (const s of steps) {
        const c = byHash.get(s.hash)
        if (!c) fail('invalidInput', 'the commits have changed; reload them')
        if (s.action === 'drop') continue
        if ((s.action === 'squash' || s.action === 'fixup') && kept.length) {
          kept[0].files = [...kept[0].files, ...c.files]
          if (s.action === 'squash') kept[0].body = [kept[0].body, c.subject].filter(Boolean).join('\n\n')
          continue
        }
        if (s.action === 'reword' && s.message) [c.subject, c.body] = [s.message.split('\n')[0], s.message.split('\n').slice(2).join('\n')]
        kept.unshift(c)
      }
      const oldHead = r.commits[0]?.hash
      r.commits = [...kept, ...r.commits.slice(steps.length)]
      moveBranchLabel(r, oldHead, r.commits[0]?.hash)
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
