// The issues tab: local issues and the GitHub issues of the repository's
// github.com remote, the selected issue with its comments, the editor dialog, and
// publishing local issues to GitHub. Builds on `app` (state.svelte.ts).
//
// Issues are loaded when the tab is opened and by its refresh button, not on every
// window focus like the rest of the app, to stay well within GitHub's rate limits.
import { api, isAppError } from './api'
import { confirmDialog } from './dialogs'
import { t } from './i18n/index.svelte'
import {
  issueApi,
  type Issue,
  type IssueDetail,
  type IssueSource,
  type IssueSources,
  type IssueStateFilter,
  type IssueUpdate,
} from './issueApi'
import { perf } from './perf'
import { app, describeError } from './state.svelte'

export type IssueFilter = 'all' | IssueSource

/** The editor dialog: a new issue, or changes to an existing one. */
export interface IssueDraft {
  mode: 'new' | 'edit'
  source: IssueSource
  id: number | null
  title: string
  body: string
}

export function issueKey(i: { source: IssueSource; id: number }): string {
  return `${i.source}:${i.id}`
}

class IssuesState {
  sources = $state.raw<IssueSources | null>(null)
  local = $state.raw<Issue[]>([])
  github = $state.raw<Issue[]>([])
  /** Why GitHub issues couldn't be loaded (local issues still show). */
  githubError = $state<string | null>(null)
  loaded = $state(false)
  loading = $state(false)
  /** Open issues as of the last load that included them (for the tab's badge). */
  openCount = $state<number | null>(null)
  filter = $state<IssueFilter>('all')
  stateFilter = $state<IssueStateFilter>('open')
  query = $state('')
  selected = $state<string | null>(null)
  detail = $state.raw<IssueDetail | null>(null)
  /** The selected issue couldn't be loaded (shown with a retry button). */
  detailFailed = $state(false)
  draft = $state<IssueDraft | null>(null)
  comment = $state('')
  /** A change is being sent; buttons are disabled meanwhile. */
  saving = $state(false)

  /** Only the latest load may apply its results (filters and repositories change). */
  private loadSeq = 0

  /** The issues shown in the list, most recently updated first. */
  list = $derived.by(() => {
    const items = this.filter === 'local' ? this.local : this.filter === 'github' ? this.github : [...this.local, ...this.github]
    const q = this.query.trim().toLowerCase()
    const shown = q
      ? items.filter(
          (i) =>
            i.title.toLowerCase().includes(q) ||
            `#${i.id}` === q ||
            String(i.id) === q ||
            i.labels.some((l) => l.name.toLowerCase().includes(q)),
        )
      : items
    return [...shown].sort((a, b) => b.updatedAt - a.updatedAt)
  })

  /** GitHub issues can be created and published to. */
  canUseGithub = $derived(!!this.sources?.githubRepo && this.sources.signedIn)

  /** Forgets everything about the previous repository (called by `app.selectRepo`). */
  resetRepoView() {
    this.loadSeq++
    this.sources = null
    this.local = []
    this.github = []
    this.githubError = null
    this.loaded = false
    this.loading = false
    this.openCount = null
    this.query = ''
    this.selected = null
    this.detail = null
    this.detailFailed = false
    this.draft = null
    this.comment = ''
  }

  /** Loads both lists (in parallel) and keeps or picks the selected issue. */
  async load() {
    const repo = app.selectedPath
    if (!repo) return
    const seq = ++this.loadSeq
    await perf.view('issues.load', async () => {
      this.loading = true
      const state = this.stateFilter
      const [sources, local, github] = await Promise.all([
        app.guard(() => issueApi.sources(repo)),
        app.guard(() => issueApi.list(repo, 'local', state)),
        app
          .track(issueApi.list(repo, 'github', state))
          .then((list) => ({ list, error: null }))
          .catch((e: unknown) => ({
            list: [] as Issue[],
            // No github.com remote is a normal state, not an error.
            error: isAppError(e) && e.code === 'noGithubRemote' ? null : describeError(e).message,
          })),
      ])
      if (seq !== this.loadSeq || repo !== app.selectedPath) return
      this.loading = false
      this.sources = sources ?? null
      this.local = local ?? []
      this.github = github.list
      this.githubError = github.error
      this.loaded = true
      if (state !== 'closed') this.openCount = [...this.local, ...this.github].filter((i) => i.state === 'open').length
      // A GitHub filter left over from another repository.
      if (this.filter === 'github' && !this.sources?.githubRepo) this.filter = 'all'
      const keep = this.list.find((i) => issueKey(i) === this.selected) ?? this.list[0]
      await (keep ? this.select(keep) : this.clearSelection())
    })
  }

