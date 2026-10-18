// The mock repository model: generated histories and the helpers the command
// handlers use to change them (commits, branch labels, stopped operations).
import type { Branch, CommitFile, Edge, FileChange, LogEntry, Operation, RefLabel, Remote, RepoEntry, StashEntry } from '../api'
import { AUTHORS, FILES, NOW, SUBJECTS, fakeHash, rng } from './data'

export interface MockCommit {
  hash: string
  parents: string[]
  authorName: string
  authorEmail: string
  authorTime: number
  subject: string
  body: string
  files: CommitFile[]
}

export interface MockRepo {
  entry: RepoEntry
  /** null: detached HEAD */
  branch: string | null
  upstream: string | null
  ahead: number
  behind: number
  files: FileChange[]
  commits: MockCommit[]
  refs: Map<string, RefLabel[]>
  localBranches: string[]
  remoteBranches: string[]
  /** Local branches with commits that are merged nowhere (deleting them needs force). */
  unmerged: string[]
  operation: Operation | null
  stashes: StashEntry[]
  lastFetched: number | null
  /** Upstreams set by the user (default: the remote branch of the same name). */
  upstreamOf?: Record<string, string | null>
  remotes?: Remote[]
}

/** Defaults for the branch / remote / stash parts of a mock repository. */
export function extras(branch: string, remote: boolean): Pick<MockRepo, 'localBranches' | 'remoteBranches' | 'unmerged' | 'operation' | 'stashes' | 'lastFetched'> {
  return {
    localBranches: [branch],
    remoteBranches: remote ? [`origin/${branch}`] : [],
    unmerged: [],
    operation: null,
    stashes: [],
    lastFetched: remote ? NOW - 1500 : null,
  }
}

/** Builds a history with a couple of feature branches merged into main. */
export function makeHistory(seed: number, count: number, branch: string): { commits: MockCommit[]; refs: Map<string, RefLabel[]> } {
  const rand = rng(seed)
  const commits: MockCommit[] = []
  let t = NOW - 600
  const make = (i: number, parents: string[], subject?: string): MockCommit => {
    const [name, email] = AUTHORS[Math.floor(rand() * AUTHORS.length)]
    t -= Math.floor(1800 + rand() * 30000)
    const nfiles = 1 + Math.floor(rand() * 3)
    const files: CommitFile[] = []
    for (let k = 0; k < nfiles; k++) {
      const path = FILES[Math.floor(rand() * FILES.length)]
      if (!files.some((f) => f.path === path)) files.push({ path, origPath: null, kind: rand() < 0.15 ? 'added' : 'modified' })
    }
    return {
      hash: fakeHash(seed * 1000 + i),
      parents,
      authorName: name,
      authorEmail: email,
      authorTime: t,
      subject: subject ?? SUBJECTS[Math.floor(rand() * SUBJECTS.length)],
      body: rand() < 0.4 ? '変更の背景や詳細をここに書きます。\n複数行の説明にも対応しています。' : '',
      files,
    }
  }

  // Generate oldest-first ids, newest-first order: mainline with occasional side branches.
  const ids = Array.from({ length: count }, (_, i) => fakeHash(seed * 1000 + i))
  let i = 0
  while (i < count) {
    const isMergePoint = i % 9 === 2 && i + 4 < count
    if (isMergePoint) {
      // merge commit at i: parents = mainline i+3, side i+1; side chain i+1 -> i+2 -> (mainline i+3's parent i+4)
      commits.push(make(i, [ids[i + 3], ids[i + 1]], `Merge branch 'feature/${['log-graph', 'rebase', 'i18n', 'diff'][(i / 9) % 4 | 0]}'`))
      commits.push(make(i + 1, [ids[i + 2]]))
      commits.push(make(i + 2, [ids[i + 4]]))
      commits.push(make(i + 3, [ids[i + 4]]))
      i += 4
    } else {
      commits.push(make(i, i + 1 < count ? [ids[i + 1]] : []))
      i += 1
    }
  }
  commits[commits.length - 1].subject = 'Initial commit'
  commits[commits.length - 1].parents = []

  const refs = new Map<string, RefLabel[]>()
  refs.set(commits[0].hash, [{ name: branch, kind: 'branch', current: true }])
  refs.set(commits[1].hash, [{ name: `origin/${branch}`, kind: 'remote', current: false }])
  if (commits.length > 12) refs.set(commits[12].hash, [{ name: 'v0.1.0', kind: 'tag', current: false }])
  return { commits, refs }
}

