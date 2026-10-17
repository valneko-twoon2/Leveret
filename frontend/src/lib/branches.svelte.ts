// The branches dialog, the "new branch" dialog and branch actions (switch,
// merge, rebase, delete, create). Builds on `app` (state.svelte.ts).
import { api, isAppError, type Branch } from './api'
import { confirmDialog } from './dialogs'
import { t } from './i18n/index.svelte'
import { perf } from './perf'
import { app, describeError } from './state.svelte'

export interface NewBranchRequest {
  /** Commit hash or full ref to start from; null for HEAD. */
  start: string | null
  /** What to show as the start point. */
  label: string | null
}

/** Same rules as `security::branch_name` in the backend (git check-ref-format). */
export function isValidBranchName(name: string): boolean {
  if (!name || name.length > 255 || name === '@' || name === 'HEAD') return false
  if (name.startsWith('-') || name.endsWith('.') || name.includes('..') || name.includes('@{')) return false
  if (/[\s~^:?*[\\\u0000-\u001f\u007f]/.test(name)) return false
  return name.split('/').every((part) => part !== '' && !part.startsWith('.') && !part.endsWith('.lock'))
}

/** Local branch name a remote branch would be checked out as (`origin/feature/x` -> `feature/x`). */
export function localName(branch: Branch): string {
  return branch.kind === 'remote' ? branch.name.slice(branch.name.indexOf('/') + 1) : branch.name
}

class BranchesState {
  branchesOpen = $state(false)
  branches = $state.raw<Branch[]>([])
  branchFilter = $state('')
  newBranch = $state.raw<NewBranchRequest | null>(null)
  newBranchName = $state('')
  newBranchCheckout = $state(true)

  newBranchError = $derived.by(() => {
    const name = this.newBranchName.trim()
    if (!name) return null
    if (!isValidBranchName(name)) return t('newBranch.invalid')
    if (this.branches.some((b) => b.kind === 'local' && b.name === name)) return t('newBranch.exists')
    return null
  })

  /** Forgets everything about the previous repository (called by `app.selectRepo`). */
  resetRepoView() {
    this.branchesOpen = false
    this.branches = []
    this.newBranch = null
  }

  async openBranches() {
    if (!app.selectedPath) return
    this.branchFilter = ''
    this.branchesOpen = true
    await perf.view('branches.open', () => this.loadBranches())
  }

  closeBranches() {
    this.branchesOpen = false
  }

  async loadBranches() {
    const repo = app.selectedPath
    if (!repo) return
    const branches = await app.guard(() => api.listBranches(repo))
    if (branches && repo === app.selectedPath) this.branches = branches
  }

  async switchBranch(b: Branch) {
    if (b.current) {
      this.closeBranches()
      return
    }
    const done = await app.run('branch', (repo) => api.switchBranch(repo, b.fullRef))
    if (done === undefined) return
    this.closeBranches()
    app.toast(t('branches.switched', { branch: localName(b) }))
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

  /** Opens the "new branch" dialog, starting from HEAD or from `start`. */
  openNewBranch(start: string | null = null, label: string | null = null) {
    if (!app.selectedPath) return
    this.newBranchName = ''
    this.newBranchCheckout = true
    this.newBranch = { start, label }
  }

  closeNewBranch() {
    this.newBranch = null
  }

  async createBranch() {
    const req = this.newBranch
    const name = this.newBranchName.trim()
    if (!req || !name || this.newBranchError) return
    const checkout = this.newBranchCheckout
    const done = await app.run('branch', (repo) => api.createBranch(repo, name, req.start, checkout))
    if (done === undefined) return
    this.newBranch = null
    this.closeBranches()
    app.toast(t('newBranch.created', { branch: name }))
    await app.refreshAfterChange()
  }
}

export const branches = new BranchesState()
