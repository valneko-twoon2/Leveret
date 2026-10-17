// Types and backend calls for issues (backend/src/issues, backend/src/commands/issues.rs).
// Like the other area files, but not part of `api` (the issues store calls it directly).
import { call } from './ipc'

/** `local`: kept next to the repository, never on GitHub unless published. */
export type IssueSource = 'local' | 'github'
export type IssueState = 'open' | 'closed'
export type IssueStateFilter = IssueState | 'all'

export interface IssueLabel {
  name: string
  /** Hex color without `#` (GitHub labels only). */
  color: string | null
}

export interface Issue {
  source: IssueSource
  /** Local id, or the GitHub issue number. */
  id: number
  title: string
  body: string
  state: IssueState
  labels: IssueLabel[]
  /** GitHub login; null for local issues. */
  author: string | null
  /** Unix seconds. */
  createdAt: number
  updatedAt: number
  comments: number
  url: string | null
  /** A local issue whose publishing stopped halfway: the GitHub issue already created. */
  publishedAs: number | null
}

export interface IssueComment {
  id: number
  author: string | null
  body: string
  createdAt: number
}

export interface IssueDetail {
  issue: Issue
  comments: IssueComment[]
}

export interface IssueSources {
  /** `owner/name` of the github.com remote, if any. */
  githubRepo: string | null
  signedIn: boolean
}

export interface IssueUpdate {
  title?: string
  body?: string
  state?: IssueState
}

export const issueApi = {
  sources: (repo: string) => call<IssueSources>('issue_sources', { repo }),
  list: (repo: string, source: IssueSource, state: IssueStateFilter) => call<Issue[]>('list_issues', { repo, source, state }),
  get: (repo: string, source: IssueSource, id: number) => call<IssueDetail>('get_issue', { repo, source, id }),
  create: (repo: string, source: IssueSource, title: string, body: string) =>
    call<Issue>('create_issue', { repo, source, title, body }),
  update: (repo: string, source: IssueSource, id: number, change: IssueUpdate) =>
    call<Issue>('update_issue', { repo, source, id, change }),
  comment: (repo: string, source: IssueSource, id: number, body: string) =>
    call<IssueComment>('comment_issue', { repo, source, id, body }),
  remove: (repo: string, id: number) => call<void>('delete_issue', { repo, id }),
  publish: (repo: string, id: number) => call<Issue>('publish_issue', { repo, id }),
}
