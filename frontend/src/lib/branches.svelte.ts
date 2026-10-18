// The branches dialog (with its tags tab, see tags.svelte.ts), the "new branch" /
// rename dialogs and branch actions (switch, merge, rebase, rename, upstream, delete,
// create) with the menu the dialog and the sidebar tree share. Builds on `app` (state.svelte.ts).
import { api, isAppError, type Branch } from './api'
import { confirmDialog } from './dialogs'
import { history } from './history.svelte'
import { t } from './i18n/index.svelte'
import { type MenuItem } from './menu.svelte'
import { perf } from './perf'
import { remotes } from './remotes.svelte'
import { app, describeError } from './state.svelte'
import { tags } from './tags.svelte'

export interface NewBranchRequest {
  /** Commit hash or full ref to start from; null for HEAD. */
  start: string | null
  /** What to show as the start point. */
  label: string | null
  /** Set when the dialog renames this local branch instead of creating one. */
  rename: string | null
}

/** Same rules as `security::branch_name` in the backend (git check-ref-format). */
export function isValidBranchName(name: string): boolean {
  if (!name || name.length > 255 || name === '@' || name === 'HEAD') return false
  if (name.startsWith('-') || name.endsWith('.') || name.includes('..') || name.includes('@{')) return false
  if (/[\s~^:?*[\\\u0000-\u001f\u007f]/.test(name)) return false
  return name.split('/').every((part) => part !== '' && !part.startsWith('.') && !part.endsWith('.lock'))
}

/** Local branch name a remote branch would be checked out as (`origin/feature/x` -> `feature/x`).
    `remoteNames` are the configured remotes (a remote name may itself contain `/`). */
export function localName(branch: Branch, remoteNames: string[] = []): string {
  if (branch.kind !== 'remote') return branch.name
  const owner = remoteNames.filter((n) => branch.name.startsWith(`${n}/`)).sort((a, b) => b.length - a.length)[0]
  return owner ? branch.name.slice(owner.length + 1) : branch.name.slice(branch.name.indexOf('/') + 1)
}

class BranchesState {
  branchesOpen = $state(false)
  /** Which list the branches dialog shows. */
  dialogTab = $state<'branches' | 'tags'>('branches')
  branches = $state.raw<Branch[]>([])
  branchesLoaded = $state(false)
  branchFilter = $state('')
  newBranch = $state.raw<NewBranchRequest | null>(null)
  newBranchName = $state('')
  newBranchCheckout = $state(true)
  /** The branch whose upstream the dialog sets, and the chosen remote branch (full ref; '' for none). */
  upstreamFor = $state.raw<Branch | null>(null)
  upstreamChoice = $state('')

  newBranchError = $derived.by(() => {
    const name = this.newBranchName.trim()
    if (!name) return null
    if (!isValidBranchName(name)) return t('newBranch.invalid')
    if (name !== this.newBranch?.rename && this.branches.some((b) => b.kind === 'local' && b.name === name)) return t('newBranch.exists')
    return null
  })

  /** Forgets everything about the previous repository (called by `app.selectRepo`). */
  resetRepoView() {
    this.branchesOpen = false
    this.branches = []
    this.branchesLoaded = false
    this.newBranch = null
    this.upstreamFor = null
  }

  async openBranches(tab: 'branches' | 'tags' = 'branches') {
    if (!app.selectedPath) return
    this.branchFilter = ''
    this.branchesOpen = true
    await this.setDialogTab(tab)
  }

  async setDialogTab(tab: 'branches' | 'tags') {
    this.dialogTab = tab
    if (tab === 'tags') await tags.load()
    else await perf.view('branches.open', () => this.loadBranches())
  }

  closeBranches() {
    this.branchesOpen = false
  }

  async loadBranches() {
    const repo = app.selectedPath
    if (!repo) return
    const branches = await app.guard(() => api.listBranches(repo))
    if (branches && repo === app.selectedPath) {
      this.branches = branches
      this.branchesLoaded = true
    }
  }

  async switchBranch(b: Branch) {
    if (b.current) {
      this.closeBranches()
      return
    }
    const done = await app.run('branch', (repo) => api.switchBranch(repo, b.fullRef))
    if (done === undefined) return
    this.closeBranches()
    app.toast(t('branches.switched', { branch: localName(b, remotes.remotes.map((r) => r.name)) }))
    await app.refreshAfterChange()
  }

  async mergeBranch(b: Branch) {
    const current = app.status?.branch ?? t('repo.detached')
    if (!(await confirmDialog(t('branches.mergeConfirm', { branch: b.name, current }), t('branches.merge')))) return
    const outcome = await app.run('operation', (repo) => api.mergeBranch(repo, b.fullRef))
    if (!outcome) return
    this.closeBranches()
    await app.handleOutcome(outcome, t('branches.merged', { branch: b.name }))
  }

  async rebaseOnto(b: Branch) {
    const current = app.status?.branch ?? t('repo.detached')
    if (!(await confirmDialog(t('branches.rebaseConfirm', { branch: b.name, current }), t('branches.rebase')))) return
    const outcome = await app.run('operation', (repo) => api.rebaseBranch(repo, b.fullRef))
    if (!outcome) return
    this.closeBranches()
    await app.handleOutcome(outcome, t('branches.rebased', { branch: b.name }))
  }

  async deleteBranch(b: Branch) {
    if (b.kind !== 'local' || b.current) return
    if (!(await confirmDialog(t('branches.deleteConfirm', { branch: b.name }), t('branches.delete')))) return
    const repo = app.selectedPath
    if (!repo || app.busy !== null) return
    app.busy = 'branch'
    try {
      try {
        await app.track(api.deleteBranch(repo, b.name, false))
      } catch (e) {
        if (!isAppError(e) || e.code !== 'branchNotMerged') throw e
        if (!(await confirmDialog(t('branches.forceDeleteConfirm', { branch: b.name }), t('branches.delete')))) return
        await app.track(api.deleteBranch(repo, b.name, true))
      }
      app.toast(t('branches.deleted', { branch: b.name }))
    } catch (e) {
      app.error = describeError(e)
    } finally {
      app.busy = null
    }
    await app.refreshAfterChange()
  }

  /** Deletes a branch on its remote (the local branches stay). */
  async deleteRemoteBranch(b: Branch) {
    if (b.kind !== 'remote') return
    if (!(await confirmDialog(t('branches.deleteRemoteConfirm', { branch: b.name }), t('branches.deleteRemote')))) return
    const done = await app.run('branch', (repo) => api.deleteRemoteBranch(repo, b.fullRef))
    if (done === undefined) return
    app.toast(t('branches.deletedRemote', { branch: b.name }))
    await app.refreshAfterChange()
  }

  /** The menu of a branch (local or remote), shared by the dialog and the sidebar tree. */
  menuItems(b: Branch): MenuItem[] {
    const idle = app.busy === null
    const onBranch = !!app.status?.branch
    const local = b.kind === 'local'
    return [
      { label: t('branches.switch'), action: () => this.switchBranch(b), disabled: !idle || b.current },
      { label: t('branches.showInHistory'), action: () => history.jumpTo(b.oid) },
      'separator',
      { label: t('branches.merge'), action: () => this.mergeBranch(b), disabled: !idle || b.current || !onBranch },
      { label: t('branches.rebase'), action: () => this.rebaseOnto(b), disabled: !idle || b.current || !onBranch },
      { label: t('branches.createFrom'), action: () => this.openNewBranch(b.fullRef, b.name), disabled: !idle },
      'separator',
      ...(local
        ? [
            { label: t('branches.rename'), action: () => this.openRename(b), disabled: !idle },
            { label: t('branches.setUpstream'), action: () => this.openUpstream(b), disabled: !idle },
          ]
        : []),
      { label: t('branches.copyName'), action: () => app.copy(b.name) },
      'separator',
      local
        ? { label: t('branches.delete'), action: () => this.deleteBranch(b), disabled: !idle || b.current, danger: true }
        : { label: t('branches.deleteRemote'), action: () => this.deleteRemoteBranch(b), disabled: !idle, danger: true },
    ]
  }

  /** Opens the "new branch" dialog, starting from HEAD or from `start`. */
  openNewBranch(start: string | null = null, label: string | null = null) {
    if (!app.selectedPath) return
    this.newBranchName = ''
    this.newBranchCheckout = true
    this.newBranch = { start, label, rename: null }
  }

  /** Opens the dialog to rename a local branch. */
  openRename(b: Branch) {
    if (!app.selectedPath || b.kind !== 'local') return
    this.newBranchName = b.name
    this.newBranch = { start: null, label: null, rename: b.name }
  }

  /** Opens the dialog that sets the remote branch a local branch tracks. */
  async openUpstream(b: Branch) {
    if (!app.selectedPath || b.kind !== 'local') return
    if (!this.branchesLoaded) await this.loadBranches()
    this.upstreamChoice = this.branches.find((x) => x.kind === 'remote' && x.name === b.upstream)?.fullRef ?? ''
    this.upstreamFor = b
  }

  closeUpstream() {
    this.upstreamFor = null
  }

  async saveUpstream() {
    const b = this.upstreamFor
    if (!b) return
    const upstream = this.upstreamChoice || null
    const done = await app.run('branch', (repo) => api.setUpstream(repo, b.name, upstream))
    if (done === undefined) return
    this.upstreamFor = null
    app.toast(t(upstream ? 'branches.upstreamSet' : 'branches.upstreamUnset', { branch: b.name }))
    await app.refreshAfterChange()
  }

  closeNewBranch() {
    this.newBranch = null
  }

  async createBranch() {
    const req = this.newBranch
    const name = this.newBranchName.trim()
    if (!req || !name || this.newBranchError) return
    if (req.rename !== null) {
      await this.renameBranch(req.rename, name)
      return
    }
    const checkout = this.newBranchCheckout
    const done = await app.run('branch', (repo) => api.createBranch(repo, name, req.start, checkout))
    if (done === undefined) return
    this.newBranch = null
    this.closeBranches()
    app.toast(t('newBranch.created', { branch: name }))
    await app.refreshAfterChange()
  }

  private async renameBranch(from: string, to: string) {
    if (from === to) {
      this.newBranch = null
      return
    }
    const done = await app.run('branch', (repo) => api.renameBranch(repo, from, to))
    if (done === undefined) return
    this.newBranch = null
    app.toast(t('branches.renamed', { from, to }))
    await app.refreshAfterChange()
  }
}

export const branches = new BranchesState()
