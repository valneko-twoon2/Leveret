// GitHub account, sign-in and clone state. Dialog state lives here (not in the
// components) so capture scenes can drive it.
import { api, type CloneProgress, type DeviceCode, type GithubAccount, type GithubRepo } from './api'
import { t } from './i18n/index.svelte'
import { issues } from './issues.svelte'
import { app, describeError } from './state.svelte'

export type SignInTab = 'browser' | 'token'
export type SignInStatus = 'idle' | 'starting' | 'waiting' | 'expired' | 'denied'
export type CloneTab = 'github' | 'url'

/** Folder name git would create for a clone URL (`.../name.git` -> `name`). */
export function repoNameFromUrl(url: string): string {
  const last = url.trim().replace(/[\\/]+$/, '').split(/[\\/:]/).pop() ?? ''
  return last.replace(/\.git$/i, '')
}

class GithubStore {
  account = $state.raw<GithubAccount | null>(null)
  deviceFlow = $state(false)
  loaded = $state(false)

  // sign-in dialog
  signInOpen = $state(false)
  signInTab = $state<SignInTab>('browser')
  signInStatus = $state<SignInStatus>('idle')
  deviceCode = $state.raw<DeviceCode | null>(null)
  token = $state('')
  busy = $state(false)
  private pollTimer: ReturnType<typeof setTimeout> | null = null
  private signInAttempt = 0

  // clone dialog
  cloneOpen = $state(false)
  cloneTab = $state<CloneTab>('github')
  repos = $state.raw<GithubRepo[]>([])
  reposLoading = $state(false)
  filter = $state('')
  selectedRepo = $state<string | null>(null)
  url = $state('')
  parentDir = $state('')
  cloning = $state(false)
  progress = $state.raw<CloneProgress | null>(null)

  filteredRepos = $derived.by(() => {
    const q = this.filter.trim().toLowerCase()
    return q ? this.repos.filter((r) => r.fullName.toLowerCase().includes(q)) : this.repos
  })

  /** URL and folder name that would be cloned with the current dialog state. */
  cloneSource = $derived.by(() => {
    if (this.cloneTab === 'github') {
      const repo = this.repos.find((r) => r.fullName === this.selectedRepo)
      return repo ? { url: repo.cloneUrl, name: repo.name } : null
    }
    const url = this.url.trim()
    const name = repoNameFromUrl(url)
    return url && name ? { url, name } : null
  })

  async load() {
    try {
      const state = await api.githubState()
      this.account = state.account
      this.deviceFlow = state.deviceFlow
    } catch (e) {
      // Offline or keyring trouble: the app still works without an account.
      console.warn('github state', e)
    } finally {
      this.loaded = true
    }
  }

  // --- sign-in ---------------------------------------------------------------

  openSignIn() {
    this.signInOpen = true
    this.signInTab = this.deviceFlow ? 'browser' : 'token'
    this.signInStatus = 'idle'
    this.deviceCode = null
    this.token = ''
    if (this.signInTab === 'browser') void this.startDeviceFlow()
  }

  closeSignIn() {
    this.signInOpen = false
    this.stopPolling()
    this.token = ''
  }

  private stopPolling() {
    this.signInAttempt++
    if (this.pollTimer) clearTimeout(this.pollTimer)
    this.pollTimer = null
  }

  async startDeviceFlow() {
    this.stopPolling()
    const attempt = this.signInAttempt
    this.signInStatus = 'starting'
    try {
      const code = await api.githubStartSignIn()
      if (attempt !== this.signInAttempt) return
      this.deviceCode = code
      this.signInStatus = 'waiting'
      this.schedulePoll(code, code.interval, attempt)
    } catch (e) {
      this.signInStatus = 'idle'
      app.error = describeError(e)
    }
  }

  private schedulePoll(code: DeviceCode, interval: number, attempt: number) {
    this.pollTimer = setTimeout(async () => {
      if (attempt !== this.signInAttempt) return
      try {
        const result = await api.githubPollSignIn(code.deviceCode)
        if (attempt !== this.signInAttempt) return
        switch (result.status) {
          case 'pending':
            return this.schedulePoll(code, interval, attempt)
          case 'slowDown':
            return this.schedulePoll(code, result.interval, attempt)
          case 'expired':
          case 'denied':
            this.signInStatus = result.status
            return
          case 'success':
            this.signedIn(result.account)
        }
      } catch (e) {
        this.signInStatus = 'idle'
        app.error = describeError(e)
      }
    }, interval * 1000)
  }

  async copyCodeAndOpen() {
    const code = this.deviceCode
    if (!code) return
    try {
      await navigator.clipboard.writeText(code.userCode)
    } catch {
      // Clipboard can be unavailable; the code is shown on screen anyway.
    }
    await this.openUrl(code.verificationUri)
  }

  async openUrl(url: string) {
    try {
      await api.openGithubUrl(url)
    } catch (e) {
      app.error = describeError(e)
    }
  }

  async signInWithToken() {
    const token = this.token.trim()
    if (!token || this.busy) return
    this.busy = true
    try {
      this.signedIn(await api.githubSignInWithToken(token))
    } catch (e) {
      app.error = describeError(e)
    } finally {
      this.busy = false
      this.token = ''
    }
  }

  private signedIn(account: GithubAccount) {
    this.account = account
    this.closeSignIn()
    app.toast(t('github.signedIn', { login: account.login }))
    if (this.cloneOpen) void this.loadRepos()
    // Signing in makes GitHub issues writable (and private ones visible).
    if (issues.loaded) void issues.load()
  }

  async signOut() {
    try {
      await api.githubSignOut()
      this.account = null
      this.repos = []
      if (issues.loaded) void issues.load()
    } catch (e) {
      app.error = describeError(e)
    }
  }

  // --- clone -----------------------------------------------------------------

  async openClone() {
    this.cloneOpen = true
    this.cloneTab = this.account ? 'github' : 'url'
    this.filter = ''
    this.selectedRepo = null
    this.url = ''
    this.progress = null
    if (!this.parentDir) {
      try {
        this.parentDir = await api.defaultCloneDir()
      } catch {
        this.parentDir = ''
      }
    }
    if (this.account && this.repos.length === 0) await this.loadRepos()
  }

  closeClone() {
    if (!this.cloning) this.cloneOpen = false
  }

  async loadRepos() {
    this.reposLoading = true
    try {
      this.repos = await api.githubListRepos()
    } catch (e) {
      app.error = describeError(e)
    } finally {
      this.reposLoading = false
    }
  }

  async clone() {
    const source = this.cloneSource
    const parent = this.parentDir.trim()
    if (!source || !parent || this.cloning) return
    this.cloning = true
    this.progress = { phase: '', percent: 0 }
    const unlisten = await api.onCloneProgress((p) => (this.progress = p))
    try {
      const entry = await api.cloneRepo(source.url, parent, source.name)
      this.cloning = false
      this.cloneOpen = false
      app.toast(t('clone.done', { name: entry.name }))
      await app.reloadRepos(entry.path)
    } catch (e) {
      app.error = describeError(e)
    } finally {
      unlisten()
      this.cloning = false
      this.progress = null
    }
  }
}

export const github = new GithubStore()
