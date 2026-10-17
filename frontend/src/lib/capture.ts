// Screenshot scenes. Each scene puts the UI into a state worth capturing.
//
// - In the app: `leveret --capture <dir> [--mock] [--scenes a,b]` runs every
//   scene off-screen and saves `<dir>/<scene>.png` (see backend/src/capture.rs).
// - In a browser: `?scene=<id>` applies one scene and sets
//   `<html data-capture-ready>` when it is done (used by scripts/screenshot.ps1).
import { tick } from 'svelte'
import { api, usingMock, type CaptureRequest } from './api'
import { github } from './github.svelte'
import { t } from './i18n/index.svelte'
import { app } from './state.svelte'

export interface Scene {
  id: string
  /** Scenes that need the mock data set (e.g. an empty repository list). */
  mockOnly?: boolean
  run: () => Promise<void>
}

async function openRepo(name: string) {
  const repo = app.repos.find((r) => r.name === name) ?? app.repos[0]
  if (repo) await app.selectRepo(repo.path)
}

export const scenes: Scene[] = [
  {
    id: '01-welcome',
    mockOnly: true,
    run: async () => {
      const mock = await import('./mock')
      mock.reset({ empty: true })
      await app.init()
      mock.reset()
    },
  },
  {
    id: '02-changes',
    run: async () => {
      await app.init()
      await openRepo('Leveret')
      await app.setTab('changes')
      const file = app.files.find((f) => f.kind === 'modified') ?? app.files[0]
      if (file) await app.selectFile(file.path)
    },
  },
  {
    id: '03-commit-message',
    run: async () => {
      await app.setTab('changes')
      const untracked = app.files.find((f) => f.kind === 'untracked')
      if (untracked) {
        if (!app.excluded.has(untracked.path)) app.toggleFile(untracked.path)
        await app.selectFile(untracked.path)
      }
      app.summary = 'ブランチ作成ダイアログを追加'
      app.description = 'ログ画面の右クリックメニューから新しいブランチを作成できるようにした。\n作成後にそのままチェックアウトすることもできる。'
    },
  },
  {
    id: '04-history',
    run: async () => {
      app.summary = ''
      app.description = ''
      await app.setTab('history')
      await app.setScope('head')
      const merge = app.log.find((c) => c.parents.length > 1)
      await app.selectCommit((merge ? app.log[app.log.indexOf(merge) + 1] : app.log[1])?.hash ?? null)
    },
  },
  {
    id: '05-history-all-branches',
    run: async () => {
      await app.setTab('history')
      await app.setScope('all')
      await app.selectCommit(app.log[0]?.hash ?? null)
    },
  },
  {
    id: '06-error',
    run: async () => {
      await app.setTab('changes')
      app.error = {
        message: t('errors.identityMissing'),
        detail: 'Author identity unknown\n\n*** Please tell me who you are.',
      }
    },
  },
  {
    id: '07-sign-in',
    mockOnly: true,
    run: async () => {
      app.error = null
      await github.load()
      github.openSignIn()
    },
  },
  {
    id: '08-clone',
    mockOnly: true,
    run: async () => {
      github.closeSignIn()
      const mock = await import('./mock')
      mock.reset({ signedIn: true })
      await github.load()
      await github.openClone()
      github.selectedRepo = github.repos.find((r) => r.private)?.fullName ?? null
    },
  },
  {
    id: '09-cloning',
    mockOnly: true,
    run: async () => {
      // Show the progress state without actually waiting for a clone.
      github.cloning = true
      github.progress = { phase: 'Receiving objects', percent: 62 }
    },
  },
]

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms))
// requestAnimationFrame may never fire in headless browsers, so don't wait on it forever.
const frame = () => Promise.race([new Promise((r) => requestAnimationFrame(() => r(null))), sleep(100)])

/** Waits until all loads have finished and the DOM has been painted. */
export async function settle(): Promise<void> {
  for (let i = 0; i < 100 && app.pending > 0; i++) await sleep(20)
  await tick()
  await frame()
  await frame()
  await sleep(150)
}

function selectScenes(ids: string[] | null): Scene[] {
  const list = scenes.filter((s) => !s.mockOnly || usingMock())
  return ids ? list.filter((s) => ids.includes(s.id)) : list
}

/** Runs the scenes requested on the command line and exits the app. */
export async function runCaptureRequest(req: CaptureRequest): Promise<void> {
  let code = 0
  const selected = selectScenes(req.scenes)
  if (selected.length === 0) {
    console.error(`no matching scenes: ${req.scenes?.join(',')}`)
    code = 2
  }
  for (const scene of selected) {
    try {
      await scene.run()
      await settle()
      const path = `${req.dir}\\${scene.id}.png`
      await api.captureScreen(path)
      console.info(`captured ${path}`)
    } catch (e) {
      console.error(`capture failed: ${scene.id}`, e)
      code = 1
    }
    app.error = null
  }
  await api.exitApp(code)
}

/** Applies the scene named by `?scene=` (runs earlier scenes first so state builds up). */
export async function runUrlScene(id: string): Promise<void> {
  const list = selectScenes(null)
  const index = list.findIndex((s) => s.id === id)
  const target = list[index]
  if (target) {
    // Scenes after the first build on the previous ones (except the welcome screen).
    const chain = target.mockOnly ? [target] : list.slice(0, index + 1).filter((s) => !s.mockOnly)
    for (const scene of chain) {
      await scene.run()
      await settle()
    }
  }
  document.documentElement.dataset.captureReady = 'true'
}
