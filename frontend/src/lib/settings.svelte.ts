// The settings screen: which section is shown, the backend's settings (git's
// global config, folders) and the actions behind each control. Display
// preferences themselves live in prefs.svelte.ts.
import { api, type AppSettings, type GitConfigKey } from './api'
import { changes } from './changes.svelte'
import { confirmDialog, pickFolder } from './dialogs'
import { history } from './history.svelte'
import { t } from './i18n/index.svelte'
import { layout } from './layout.svelte'
import { perf } from './perf'
import { prefs } from './prefs.svelte'
import { app } from './state.svelte'

export const SECTIONS = ['general', 'git', 'changes', 'remote', 'agents', 'advanced', 'about', 'licenses'] as const
export type Section = (typeof SECTIONS)[number]

class SettingsState {
  open = $state(false)
  section = $state<Section>('general')
  info = $state.raw<AppSettings | null>(null)

  async openSettings(section: Section = 'general') {
    this.section = section
    this.open = true
    await perf.view('settings.open', () => this.load())
  }

  close() {
    this.open = false
  }

  async load() {
    const info = await app.guard(() => api.getSettings())
    if (info) this.info = info
  }

  /** Saves one global git setting (an empty value unsets it). */
  async saveGit(key: GitConfigKey, value: string) {
    if ((await app.guard(() => api.setGitConfig(key, value.trim()))) === undefined) return
    app.toast(t('settings.git.saved'))
    await this.load()
  }

  async chooseCloneDir() {
    const dir = await pickFolder(t('settings.remote.cloneDir'))
    if (!dir) return
    if ((await app.guard(() => api.setCloneDir(dir))) !== undefined) await this.load()
  }

  async resetCloneDir() {
    if ((await app.guard(() => api.setCloneDir(null))) !== undefined) await this.load()
  }

  async openFolder(which: 'config' | 'logs') {
    await app.guard(() => api.openAppFolder(which))
  }

  async setShowUntracked(show: boolean) {
    prefs.setShowUntracked(show)
    await app.loadStatus()
  }

  /** Diffs are fetched again so the setting shows right away. */
  async setIgnoreWhitespace(ignore: boolean) {
    prefs.set('ignoreWhitespace', ignore)
    await Promise.all([changes.selectFile(changes.selectedFile), history.reloadCommitFile()])
  }

  async setContextLines(lines: number) {
    prefs.set('contextLines', lines)
    await Promise.all([changes.selectFile(changes.selectedFile), history.reloadCommitFile()])
  }

  async resetDisplay() {
    if (!(await confirmDialog(t('settings.advanced.resetConfirm'), t('settings.advanced.reset')))) return
    const untracked = prefs.showUntracked
    prefs.reset()
    layout.resetAll()
    app.perfOpen = false
    app.toast(t('settings.advanced.resetDone'))
    if (untracked !== prefs.showUntracked) await app.loadStatus()
  }
}

export const settings = new SettingsState()
