// WinMerge, the external diff tool: whether it is installed and opening diffs in it.
import { api, type CommitDetail, type CommitFile, type FileChange } from './api'
import { app } from './state.svelte'

class DiffToolState {
  /** WinMerge is installed; the buttons and menu items only appear then. */
  available = $state(false)
  private checked = false

  /** Looks for WinMerge once, when a diff is first shown. */
  async check() {
    if (this.checked) return
    this.checked = true
    this.available = await api.getDiffTool().catch(() => false)
  }

  /** Uncommitted changes of a file; conflicted files open as a 3-way merge. */
  async openWorking(file: FileChange) {
    const repo = app.selectedPath
    if (repo && this.available) await app.guard(() => api.openWorkingInDiffTool(repo, file))
  }

  async openCommit(detail: CommitDetail, file: CommitFile) {
    const repo = app.selectedPath
    if (repo && this.available) await app.guard(() => api.openCommitInDiffTool(repo, detail, file))
  }
}

export const diffTool = new DiffToolState()