export function makeRepos(): MockRepo[] {
  const leveret = makeHistory(1, 240, 'main')
  const dotfiles = makeHistory(2, 30, 'master')
  const web = makeHistory(3, 80, 'main')
  return [
    {
      entry: { name: 'dotfiles', path: 'C:\\Users\\valneko-twoon2\\dotfiles' },
      branch: 'master',
      upstream: 'origin/master',
      ahead: 0,
      behind: 0,
      files: [],
      ...dotfiles,
      ...extras('master', true),
    },
    {
      entry: { name: 'Leveret', path: 'E:\\Code\\Leveret' },
      branch: 'main',
      upstream: 'origin/main',
      ahead: 1,
      behind: 0,
      files: [
        { path: 'Cargo.toml', origPath: null, kind: 'modified' },
        { path: 'docs/設計メモ.md', origPath: null, kind: 'untracked' },
        { path: 'src/git/branch.rs', origPath: null, kind: 'modified' },
        { path: 'src/old_dialog.rs', origPath: null, kind: 'deleted' },
        { path: 'src/ui/branch_dialog.rs', origPath: null, kind: 'added' },
        { path: 'src/ui/history.rs', origPath: 'src/ui/log.rs', kind: 'renamed' },
      ],
      ...leveret,
      ...extras('main', true),
      localBranches: ['main', 'feature/branch-dialog', 'feature/rebase', 'fix/sjis-diff'],
      remoteBranches: ['origin/main', 'origin/feature/branch-dialog', 'origin/release/0.1'],
      unmerged: ['feature/rebase'],
      stashes: [{ index: 0, oid: '5a5c0de1'.padEnd(40, '0'), message: 'On main: 差分ビューの配色を試す', time: NOW - 86400 * 2 }],
    },
    {
      entry: { name: 'web-app', path: 'E:\\Code\\web-app' },
      branch: 'feature/login',
      upstream: null,
      ahead: 0,
      behind: 0,
      files: [{ path: 'src/routes/login.svelte', origPath: null, kind: 'modified' }],
      ...web,
      ...extras('feature/login', true),
      remoteBranches: ['origin/main'],
    },
  ]
}

/** Same lane algorithm as backend/src/git/log.rs (build_graph). */
export function buildGraph(entries: LogEntry[]): void {
  const lanes: ([string, number] | null)[] = []
  let nextColor = 0
  const freeSlot = () => {
    const i = lanes.indexOf(null)
    if (i >= 0) return i
    lanes.push(null)
    return lanes.length - 1
  }
  for (const e of entries) {
    const edges: Edge[] = []
    let col = lanes.findIndex((l) => l?.[0] === e.hash)
    const existing = col >= 0
    if (!existing) {
      col = freeSlot()
      lanes[col] = [e.hash, nextColor++]
    }
    const color = lanes[col]![1]
    const through: [number, number][] = []
    lanes.forEach((l, i) => {
      if (!l) return
      if (l[0] === e.hash) {
        if (existing) edges.push({ from: i, to: col, kind: 'up', color: l[1] })
      } else {
        through.push([i, l[1]])
        edges.push({ from: i, to: i, kind: 'up', color: l[1] })
      }
    })
    lanes.forEach((l, i) => {
      if (l?.[0] === e.hash) lanes[i] = null
    })
    e.parents.forEach((p, k) => {
      let j = k === 0 ? -1 : lanes.findIndex((l) => l?.[0] === p)
      let c: number
      if (j >= 0) {
        c = lanes[j]![1]
      } else {
        j = k === 0 ? col : freeSlot()
        c = k === 0 ? color : nextColor++
        lanes[j] = [p, c]
      }
      edges.push({ from: col, to: j, kind: 'down', color: c })
    })
    for (const [i, c] of through) edges.push({ from: i, to: i, kind: 'down', color: c })
    const width = Math.max(col, ...edges.map((x) => Math.max(x.from, x.to))) + 1
    e.graph = { col, color, width, edges }
    while (lanes.length && lanes[lanes.length - 1] === null) lanes.pop()
  }
}

