// Tags: the list in the branches dialog, the "new tag" dialog and tag actions
// (create, push, delete, check out). Builds on `app` (state.svelte.ts).
import { api, type Tag } from './api'
import { isValidBranchName } from './branches.svelte'
import { confirmDialog } from './dialogs'
import { shortHash } from './format'
import { t } from './i18n/index.svelte'
import { perf } from './perf'
import { app } from './state.svelte'

export interface NewTagRequest {
  /** Commit to tag. */
  target: string
  /** What to show as the commit (short hash and subject). */
  label: string
}

class TagsState {
  tags = $state.raw<Tag[]>([])
  loaded = $state(false)
  newTag = $state.raw<NewTagRequest | null>(null)
  newTagName = $state('')
  newTagMessage = $state('')

  newTagError = $derived.by(() => {
    const name = this.newTagName.trim()
    if (!name) return null
    // Tag names follow the same rules as branch names.
    if (!isValidBranchName(name)) return t('tags.invalid')
    if (this.tags.some((tag) => tag.name === name)) return t('tags.exists')
    return null
  })

  /** Forgets everything about the previous repository (called by `app.selectRepo`). */
  resetRepoView() {
    this.tags = []
    this.loaded = false
    this.newTag = null
  }

  async load() {
    const repo = app.selectedPath
    if (!repo) return
    await perf.view('tags.open', async () => {
      const tags = await app.guard(() => api.listTags(repo))
      if (tags && repo === app.selectedPath) {
        this.tags = tags
        this.loaded = true
      }
    })
  }

  /** Opens the "new tag" dialog for a commit (HEAD when omitted). */
  openNewTag(target?: string, subject?: string) {
    const hash = target ?? app.status?.headOid
    if (!app.selectedPath || !hash) return
    this.newTagName = ''
    this.newTagMessage = ''
    this.newTag = { target: hash, label: subject ? `${shortHash(hash)} ${subject}` : shortHash(hash) }
    // Existing names are needed to warn about duplicates.
    if (!this.loaded) void this.load()
  }

  closeNewTag() {
    this.newTag = null
  }

  async create() {
    const req = this.newTag
    const name = this.newTagName.trim()
    if (!req || !name || this.newTagError) return
    const message = this.newTagMessage
    const done = await app.run('tag', (repo) => api.createTag(repo, name, req.target, message))
    if (done === undefined) return
    this.newTag = null
    app.toast(t('tags.created', { tag: name }))
    await this.afterChange()
  }

  async push(tag: Tag) {
    const done = await app.run('tag', (repo) => api.pushTag(repo, tag.name))
    if (done !== undefined) app.toast(t('tags.pushed', { tag: tag.name }))
  }

  /** Deletes a tag; with `remote`, from the remote too (asks first either way). */
  async remove(tag: Tag, remote: boolean) {
    const message = t(remote ? 'tags.deleteRemoteConfirm' : 'tags.deleteConfirm', { tag: tag.name })
    if (!(await confirmDialog(message, t('tags.delete')))) return
    const done = await app.run('tag', (repo) => api.deleteTag(repo, tag.name, remote))
    if (done === undefined) return
    app.toast(t('tags.deleted', { tag: tag.name }))
    await this.afterChange()
  }

  async checkout(tag: Tag) {
    if (!(await confirmDialog(t('tags.checkoutConfirm', { tag: tag.name }), t('tags.checkout')))) return
    const done = await app.run('branch', (repo) => api.checkoutCommit(repo, tag.target))
    if (done === undefined) return
    app.toast(t('tags.checkedOut', { tag: tag.name }))
    await app.refreshAfterChange()
  }

  /** Tags show up in the history and in the list. */
  private async afterChange() {
    await Promise.all([this.load(), app.refreshAfterChange()])
  }
}

export const tags = new TagsState()