  async setStateFilter(state: IssueStateFilter) {
    this.stateFilter = state
    await this.load()
  }

  private clearSelection() {
    this.selected = null
    this.detail = null
    this.detailFailed = false
  }

  async select(issue: Issue) {
    const key = issueKey(issue)
    const repo = app.selectedPath
    if (!repo) return
    if (this.selected !== key) {
      this.selected = key
      this.detail = null
      this.comment = ''
    }
    this.detailFailed = false
    await perf.view('issue.open', async () => {
      const detail = await app.guard(() => issueApi.get(repo, issue.source, issue.id))
      if (this.selected !== key || repo !== app.selectedPath) return
      if (detail) this.detail = detail
      else this.detailFailed = true
    })
  }

  /** Loads the selected issue again (after a failure). */
  async retrySelected() {
    const issue = this.list.find((i) => issueKey(i) === this.selected)
    if (issue) await this.select(issue)
  }

  /** Runs a change once at a time, timing it and reporting failures. */
  private async write<T>(name: string, work: (repo: string) => Promise<T>): Promise<T | undefined> {
    const repo = app.selectedPath
    if (!repo || this.saving) return undefined
    this.saving = true
    try {
      return await app.track(perf.time('action', name, () => work(repo)))
    } catch (e) {
      app.error = describeError(e)
      return undefined
    } finally {
      this.saving = false
    }
  }

  openNew() {
    this.draft = { mode: 'new', source: 'local', id: null, title: '', body: '' }
  }

  openEdit() {
    const i = this.detail?.issue
    if (i) this.draft = { mode: 'edit', source: i.source, id: i.id, title: i.title, body: i.body }
  }

  closeDraft() {
    this.draft = null
  }

  async saveDraft() {
    const d = this.draft
    if (!d || !d.title.trim()) return
    const saved =
      d.mode === 'new'
        ? await this.write('issue.create', (repo) => issueApi.create(repo, d.source, d.title, d.body))
        : await this.write('issue.update', (repo) => issueApi.update(repo, d.source, d.id ?? 0, { title: d.title, body: d.body }))
    if (!saved) return
    this.draft = null
    this.selected = issueKey(saved)
    if (d.mode === 'new') {
      if (this.filter !== 'all' && this.filter !== saved.source) this.filter = 'all'
      if (this.stateFilter === 'closed') this.stateFilter = 'open'
      app.toast(t('issues.created', { title: saved.title }))
    }
    await this.load()
  }

  private async change(change: IssueUpdate, done: string) {
    const i = this.detail?.issue
    if (!i) return
    const saved = await this.write('issue.update', (repo) => issueApi.update(repo, i.source, i.id, change))
    if (!saved) return
    app.toast(done)
    await this.load()
  }

  close() {
    return this.change({ state: 'closed' }, t('issues.closedDone'))
  }

  reopen() {
    return this.change({ state: 'open' }, t('issues.reopenedDone'))
  }

  async addComment() {
    const i = this.detail?.issue
    const body = this.comment
    const repo = app.selectedPath
    if (!i || !body.trim()) return
    const key = issueKey(i)
    const saved = await this.write('issue.comment', (path) => issueApi.comment(path, i.source, i.id, body))
    // The user may have moved on to another issue or repository meanwhile.
    if (!saved || this.selected !== key || app.selectedPath !== repo) return
    if (this.comment === body) this.comment = ''
    await this.select(i)
  }

  async remove() {
    const i = this.detail?.issue
    if (!i || i.source !== 'local') return
    if (!(await confirmDialog(t('issues.deleteConfirm', { title: i.title }), t('issues.delete')))) return
    const done = await this.write('issue.delete', (repo) => issueApi.remove(repo, i.id))
    if (done === undefined) return
    app.toast(t('issues.deleted'))
    this.selected = null
    await this.load()
  }

  /**
   * Creates the local issue on GitHub (with its comments) and removes the local copy.
   * A publish that stopped halfway is resumed by the backend, so a retry is safe.
   */
  async publish() {
    const i = this.detail?.issue
    const repo = this.sources?.githubRepo
    if (!i || i.source !== 'local' || !repo) return
    if (!(await confirmDialog(t('issues.publishConfirm', { title: i.title, repo }), t('issues.publish')))) return
    const issue = await this.write('issue.publish', (path) => issueApi.publish(path, i.id))
    if (issue) {
      app.toast(t('issues.published', { number: issue.id }))
      this.selected = issueKey(issue)
      if (this.filter === 'local') this.filter = 'all'
    }
    // Also after a failure: the local issue may now be partly published.
    await this.load()
  }

  async openInBrowser() {
    const url = this.detail?.issue.url
    if (url) await app.guard(() => api.openGithubUrl(url))
  }
}

export const issues = new IssuesState()