/** Adds a commit on top of HEAD and moves the current branch label to it. */
export function addCommit(r: MockRepo, subject: string, body: string, files: CommitFile[]): MockCommit {
  const commit: MockCommit = {
    hash: fakeHash(Date.now() % 100000 + r.commits.length),
    parents: r.commits[0] ? [r.commits[0].hash] : [],
    authorName: AUTHORS[0][0],
    authorEmail: AUTHORS[0][1],
    authorTime: Math.floor(Date.now() / 1000),
    subject,
    body,
    files,
  }
  const previous = r.commits[0]?.hash
  r.commits.unshift(commit)
  moveBranchLabel(r, previous, commit.hash)
  r.ahead += 1
  return commit
}

/** Moves the "current branch" label from one commit to another. */
export function moveBranchLabel(r: MockRepo, from: string | undefined, to: string | undefined) {
  if (from) r.refs.set(from, (r.refs.get(from) ?? []).filter((x) => !x.current))
  if (to && r.branch) r.refs.set(to, [{ name: r.branch, kind: 'branch', current: true }, ...(r.refs.get(to) ?? []).filter((x) => !x.current)])
}

/** Ends the stopped operation: continue (commit) or abort / skip (drop the conflicted state). */
export function finishOperation(r: MockRepo, commit: boolean) {
  const op = r.operation
  if (!op) return
  if (op.kind === 'rebase') r.branch = op.branch
  if (commit) addCommit(r, op.kind === 'merge' ? "Merge branch 'feature/rebase'" : 'rebase: 進捗表示を追加', '', [])
  r.files = r.files.filter((f) => f.kind !== 'conflicted' && f.path !== 'src/git/rebase.rs' && f.path !== 'src/ui/history.rs')
  r.operation = null
}

/** The remotes of a mock repository: `origin` when it has remote branches. */
export function remotesOf(r: MockRepo): Remote[] {
  r.remotes ??= r.remoteBranches.length ? [{ name: 'origin', fetchUrl: `https://github.com/valneko-twoon2/${r.entry.name}.git`, pushUrl: null }] : []
  return r.remotes
}

export function listBranches(r: MockRepo): Branch[] {
  const head = r.commits[0]
  const make = (name: string, kind: Branch['kind'], i: number): Branch => {
    const c = r.commits[Math.min(r.commits.length - 1, i * 3)] ?? head
    const current = kind === 'local' && name === r.branch
    const derived = r.remoteBranches.includes(`origin/${name}`) ? `origin/${name}` : null
    const upstream = kind === 'local' ? (r.upstreamOf && name in r.upstreamOf ? r.upstreamOf[name] : derived) : null
    return {
      name,
      fullRef: kind === 'local' ? `refs/heads/${name}` : `refs/remotes/${name}`,
      kind,
      current,
      upstream,
      upstreamGone: false,
      ahead: current ? r.ahead : name === 'fix/sjis-diff' ? 2 : 0,
      behind: current ? r.behind : 0,
      oid: c?.hash ?? '',
      time: c?.authorTime ?? 0,
      subject: c?.subject ?? '',
    }
  }
  return [...r.localBranches.map((b, i) => make(b, 'local', i)), ...r.remoteBranches.map((b, i) => make(b, 'remote', i))]
}
