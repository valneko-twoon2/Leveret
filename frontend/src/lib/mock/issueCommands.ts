// Mock issue commands: local issues for every repository, GitHub issues for the
// Leveret repository (whose mock remote is on github.com).
import type { Issue, IssueComment, IssueDetail, IssueSource, IssueSources, IssueStateFilter, IssueUpdate } from '../issueApi'
import { db, fail, UNHANDLED } from './state'

const GITHUB_REPO = 'valneko-twoon2/Leveret'
const LEVERET = 'E:\\Code\\Leveret'

interface MockIssue {
  issue: Issue
  comments: IssueComment[]
}

const now = () => Math.floor(Date.now() / 1000)
const HOUR = 3600
const DAY = 24 * HOUR

function make(
  source: IssueSource,
  id: number,
  title: string,
  body: string,
  ageHours: number,
  extra: Partial<Issue> = {},
  comments: [string | null, string, number][] = [],
): MockIssue {
  const created = now() - ageHours * HOUR
  const list = comments.map(([author, text, hours], i) => ({ id: i + 1, author, body: text, createdAt: now() - hours * HOUR }))
  return {
    issue: {
      source,
      id,
      title,
      body,
      state: 'open',
      labels: [],
      author: source === 'github' ? 'valneko-twoon2' : null,
      createdAt: created,
      updatedAt: list.length ? list[list.length - 1].createdAt : created,
      comments: list.length,
      url: source === 'github' ? `https://github.com/${GITHUB_REPO}/issues/${id}` : null,
      publishedAs: null,
      ...extra,
    },
    comments: list,
  }
}

function seedGithub(): MockIssue[] {
  return [
    make(
      'github',
      42,
      '大きなリポジトリで履歴の表示が遅い',
      'コミットが 10 万件あるリポジトリで履歴タブを開くと 1 秒ほど待たされます。\n\n手順:\n1. 大きなリポジトリを開く\n2. 履歴タブを選ぶ\n\nベンチマーク（scripts\\benchmark.bat）の結果も添付します。',
      30,
      { labels: [{ name: 'performance', color: 'fbca04' }, { name: 'history', color: '0e8a16' }] },
      [
        ['hana-sato', '手元でも再現しました。git log の起動に 60ms ほどかかっています。', 20],
        ['valneko-twoon2', '一覧の取得と詳細の取得を並列にしてみます。', 6],
      ],
    ),
    make('github', 41, 'WinMerge で差分を開きたい', '差分の右クリックメニューから WinMerge を起動できると便利です。', 72, {
      labels: [{ name: 'enhancement', color: 'a2eeef' }],
    }),
    make('github', 39, 'ダークテーマで差分の削除行が見づらい', '', 9 * 24, {
      labels: [{ name: 'bug', color: 'd73a4a' }, { name: 'ui', color: 'c5def5' }],
      author: 'ken-ito',
    }),
    make('github', 35, 'スタッシュの一覧に日時を表示する', 'スタッシュを作った日時が分かるとよい。', 20 * 24, { state: 'closed' }),
  ]
}

function seedLocal(path: string): MockIssue[] {
  if (path !== LEVERET) return []
  return [
    make('local', 2, 'リリース前にアイコンの縁取りを確認する', 'タスクバーの小さいサイズで縁がにじんでいないか見る。', 3),
    make('local', 1, 'Issue のテンプレートを用意する', 'バグ報告と機能要望の 2 種類。\n反映前にチームで見直す。', 2 * DAY / HOUR, {}, [[null, '項目案: 再現手順、期待する動作、環境', 26]]),
  ]
}

let local = new Map<string, MockIssue[]>()
let github = seedGithub()

export function resetIssues() {
  local = new Map()
  github = seedGithub()
}

function localOf(path: string): MockIssue[] {
  let list = local.get(path)
  if (!list) {
    list = seedLocal(path)
    local.set(path, list)
  }
  return list
}

function store(path: string, source: IssueSource): MockIssue[] {
  if (source === 'local') return localOf(path)
  if (path !== LEVERET) fail('noGithubRemote')
  return github
}

function requireSignIn(source: IssueSource) {
  if (source === 'github' && !db.account) fail('githubNotSignedIn')
}

function find(list: MockIssue[], id: number): MockIssue {
  const item = list.find((m) => m.issue.id === id)
  if (!item) fail('issueNotFound', String(id))
  return item
}

function matches(filter: IssueStateFilter, issue: Issue) {
  return filter === 'all' || issue.state === filter
}

function create(path: string, source: IssueSource, title: string, body: string): Issue {
  requireSignIn(source)
  if (!title.trim()) fail('invalidInput', 'issue title')
  const list = store(path, source)
  const id = Math.max(source === 'github' ? 42 : 0, ...list.map((m) => m.issue.id)) + 1
  const item = make(source, id, title.trim(), body, 0)
  list.push(item)
  return item.issue
}

/** Handles one issue command; `UNHANDLED` for any other command. */
export async function issueCommands(cmd: string, args: Record<string, unknown>): Promise<unknown> {
  const path = String(args.repo)
  const source = args.source as IssueSource
  const id = Number(args.id)
  switch (cmd) {
    case 'issue_sources':
      return { githubRepo: path === LEVERET ? GITHUB_REPO : null, signedIn: db.account !== null } satisfies IssueSources
    case 'list_issues':
      return store(path, source)
        .filter((m) => matches(args.state as IssueStateFilter, m.issue))
        .map((m) => m.issue)
    case 'get_issue': {
      const m = find(store(path, source), id)
      return { issue: m.issue, comments: m.comments } satisfies IssueDetail
    }
    case 'create_issue':
      return create(path, source, String(args.title), String(args.body))
    case 'update_issue': {
      requireSignIn(source)
      const m = find(store(path, source), id)
      const change = args.change as IssueUpdate
      m.issue = { ...m.issue, ...change, title: change.title?.trim() ?? m.issue.title, updatedAt: now() }
      return m.issue
    }
    case 'comment_issue': {
      requireSignIn(source)
      const m = find(store(path, source), id)
      const comment: IssueComment = {
        id: m.comments.length + 1,
        author: source === 'github' ? 'valneko-twoon2' : null,
        body: String(args.body),
        createdAt: now(),
      }
      m.comments.push(comment)
      m.issue = { ...m.issue, comments: m.comments.length, updatedAt: comment.createdAt }
      return comment
    }
    case 'delete_issue': {
      const list = localOf(path)
      find(list, id)
      local.set(
        path,
        list.filter((m) => m.issue.id !== id),
      )
      return null
    }
    case 'publish_issue': {
      if (path !== LEVERET) fail('noGithubRemote')
      requireSignIn('github')
      const m = find(localOf(path), id)
      const issue = create(path, 'github', m.issue.title, m.issue.body)
      const published = find(github, issue.id)
      published.comments = m.comments.map((c) => ({ ...c, author: 'valneko-twoon2' }))
      published.issue = { ...published.issue, state: m.issue.state, comments: published.comments.length }
      local.set(
        path,
        localOf(path).filter((x) => x.issue.id !== id),
      )
      return published.issue
    }
    default:
      return UNHANDLED
  }
}
