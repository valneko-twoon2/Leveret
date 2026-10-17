// In-memory backend used in a plain browser (UI preview) and for deterministic
// screenshots (`leveret --capture <dir> --mock`). It mimics the Tauri commands
// in backend/src/lib.rs closely enough to exercise every screen.
import type {
  AppError,
  Branch,
  CloneProgress,
  CommitDetail,
  CommitFile,
  CommitRequest,
  DeviceCode,
  Edge,
  GithubAccount,
  GithubRepo,
  GithubState,
  PollResult,
  FileChange,
  FileDiff,
  LogEntry,
  LogScope,
  Operation,
  Outcome,
  RefLabel,
  RemoteOp,
  RemoteProgress,
  RepoEntry,
  RepoList,
  RepoStatus,
  StashEntry,
} from './api'

interface MockCommit {
  hash: string
  parents: string[]
  authorName: string
  authorEmail: string
  authorTime: number
  subject: string
  body: string
  files: CommitFile[]
}

interface MockRepo {
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
}

/** Defaults for the branch / remote / stash parts of a mock repository. */
function extras(branch: string, remote: boolean): Pick<MockRepo, 'localBranches' | 'remoteBranches' | 'unmerged' | 'operation' | 'stashes' | 'lastFetched'> {
  return {
    localBranches: [branch],
    remoteBranches: remote ? [`origin/${branch}`] : [],
    unmerged: [],
    operation: null,
    stashes: [],
    lastFetched: remote ? NOW - 1500 : null,
  }
}

const NOW = Date.UTC(2026, 9, 17, 11, 30) / 1000

function rng(seed: number) {
  let s = seed
  return () => {
    s = (s * 1664525 + 1013904223) % 4294967296
    return s / 4294967296
  }
}

function fakeHash(n: number): string {
  const rand = rng(n * 7919 + 104729)
  for (let i = 0; i < 4; i++) rand()
  let h = ''
  for (let i = 0; i < 40; i++) h += Math.floor(rand() * 16).toString(16)
  return h
}

const SUBJECTS = [
  'ログ画面にブランチ作成メニューを追加',
  'コミット画面にステージ機能を追加',
  'WinMerge 連携: 一時ファイルの後始末',
  'rebase: 進捗表示を追加',
  'ロググラフのレーン割り当てを改善',
  'プッシュ/プル/フェッチのダイアログ',
  'git.exe ラッパーを追加',
  'Fix typo in README',
  '差分ビューの行番号を右寄せに',
  'Shift_JIS のファイルを正しく表示する',
  'リポジトリ一覧の並び順を名前順に',
  'Update dependencies',
  'エラーメッセージを多言語化',
  'Refactor status parser',
  'ダークテーマの配色を調整',
  'コミットメッセージの検証を追加',
]
const AUTHORS: [string, string][] = [
  ['dutch', 'dutch@example.com'],
  ['Hana Sato', 'hana@example.com'],
  ['Ken Ito', 'ken@example.com'],
]
const FILES = ['src/ui/log.rs', 'src/git/branch.rs', 'src/main.rs', 'README.md', 'Cargo.toml', 'src/i18n/ja.ts', 'docs/設計メモ.md']

