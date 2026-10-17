// Mock commands for repositories, status, the commit form, history (including
// revert / cherry-pick), discard / resolve / ignore and stashes.
import type { CommitDetail, CommitRequest, LogEntry, LogScope, Outcome, RepoList, RepoStatus } from '../api'
import { sampleDiff } from './data'
import { addCommit, buildGraph, extras, makeHistory, moveBranchLabel } from './repo'
import { db, fail, find, UNHANDLED } from './state'

/** Handles one command of this group; `UNHANDLED` for any other command. */
export async function repoCommands(cmd: string, args: Record<string, unknown>): Promise<unknown> {
  switch (cmd) {
    case 'list_repos':
      return { repos: db.repos.map((r) => r.entry), selected: db.selected } satisfies RepoList
    case 'add_repo': {
      const path = String(args.path).replace(/[\\/]+$/, '')
      if (db.repos.some((r) => r.entry.path.toLowerCase() === path.toLowerCase())) fail('repoAlreadyAdded', path)
      const name = path.split(/[\\/]/).pop() || path
      const history = makeHistory(db.repos.length + 10, 20, 'main')
      db.repos.push({ entry: { name, path }, branch: 'main', upstream: null, ahead: 0, behind: 0, files: [], ...history, ...extras('main', false) })
      db.repos.sort((a, b) => a.entry.name.toLowerCase().localeCompare(b.entry.name.toLowerCase()))
      db.selected = path
      return { name, path }
    }
    case 'remove_repo':
      db.repos = db.repos.filter((r) => r.entry.path !== args.path)
      if (db.selected === args.path) db.selected = null
      return null
    case 'select_repo':
      db.selected = (args.path as string | null) ?? null
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
    case 'get_diff_tool':
      return true
    case 'open_working_in_diff_tool':
    case 'open_commit_in_diff_tool':
      console.info(`mock: would open ${String(args.path)} in WinMerge`)
      return null
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
    default:
      return UNHANDLED
  }
}
