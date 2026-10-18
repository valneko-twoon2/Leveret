// The changes tab: file selection (whole files or picked lines) and the
// working-tree diff, the commit form, discard / resolve / ignore and stashes.
// Builds on `app` (state.svelte.ts), which loads the status these act on.
import { SvelteMap, SvelteSet } from 'svelte/reactivity'
import { api, isAppError, type FileChange, type FileDiff, type LineSelection, type RepoStatus, type StashEntry } from './api'
import type { LinePicker } from './components/DiffView.svelte'
import { confirmDialog } from './dialogs'
import { t } from './i18n/index.svelte'
import { menu } from './menu.svelte'
import { rememberMessage } from './messageHistory'
import { perf } from './perf'
import { prefs } from './prefs.svelte'
import { app, describeError } from './state.svelte'

/** `hunk:line` of a line in a diff. */
const lineKey = (h: number, l: number) => `${h}:${l}`

/** The added and deleted lines of a diff, as line keys. */
function changeKeys(diff: FileDiff): string[] {
  return diff.hunks.flatMap((hunk, h) =>
    hunk.lines.flatMap((line, l) => (line.kind === 'add' || line.kind === 'delete' ? [lineKey(h, l)] : [])),
  )
}

/** A file of which only some lines are checked: the lines left out, for the diff with `token`. */
interface LinePicks {
  token: string
  untracked: boolean
  changes: string[]
  excluded: SvelteSet<string>
}

class ChangesState {
  /** Paths unchecked by the user; everything else is included in the commit. */
  excluded = new SvelteSet<string>()
  /** Files with only some lines checked. */
  picks = new SvelteMap<string, LinePicks>()
  selectedFile = $state<string | null>(null)
  workingDiff = $state.raw<FileDiff | null>(null)
  summary = $state('')
  description = $state('')
  amend = $state(false)
  stashes = $state.raw<StashEntry[]>([])

  includedFiles = $derived(app.files.filter((f) => !this.excluded.has(f.path)))
  selectedChange = $derived(app.files.find((f) => f.path === this.selectedFile) ?? null)

  canCommit = $derived(
    app.busy === null &&
      !app.commitBlocked &&
      this.summary.trim() !== '' &&
      (this.amend ? app.canRewriteHead : this.includedFiles.length > 0),
  )

  /** Message put into the form by "amend", so turning amend off can clear it again. */
  private amendPrefill: { summary: string; description: string } | null = null

  /** Forgets everything about the previous repository (called by `app.selectRepo`). */
  resetRepoView() {
    this.excluded.clear()
    this.picks.clear()
    this.selectedFile = null
    this.workingDiff = null
    this.summary = ''
    this.description = ''
    this.amend = false
    this.amendPrefill = null
    this.stashes = []
  }

  /** Brings the tab in line with a freshly loaded status (called by `app.loadStatus`). */
  async syncWithStatus(status: RepoStatus) {
    if (this.amend && !app.canRewriteHead) this.setAmend(false)
    // Forget exclusions for files that are gone.
    const present = new Set(status.files.map((f) => f.path))
    for (const p of [...this.excluded]) if (!present.has(p)) this.excluded.delete(p)
    for (const p of [...this.picks.keys()]) if (!present.has(p)) this.picks.delete(p)
    const keep = this.selectedFile && present.has(this.selectedFile) ? this.selectedFile : (status.files[0]?.path ?? null)
    await this.selectFile(keep)
  }

  async loadStashes() {
    const repo = app.selectedPath
    if (!repo) return
    // Not worth an error dialog: the list just stays as it was.
    const stashes = await app.track(api.listStashes(repo)).catch(() => null)
    if (stashes && repo === app.selectedPath) this.stashes = stashes
  }

  async selectFile(path: string | null) {
    this.selectedFile = path
    const repo = app.selectedPath
    const file = app.files.find((f) => f.path === path)
    if (!repo || !file) {
      this.workingDiff = null
      return
    }
    await perf.view('diff.file', async () => {
      const diff = await app.guard(() => api.getWorkingDiff(repo, file))
      if (!diff || this.selectedFile !== path || repo !== app.selectedPath) return
      // Picked lines refer to the diff they were picked in.
      const picks = this.picks.get(file.path)
      if (picks && picks.token !== diff.token) {
        this.picks.delete(file.path)
        app.toast(t('changes.partialReset', { path: file.path }))
      }
      this.workingDiff = diff
    })
  }

  toggleFile(path: string) {
    this.picks.delete(path)
    if (this.excluded.has(path)) this.excluded.delete(path)
    else this.excluded.add(path)
  }

  /** Whether a file is checked: wholly, partly (some lines) or not at all. */
  fileState(path: string): 'all' | 'some' | 'none' {
    return this.excluded.has(path) ? 'none' : this.picks.has(path) ? 'some' : 'all'
  }