/** Builds a history with a couple of feature branches merged into main. */
function makeHistory(seed: number, count: number, branch: string): { commits: MockCommit[]; refs: Map<string, RefLabel[]> } {
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

function makeRepos(): MockRepo[] {
  const leveret = makeHistory(1, 240, 'main')
  const dotfiles = makeHistory(2, 30, 'master')
  const web = makeHistory(3, 80, 'main')
  return [
    {
      entry: { name: 'dotfiles', path: 'C:\\Users\\dutch\\dotfiles' },
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
      stashes: [{ index: 0, message: 'On main: 差分ビューの配色を試す', time: NOW - 86400 * 2 }],
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

let repos: MockRepo[] = makeRepos()
let selected: string | null = 'E:\\Code\\Leveret'

const MOCK_ACCOUNT: GithubAccount = { login: 'dutch', name: 'Dutch', htmlUrl: 'https://github.com/dutch', avatar: null }
let account: GithubAccount | null = null
let pollCount = 0
const cloneListeners = new Set<(p: CloneProgress) => void>()
const remoteListeners = new Set<(p: RemoteProgress) => void>()

const GITHUB_REPOS: GithubRepo[] = [
  ['dutch', 'Leveret', 'A small, friendly Git client for Windows', false],
  ['dutch', 'dotfiles', 'My configuration files', false],
  ['dutch', 'web-app', null, true],
  ['dutch', 'advent-of-code', 'Solutions in Rust', false],
  ['hana-sato', 'design-notes', '設計メモとスケッチ', true],
  ['example-org', 'api-server', 'Backend API for the example service', true],
  ['example-org', 'docs', 'Public documentation site', false],
].map(([owner, name, description, priv], i) => ({
  fullName: `${owner}/${name}`,
  owner: owner as string,
  name: name as string,
  description: description as string | null,
  private: priv as boolean,
  fork: false,
  cloneUrl: `https://github.com/${owner}/${name}.git`,
  htmlUrl: `https://github.com/${owner}/${name}`,
  pushedAt: new Date((NOW - i * 86400 * 3) * 1000).toISOString(),
}))

/** Resets mock state. `empty` starts with no registered repositories. */
export function reset(options: { empty?: boolean; signedIn?: boolean } = {}): void {
  repos = options.empty ? [] : makeRepos()
  selected = options.empty ? null : 'E:\\Code\\Leveret'
  account = options.signedIn ? MOCK_ACCOUNT : null
  pollCount = 0
}

export function onCloneProgress(handler: (p: CloneProgress) => void): () => void {
  cloneListeners.add(handler)
  return () => cloneListeners.delete(handler)
}

export function onRemoteProgress(handler: (p: RemoteProgress) => void): () => void {
  remoteListeners.add(handler)
  return () => remoteListeners.delete(handler)
}

function fail(code: string, detail = ''): never {
  throw { code, detail } satisfies AppError
}

function find(path: unknown): MockRepo {
  const repo = repos.find((r) => r.entry.path === path)
  if (!repo) fail('notARepository', String(path))
  return repo
}

const SAMPLE_OLD = `use std::path::Path;

pub fn create_branch(repo: &Path, name: &str) -> Result<()> {
    Git::new(repo).args(["branch", name]).run()?;
    Ok(())
}

pub fn delete_branch(repo: &Path, name: &str) -> Result<()> {
    Git::new(repo).args(["branch", "-d", name]).run()?;
    Ok(())
}`.split('\n')

const SAMPLE_NEW = `use std::path::Path;

/// 新しいブランチを作成し、必要ならチェックアウトする。
pub fn create_branch(repo: &Path, name: &str, checkout: bool) -> Result<()> {
    if checkout {
        Git::new(repo).args(["switch", "-c", name]).run()?;
    } else {
        Git::new(repo).args(["branch", name]).run()?;
    }
    Ok(())
}

pub fn delete_branch(repo: &Path, name: &str) -> Result<()> {
    Git::new(repo).args(["branch", "-d", name]).run()?;
    Ok(())
}`.split('\n')

function sampleDiff(kind: string, path: string): FileDiff {
  const lines = (text: string[], k: 'add' | 'delete') =>
    text.map((t, i) => ({ kind: k, oldNo: k === 'delete' ? i + 1 : null, newNo: k === 'add' ? i + 1 : null, text: t }))
  if (path.endsWith('.png')) return { binary: true, tooLarge: false, truncated: false, hunks: [] }
  if (kind === 'added' || kind === 'untracked') {
    const text = path.endsWith('.md')
      ? ['# 設計メモ', '', '## ブランチ作成', '', '- ログ画面の右クリックから作成できるようにする', '- 作成後にチェックアウトするかを選べる']
      : SAMPLE_NEW
    return { binary: false, tooLarge: false, truncated: false, hunks: [{ header: `@@ -0,0 +1,${text.length} @@`, lines: lines(text, 'add') }] }
  }
  if (kind === 'deleted') {
    return { binary: false, tooLarge: false, truncated: false, hunks: [{ header: `@@ -1,${SAMPLE_OLD.length} +0,0 @@`, lines: lines(SAMPLE_OLD, 'delete') }] }
  }
  return {
    binary: false,
    tooLarge: false,
    truncated: false,
    hunks: [
      {
        header: '@@ -1,6 +1,11 @@',
        lines: [
          { kind: 'context', oldNo: 1, newNo: 1, text: SAMPLE_OLD[0] },
          { kind: 'context', oldNo: 2, newNo: 2, text: '' },
          { kind: 'add', oldNo: null, newNo: 3, text: SAMPLE_NEW[2] },
          { kind: 'delete', oldNo: 3, newNo: null, text: SAMPLE_OLD[2] },
          { kind: 'delete', oldNo: 4, newNo: null, text: SAMPLE_OLD[3] },
          ...SAMPLE_NEW.slice(3, 9).map((text, i) => ({ kind: 'add' as const, oldNo: null, newNo: 4 + i, text })),
          { kind: 'context', oldNo: 5, newNo: 10, text: SAMPLE_OLD[4] },
          { kind: 'context', oldNo: 6, newNo: 11, text: SAMPLE_OLD[5] },
        ],
      },
    ],
  }
}

/** Same lane algorithm as backend/src/git/log.rs (build_graph). */
function buildGraph(entries: LogEntry[]): void {
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
function addCommit(r: MockRepo, subject: string, body: string, files: CommitFile[]): MockCommit {
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
function moveBranchLabel(r: MockRepo, from: string | undefined, to: string | undefined) {
  if (from) r.refs.set(from, (r.refs.get(from) ?? []).filter((x) => !x.current))
  if (to && r.branch) r.refs.set(to, [{ name: r.branch, kind: 'branch', current: true }, ...(r.refs.get(to) ?? []).filter((x) => !x.current)])
}

/** Ends the stopped operation: continue (commit) or abort / skip (drop the conflicted state). */
function finishOperation(r: MockRepo, commit: boolean) {
  const op = r.operation
  if (!op) return
  if (op.kind === 'rebase') r.branch = op.branch
  if (commit) addCommit(r, op.kind === 'merge' ? "Merge branch 'feature/rebase'" : 'rebase: 進捗表示を追加', '', [])
  r.files = r.files.filter((f) => f.kind !== 'conflicted' && f.path !== 'src/git/rebase.rs' && f.path !== 'src/ui/history.rs')
  r.operation = null
}

function listBranches(r: MockRepo): Branch[] {
  const head = r.commits[0]
  const make = (name: string, kind: Branch['kind'], i: number): Branch => {
    const c = r.commits[Math.min(r.commits.length - 1, i * 3)] ?? head
    const current = kind === 'local' && name === r.branch
    const upstream = kind === 'local' && r.remoteBranches.includes(`origin/${name}`) ? `origin/${name}` : null
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

const delay = (ms: number) => new Promise((r) => setTimeout(r, ms))

export async function handle(cmd: string, args: Record<string, unknown>): Promise<unknown> {
  await delay(30)
  switch (cmd) {
    case 'list_repos':
      return { repos: repos.map((r) => r.entry), selected } satisfies RepoList
    case 'add_repo': {
      const path = String(args.path).replace(/[\\/]+$/, '')
      if (repos.some((r) => r.entry.path.toLowerCase() === path.toLowerCase())) fail('repoAlreadyAdded', path)
      const name = path.split(/[\\/]/).pop() || path
      const history = makeHistory(repos.length + 10, 20, 'main')
      repos.push({ entry: { name, path }, branch: 'main', upstream: null, ahead: 0, behind: 0, files: [], ...history, ...extras('main', false) })
      repos.sort((a, b) => a.entry.name.toLowerCase().localeCompare(b.entry.name.toLowerCase()))
      selected = path
      return { name, path }
    }
    case 'remove_repo':
      repos = repos.filter((r) => r.entry.path !== args.path)
      if (selected === args.path) selected = null
      return null
    case 'select_repo':
      selected = (args.path as string | null) ?? null
      return null
    case 'get_status': {
      const r = find(args.repo)
      return {
        branch: r.branch,
        headOid: r.commits[0]?.hash ?? null,
        upstream: r.upstream,
        ahead: r.ahead,
        behind: r.behind,
        operation: r.operation,
        hasRemote: r.remoteBranches.length > 0,
        lastFetched: r.lastFetched,
        files: r.files.map((f) => ({ ...f })),
      } satisfies RepoStatus
    }
    case 'get_working_diff': {
      const r = find(args.repo)
      const file = r.files.find((f) => f.path === args.path)
      return sampleDiff(file?.kind ?? 'modified', String(args.path))
    }
    case 'commit': {
      const r = find(args.repo)
      const req = args.request as CommitRequest
      if (!req.summary.trim()) fail('emptySummary')
      if (!req.paths.length && !req.amend) fail('nothingSelected')
      const committed = r.files.filter((f) => req.paths.includes(f.path))
      r.files = r.files.filter((f) => !req.paths.includes(f.path))
      const files = committed.map((f) => ({ path: f.path, origPath: f.origPath, kind: f.kind === 'untracked' ? ('added' as const) : f.kind }))
      if (req.amend && r.commits[0]) {
        const head = r.commits[0]
        head.subject = req.summary.trim()
        head.body = req.description.trim()
        head.files = [...head.files, ...files]
        return head.hash
      }
      const hash = addCommit(r, req.summary.trim(), req.description.trim(), files).hash
      if (r.operation?.kind === 'merge') r.operation = null
      return hash
    }
    case 'get_log': {
      const r = find(args.repo)
      const limit = Number(args.limit)
      void (args.scope as LogScope)
      const entries: LogEntry[] = r.commits.slice(0, limit).map((c) => ({
        hash: c.hash,
        parents: c.parents,
        authorName: c.authorName,
        authorEmail: c.authorEmail,
        authorTime: c.authorTime,
        refs: r.refs.get(c.hash) ?? [],
        subject: c.subject,
        graph: { col: 0, color: 0, width: 1, edges: [] },
      }))
      buildGraph(entries)
      return entries
    }
    case 'get_commit': {
      const r = find(args.repo)
      const c = r.commits.find((x) => x.hash === args.hash)
      if (!c) fail('gitFailed', `bad revision '${String(args.hash)}'`)
      return {
        hash: c.hash,
        parents: c.parents,
        authorName: c.authorName,
        authorEmail: c.authorEmail,
        authorTime: c.authorTime,
        committerName: c.authorName,
        committerEmail: c.authorEmail,
        commitTime: c.authorTime,
        message: c.body ? `${c.subject}\n\n${c.body}` : c.subject,
        files: c.files,
      } satisfies CommitDetail
    }
    case 'get_commit_diff': {
      const r = find(args.repo)
      const c = r.commits.find((x) => x.hash === args.hash)
      const f = c?.files.find((x) => x.path === args.path)
      return sampleDiff(f?.kind ?? 'modified', String(args.path))
    }
    case 'discard_changes': {
      const r = find(args.repo)
      const paths = args.paths as string[]
      r.files = r.files.filter((f) => !paths.includes(f.path))
      return null
    }
    case 'mark_resolved': {
      const r = find(args.repo)
      const paths = args.paths as string[]
      r.files = r.files.map((f) => (paths.includes(f.path) && f.kind === 'conflicted' ? { ...f, kind: 'modified' } : f))
      return null
    }
    case 'ignore_path': {
      const r = find(args.repo)
      r.files = r.files.filter((f) => f.path !== args.path)
      if (!r.files.some((f) => f.path === '.gitignore')) r.files.push({ path: '.gitignore', origPath: null, kind: 'modified' })
      return null
    }
    case 'show_in_explorer':
      console.info(`mock: would open ${String(args.repo)} ${String(args.path ?? '')}`)
      return null
    case 'undo_last_commit': {
      const r = find(args.repo)
      if (r.upstream && r.ahead === 0) fail('alreadyPushed')
      const head = r.commits.shift()
      if (!head) fail('noChanges')
      r.ahead = Math.max(0, r.ahead - 1)
      moveBranchLabel(r, head.hash, r.commits[0]?.hash)
      for (const f of head.files) {
        if (!r.files.some((x) => x.path === f.path)) r.files.push({ path: f.path, origPath: f.origPath, kind: f.kind })
      }
      r.files.sort((x, y) => x.path.localeCompare(y.path))
      return head.body ? `${head.subject}\n\n${head.body}` : head.subject
    }
    case 'revert_commit':
    case 'cherry_pick': {
      const r = find(args.repo)
      const c = r.commits.find((x) => x.hash === args.hash)
      if (!c) fail('gitFailed', `bad revision '${String(args.hash)}'`)
      if (r.operation) fail('gitFailed', 'another operation is in progress')
      const subject = cmd === 'revert_commit' ? `Revert "${c.subject}"` : c.subject
      addCommit(r, subject, cmd === 'revert_commit' ? `This reverts commit ${c.hash}.` : c.body, c.files)
      return 'done' satisfies Outcome
    }
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
    case 'list_stashes':
      return find(args.repo).stashes
    case 'stash_save': {
      const r = find(args.repo)
      if (!r.files.length) fail('noChanges')
      const message = String(args.message).trim() || `WIP on ${r.branch ?? '(no branch)'}: ${r.commits[0]?.subject ?? ''}`
      r.stashes = [{ index: 0, message: `On ${r.branch ?? '(no branch)'}: ${message}`, time: Math.floor(Date.now() / 1000) }, ...r.stashes].map((x, i) => ({ ...x, index: i }))
      r.files = []
      return null
    }
    case 'stash_pop':
    case 'stash_drop': {
      const r = find(args.repo)
      const index = Number(args.index)
      if (!r.stashes[index]) fail('gitFailed', `stash@{${index}} is not a valid reference`)
      if (cmd === 'stash_pop') r.files.push({ path: 'src/ui/diff.css', origPath: null, kind: 'modified' })
      r.stashes = r.stashes.filter((x) => x.index !== index).map((x, i) => ({ ...x, index: i }))
      return cmd === 'stash_pop' ? ('done' satisfies Outcome) : null
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
    case 'github_state':
      return { account, deviceFlow: true } satisfies GithubState
    case 'github_start_sign_in':
      pollCount = 0
      return {
        deviceCode: 'mockdevicecode',
        userCode: 'WDJB-MJHT',
        verificationUri: 'https://github.com/login/device',
        expiresIn: 900,
        interval: 1,
      } satisfies DeviceCode
    case 'github_poll_sign_in':
      if (++pollCount < 3) return { status: 'pending' } satisfies PollResult
      account = MOCK_ACCOUNT
      return { status: 'success', account } satisfies PollResult
    case 'github_sign_in_with_token':
      if (!String(args.token).trim()) fail('invalidInput', 'token')
      account = MOCK_ACCOUNT
      return account
    case 'github_sign_out':
      account = null
      return null
    case 'github_list_repos':
      if (!account) fail('githubNotSignedIn')
      return GITHUB_REPOS
    case 'open_github_url':
      console.info(`mock: would open ${String(args.url)}`)
      return null
    case 'default_clone_dir':
      return 'C:\\Users\\dutch\\Documents\\GitHub'
    case 'clone_repo': {
      const name = String(args.name)
      const path = `${String(args.parent).replace(/[\\/]+$/, '')}\\${name}`
      if (repos.some((r) => r.entry.path.toLowerCase() === path.toLowerCase())) fail('cloneTargetExists', path)
      const phases: [string, number][] = [
        ['Counting objects', 5],
        ['Receiving objects', 30],
        ['Receiving objects', 60],
        ['Receiving objects', 80],
        ['Resolving deltas', 95],
        ['Updating files', 100],
      ]
      for (const [phase, percent] of phases) {
        await delay(250)
        for (const l of cloneListeners) l({ phase, percent })
      }
      return handle('add_repo', { path })
    }
    default:
      fail('other', `mock: unknown command ${cmd}`)
  }
}
