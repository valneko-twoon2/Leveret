// The remotes of the current repository: the list (also shown in the sidebar tree)
// and the dialog to add, rename, re-point and remove them. Builds on `app` (state.svelte.ts).
import { api, type Remote } from './api'
import { isValidBranchName } from './branches.svelte'
import { confirmDialog } from './dialogs'
import { t } from './i18n/index.svelte'
import { app } from './state.svelte'

/** Same rules as `security::clone_url` in the backend (https, ssh and scp-like only). */
export function isValidRemoteUrl(url: string): boolean {
  if (!url || url.length > 2048 || /[\s\u0000-\u001f\u007f]/.test(url) || url.startsWith('-') || url.includes('::')) return false
  if (url.startsWith('https://') || url.startsWith('ssh://')) return true
  const at = url.indexOf('@')
  const colon = url.indexOf(':')
  return at > 0 && colon > at + 1 && !url.slice(0, colon).includes('/')
}

/** Same rules as `security::remote_name` in the backend. */
export function isValidRemoteName(name: string): boolean {
  return name.length <= 64 && !name.includes('/') && isValidBranchName(name)
}

/** What the dialog's form edits: a new remote, or an existing one. */
export interface RemoteDraft {
  /** null: a new remote. */
  original: string | null
  name: string
  url: string
  pushUrl: string
}

class RemotesState {
  remotes = $state.raw<Remote[]>([])
  loaded = $state(false)
  dialogOpen = $state(false)
  draft = $state<RemoteDraft | null>(null)

  draftError = $derived.by(() => {
    const d = this.draft
    if (!d) return null
    const name = d.name.trim()
    if (name && !isValidRemoteName(name)) return t('remotes.invalidName')
    if (name && name !== d.original && this.remotes.some((r) => r.name === name)) return t('remotes.exists')
    if (d.url.trim() && !isValidRemoteUrl(d.url.trim())) return t('remotes.invalidUrl')
    if (d.pushUrl.trim() && !isValidRemoteUrl(d.pushUrl.trim())) return t('remotes.invalidUrl')
    return null
  })

  canSave = $derived(!!this.draft && !!this.draft.name.trim() && !!this.draft.url.trim() && this.draftError === null && app.busy === null)

  /** Forgets everything about the previous repository (called by `app.selectRepo`). */
  resetRepoView() {
    this.remotes = []
    this.loaded = false
    this.dialogOpen = false
    this.draft = null
  }

  async load() {
    const repo = app.selectedPath
    if (!repo) return
    // Not worth an error dialog: the list just stays as it was.
    const list = await app.track(api.listRemotes(repo)).catch(() => null)
    if (list && repo === app.selectedPath) {
      this.remotes = list
      this.loaded = true
    }
  }

  async openDialog() {
    if (!app.selectedPath) return
    this.draft = null
    this.dialogOpen = true
    await this.load()
  }

  closeDialog() {
    this.dialogOpen = false
    this.draft = null
  }

  startAdd() {
    this.draft = { original: null, name: this.remotes.length ? '' : 'origin', url: '', pushUrl: '' }
  }

  startEdit(remote: Remote) {
    this.draft = { original: remote.name, name: remote.name, url: remote.fetchUrl, pushUrl: remote.pushUrl ?? '' }
  }

  cancelDraft() {
    this.draft = null
  }

  async save() {
    const d = this.draft
    if (!d || !this.canSave) return
    const name = d.name.trim()
    const url = d.url.trim()
    const pushUrl = d.pushUrl.trim()
    const before = this.remotes.find((r) => r.name === d.original)
    const done = await app.run('branch', async (repo) => {
      if (!before) {
        await api.addRemote(repo, name, url)
        if (pushUrl && pushUrl !== url) await api.setRemoteUrl(repo, name, pushUrl, true)
        return
      }
      if (name !== before.name) await api.renameRemote(repo, before.name, name)
      if (url !== before.fetchUrl) await api.setRemoteUrl(repo, name, url, false)
      if (pushUrl !== (before.pushUrl ?? '') && pushUrl) await api.setRemoteUrl(repo, name, pushUrl, true)
    })
    if (done === undefined) return
    this.draft = null
    app.toast(t(before ? 'remotes.saved' : 'remotes.added', { name }))
    await this.reloadAll()
  }

  async remove(remote: Remote) {
    if (!(await confirmDialog(t('remotes.removeConfirm', { name: remote.name }), t('remotes.remove')))) return
    const done = await app.run('branch', (repo) => api.removeRemote(repo, remote.name))
    if (done === undefined) return
    app.toast(t('remotes.removed', { name: remote.name }))
    await this.reloadAll()
  }

  /** Remotes change the remote branches and what the toolbar offers (fetch / push). */
  private async reloadAll() {
    await Promise.all([this.load(), app.refreshAfterChange()])
  }
}

export const remotes = new RemotesState()