  /** Picking lines of the selected file's diff (DiffView in the Changes tab). */
  readonly picker: LinePicker = {
    included: (h, l) => {
      const path = this.selectedFile
      return !!path && !this.excluded.has(path) && !this.picks.get(path)?.excluded.has(lineKey(h, l))
    },
    setLines: (lines, include) => this.setLines(lines, include),
    onmenu: (e, h, l) => this.lineMenu(e, h, l),
  }

  private setLines(lines: [number, number][], include: boolean) {
    const file = this.selectedChange
    const diff = this.workingDiff
    if (!file || !diff?.selectable) return
    const changes = changeKeys(diff)
    let picks = this.picks.get(file.path)
    if (!picks) {
      // Starting from the file's checkbox: everything or nothing included.
      const excluded = new SvelteSet(this.excluded.has(file.path) ? changes : [])
      picks = { token: diff.token, untracked: file.kind === 'untracked', changes, excluded }
    }
    for (const [h, l] of lines) {
      if (include) picks.excluded.delete(lineKey(h, l))
      else picks.excluded.add(lineKey(h, l))
    }
    this.picks.delete(file.path)
    if (picks.excluded.size >= changes.length) {
      this.excluded.add(file.path)
    } else {
      this.excluded.delete(file.path)
      if (picks.excluded.size > 0) this.picks.set(file.path, picks)
    }
  }

  /** Right click on a diff line: discard that line or its hunk. */
  private lineMenu(e: MouseEvent, h: number, l: number | null) {
    const diff = this.workingDiff
    const hunk = diff?.hunks[h]
    if (!diff?.selectable || !hunk) return
    const isChange = (i: number) => hunk.lines[i].kind === 'add' || hunk.lines[i].kind === 'delete'
    const hunkLines = hunk.lines.flatMap((_, i) => (isChange(i) ? [[h, i] as [number, number]] : []))
    const idle = app.busy === null
    menu.open(e, [
      ...(l !== null && isChange(l)
        ? [{ label: t('discard.line'), action: () => this.discardLines([[h, l]]), disabled: !idle, danger: true }]
        : []),
      { label: t('discard.hunk'), action: () => this.discardLines(hunkLines), disabled: !idle, danger: true },
    ])
  }

  /** Discards some lines of the selected file (the file goes to the Recycle Bin first). */
  async discardLines(lines: [number, number][]) {
    const file = this.selectedChange
    const diff = this.workingDiff
    if (!file || !diff || lines.length === 0) return
    if (lines.length >= changeKeys(diff).length) return this.discard([file.path])
    const question = t('discard.linesConfirm', { path: file.path, count: lines.length })
    if (!(await confirmDialog(`${question}\n${t('discard.confirmNote')}`, t('discard.title')))) return
    const selection: LineSelection = { path: file.path, untracked: file.kind === 'untracked', token: diff.token, lines }
    const done = await app.run('discard', (repo) => api.discardLines(repo, selection))
    if (done === undefined) return
    app.toast(t('discard.linesDone', { count: lines.length }))
    await app.loadStatus()
  }

  toggleAll() {
    this.picks.clear()
    if (this.includedFiles.length === app.files.length) {
      for (const f of app.files) this.excluded.add(f.path)
    } else {
      this.excluded.clear()
    }
  }

  /** Turns "amend the last commit" on (pre-filling its message) or off. */
  async setAmend(on: boolean) {
    this.amend = on
    const repo = app.selectedPath
    const head = app.status?.headOid
    if (on && repo && head && !this.summary.trim() && !this.description.trim()) {
      const detail = await app.guard(() => api.getCommit(repo, head))
      if (!detail || !this.amend) return
      const [summary, ...rest] = detail.message.split('\n')
      this.summary = summary
      this.description = rest.join('\n').trim()
      this.amendPrefill = { summary: this.summary, description: this.description }
    } else if (!on && this.amendPrefill) {
      if (this.summary === this.amendPrefill.summary && this.description === this.amendPrefill.description) {
        this.summary = ''
        this.description = ''
      }
      this.amendPrefill = null
    }
  }

  async commit() {
    if (!this.canCommit) return
    const whole = this.includedFiles.filter((f) => !this.picks.has(f.path))
    const paths = whole.flatMap((f: FileChange) => (f.origPath ? [f.path, f.origPath] : [f.path]))
    const partial = this.includedFiles.flatMap((f): LineSelection[] => {
      const picks = this.picks.get(f.path)
      if (!picks) return []
      const lines = picks.changes.filter((k) => !picks.excluded.has(k)).map((k) => k.split(':').map(Number) as [number, number])
      return [{ path: f.path, untracked: picks.untracked, token: picks.token, lines }]
    })
    const summary = this.summary.trim()
    const amend = this.amend
    const request = { summary, description: this.description, paths, amend, partial }
    const repo = app.selectedPath
    const hash = await app.run('commit', (repo) => api.commit(repo, request))
    if (!hash) return
    if (repo) rememberMessage(repo, { summary, description: this.description.trim() })
    this.picks.clear()
    this.summary = ''
    this.description = ''
    this.amend = false
    this.amendPrefill = null
    app.toast(amend ? t('commit.amended') : t('commit.done', { summary }))
    // Push refreshes the views itself when it runs.
    if (this.canPushAfterCommit && prefs.pushAfterCommit && (await app.remote('push'))) return
    await app.refreshAfterChange()
  }

