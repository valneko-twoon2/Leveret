// Static data for the mock backend: commit subjects, authors and files for the
// generated histories, the GitHub account and repositories, AI agent tools and
// activity, sample diffs and backend perf samples.
import type { AgentActivity, AgentTool, FileDiff, GithubAccount, GithubRepo } from '../api'
import type { PerfSample } from '../perf'

export const NOW = Date.UTC(2026, 9, 17, 11, 30) / 1000

export function rng(seed: number) {
  let s = seed
  return () => {
    s = (s * 1664525 + 1013904223) % 4294967296
    return s / 4294967296
  }
}

export function fakeHash(n: number): string {
  const rand = rng(n * 7919 + 104729)
  for (let i = 0; i < 4; i++) rand()
  let h = ''
  for (let i = 0; i < 40; i++) h += Math.floor(rand() * 16).toString(16)
  return h
}

export const SUBJECTS = [
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
export const AUTHORS: [string, string][] = [
  ['valneko-twoon2', 'valneko-twoon2@example.com'],
  ['Hana Sato', 'hana@example.com'],
  ['Ken Ito', 'ken@example.com'],
]
export const FILES = ['src/ui/log.rs', 'src/git/branch.rs', 'src/main.rs', 'README.md', 'Cargo.toml', 'src/i18n/ja.ts', 'docs/設計メモ.md']

export const MOCK_ACCOUNT: GithubAccount = { login: 'valneko-twoon2', name: 'valneko-twoon2', htmlUrl: 'https://github.com/valneko-twoon2', avatar: null }

export const AGENT_TOOLS: AgentTool[] = [
  ...['list_repositories', 'get_status', 'get_diff', 'open_in_diff_tool', 'get_log', 'get_commit', 'compare_commits', 'blame_file', 'list_branches', 'list_stashes', 'get_stash', 'list_tags', 'list_issues', 'get_issue'].map((name) => ({ name, level: 'read' as const })),
  ...['add_repository', 'clone_repository', 'commit', 'undo_last_commit', 'fetch', 'pull', 'push', 'create_branch', 'switch_branch', 'checkout_commit', 'merge_branch', 'rebase_branch', 'revert_commit', 'cherry_pick', 'mark_resolved', 'continue_operation', 'stash_save', 'stash_apply', 'stash_pop', 'create_tag', 'push_tag', 'ignore_path', 'create_issue', 'update_issue', 'comment_issue', 'publish_issue'].map((name) => ({ name, level: 'write' as const })),
  ...['discard_changes', 'discard_hunks', 'reset_branch', 'restore_file', 'delete_branch', 'resolve_conflict', 'abort_operation', 'skip_operation', 'stash_drop', 'delete_tag', 'delete_issue'].map((name) => ({ name, level: 'destructive' as const })),
].map((t) => ({ ...t, title: t.name }))

export const AGENT_ACTIVITY: AgentActivity[] = [
  { time: (NOW - 3600 * 5) * 1000, client: 'claude-code', tool: 'create_branch', repo: 'Leveret', ok: true, message: 'done' },
  { time: (NOW - 3600 * 4) * 1000, client: 'claude-code', tool: 'commit', repo: 'Leveret', ok: true, message: 'committed' },
  { time: (NOW - 3600 * 2) * 1000, client: 'codex-mcp-client', tool: 'push', repo: 'web-app', ok: false, message: 'push was rejected' },
  { time: (NOW - 1200) * 1000, client: 'Visual Studio Code', tool: 'stash_save', repo: 'Leveret', ok: true, message: 'done' },
]

export const GITHUB_REPOS: GithubRepo[] = [
  ['valneko-twoon2', 'Leveret', 'A small, friendly Git client for Windows', false],
  ['valneko-twoon2', 'dotfiles', 'My configuration files', false],
  ['valneko-twoon2', 'web-app', null, true],
  ['valneko-twoon2', 'advent-of-code', 'Solutions in Rust', false],
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

export function sampleDiff(kind: string, path: string): FileDiff {
  const lines = (text: string[], k: 'add' | 'delete') =>
    text.map((t, i) => ({ kind: k, oldNo: k === 'delete' ? i + 1 : null, newNo: k === 'add' ? i + 1 : null, text: t }))
  // Like the backend: modified, added and untracked text files can be picked line by line.
  const base = { binary: false, tooLarge: false, truncated: false, selectable: true, token: `${kind}:${path}` }
  if (path.endsWith('.png')) return { ...base, binary: true, selectable: false, hunks: [] }
  if (kind === 'added' || kind === 'untracked') {
    const text = path.endsWith('.md')
      ? ['# 設計メモ', '', '## ブランチ作成', '', '- ログ画面の右クリックから作成できるようにする', '- 作成後にチェックアウトするかを選べる']
      : SAMPLE_NEW
    return { ...base, hunks: [{ header: `@@ -0,0 +1,${text.length} @@`, lines: lines(text, 'add') }] }
  }
  if (kind === 'deleted') {
    return { ...base, selectable: false, hunks: [{ header: `@@ -1,${SAMPLE_OLD.length} +0,0 @@`, lines: lines(SAMPLE_OLD, 'delete') }] }
  }
  return {
    ...base,
    selectable: kind === 'modified',
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

/** Backend timings as the real app would report them (git processes, GitHub requests). */
export function mockPerfSamples(): PerfSample[] {
  const rows: [PerfSample['kind'], string, number[]][] = [
    ['startup', 'window created', [182]],
    ['startup', 'page loaded', [412]],
    ['git', 'git status', [28, 31, 26, 35, 30]],
    ['git', 'git log', [64, 58, 71]],
    ['git', 'git diff', [12, 15, 11, 14, 13, 18]],
    ['git', 'git show', [22, 19, 25]],
    ['git', 'git for-each-ref', [9, 11]],
    ['git', 'git fetch', [842]],
    ['http', 'GET /user', [231]],
    ['http', 'GET /user/repos', [388]],
  ]
  let seq = 0
  return rows.flatMap(([kind, name, values]) =>
    values.map((ms) => ({ seq: ++seq, time: Date.now(), kind, name, ms, ok: true, source: 'backend' as const })),
  )
}
