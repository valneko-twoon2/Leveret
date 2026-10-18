// Screenshot capture. Each scene (lib/scenes/) puts the UI into a state worth capturing.
//
// - In the app: `leveret --capture <dir> [--mock] [--scenes a,b]` runs every
//   scene off-screen and saves `<dir>/<scene>.png` (see backend/src/capture.rs).
// - In a browser: `?scene=<id>` applies one scene and sets
//   `<html data-capture-ready>` when it is done (used by scripts/screenshot.ps1).
import { api, usingMock, type CaptureRequest } from './api'
import { t } from './i18n/index.svelte'
import { app } from './state.svelte'
import { appScenes } from './scenes/app'
import { settle, type Scene } from './scenes/helpers'
import { toolScenes } from './scenes/tools'

export const scenes: Scene[] = [...appScenes, ...toolScenes]


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
  // Scenes may be picked individually, so start from a loaded app.
  await app.init()
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