  /** "Push after commit" applies: there is a remote and a branch to push. */
  get canPushAfterCommit(): boolean {
    return !!app.status?.hasRemote && !!app.status.branch
  }

  /** Puts a remembered message back into the form. */
  useMessage(summary: string, description: string) {
    this.summary = summary
    this.description = description
  }

  /** Removes the last commit, keeping its changes and putting its message back into the form. */
  async undoLastCommit() {
    const message = await app.run('commit', (repo) => api.undoLastCommit(repo))
    if (message === undefined) return
    if (!this.summary.trim() && !this.description.trim()) {
      const [summary, ...rest] = message.split('\n')
      this.summary = summary
      this.description = rest.join('\n').trim()
    }
    app.toast(t('commit.undone'))
    await app.setTab('changes')
    await app.refreshAfterChange()
  }

  // --- discard, resolve, ignore ---------------------------------------------

  async discard(paths: string[]) {
    if (paths.length === 0) return
    const question = paths.length === 1 ? t('discard.confirmOne', { path: paths[0] }) : t('discard.confirmMany', { count: paths.length })
    if (!(await confirmDialog(`${question}\n${t('discard.confirmNote')}`, t('discard.title')))) return
    const done = await app.run('discard', (repo) => api.discardChanges(repo, paths))
    if (done === undefined) return
    app.toast(t('discard.done', { count: paths.length }))
    await app.loadStatus()
  }

  /** Marks files as resolved, asking first when conflict markers are still in them. */
  async markResolved(paths: string[]) {
    const repo = app.selectedPath
    if (paths.length === 0 || !repo || app.busy !== null) return
    app.busy = 'operation'
    try {
      try {
        await app.track(api.markResolved(repo, paths, false))
      } catch (e) {
        if (!isAppError(e) || e.code !== 'conflictMarkersRemain') throw e
        if (!(await confirmDialog(t('discard.markersConfirm', { files: e.detail }), t('discard.markResolved')))) return
        await app.track(api.markResolved(repo, paths, true))
      }
      app.toast(t('discard.resolvedDone', { count: paths.length }))
    } catch (e) {
      app.error = describeError(e)
    } finally {
      app.busy = null
    }
    await app.loadStatus()
  }

  /** Resolves conflicted files with one side's version (current files go to the Recycle Bin). */
  async takeSide(paths: string[], side: 'ours' | 'theirs') {
    if (paths.length === 0) return
    const message = t(side === 'ours' ? 'discard.useOursConfirm' : 'discard.useTheirsConfirm', { count: paths.length })
    if (!(await confirmDialog(message, t(side === 'ours' ? 'discard.useOurs' : 'discard.useTheirs')))) return
    const done = await app.run('operation', (repo) => api.resolveConflict(repo, paths, side))
    if (done === undefined) return
    app.toast(t('discard.resolvedDone', { count: paths.length }))
    await app.loadStatus()
  }

  async ignore(path: string, byExtension: boolean) {
    const done = await app.run('discard', (repo) => api.ignorePath(repo, path, byExtension))
    if (done === undefined) return
    app.toast(t('discard.ignored'))
    await app.loadStatus()
  }

  // --- stash ----------------------------------------------------------------

  async stashSave() {
    if (app.files.length === 0) {
      app.toast(t('stash.noChanges'))
      return
    }
    const done = await app.run('stash', (repo) => api.stashSave(repo, this.summary.trim(), prefs.showUntracked))
    if (done === undefined) return
    app.toast(t('stash.saved'))
    await Promise.all([app.loadStatus(), this.loadStashes()])
  }

  async stashPop(index: number) {
    const outcome = await app.run('stash', (repo) => api.stashPop(repo, index))
    if (!outcome) return
    if (outcome === 'conflicts') app.toast(t('operation.stashConflicts'), 'error')
    else app.toast(t('stash.popped'))
    await app.setTab('changes')
    await Promise.all([app.loadStatus(), this.loadStashes()])
  }

  async stashDrop(entry: StashEntry) {
    if (!(await confirmDialog(t('stash.dropConfirm', { message: entry.message }), t('stash.drop')))) return
    const done = await app.run('stash', (repo) => api.stashDrop(repo, entry.index))
    if (done === undefined) return
    app.toast(t('stash.dropped'))
    await this.loadStashes()
  }
}

export const changes = new ChangesState()
