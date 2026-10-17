// Global keyboard shortcuts (the window's keydown handler in App.svelte). Their
// display strings are in locales/ja/app.ts `shortcuts`, listed in the README.
import { api, inTauri } from './api'
import { branches } from './branches.svelte'
import { menu } from './menu.svelte'
import { settings } from './settings.svelte'
import { app, describeError } from './state.svelte'

/** Whether a dialog (or menu) is open, so global shortcuts stay out of its way. */
export function overlayOpen(): boolean {
  return menu.current !== null || document.querySelector('[role="dialog"]') !== null
}

async function saveScreenshot() {
  try {
    const path = await api.captureScreen()
    app.toast(path)
  } catch (e) {
    app.error = describeError(e)
  }
}

/** Ctrl+<key> while a repository is open. */
const CTRL: Record<string, () => unknown> = {
  Digit1: () => app.setTab('changes'),
  Digit2: () => app.setTab('history'),
  Digit3: () => app.setTab('issues'),
  KeyB: () => branches.openBranches(),
  KeyF: async () => {
    await app.setTab('history')
    document.getElementById('history-search')?.focus()
  },
}

/** Ctrl+Shift+<key> while a repository is open. */
const CTRL_SHIFT: Record<string, () => unknown> = {
  KeyN: () => app.status?.headOid && branches.openNewBranch(),
  KeyF: () => app.status?.hasRemote && app.remote('fetch'),
  KeyL: () => app.status?.upstream && app.remote('pull'),
  KeyP: () => app.status?.hasRemote && app.status.branch && app.remote('push'),
}

export function handleShortcut(e: KeyboardEvent) {
  const run = (action: () => unknown) => {
    e.preventDefault()
    void action()
  }
  if (e.key === 'F5') return run(() => app.refresh({ explicit: true }))
  if (inTauri && e.ctrlKey && e.shiftKey && e.code === 'KeyS') return run(saveScreenshot)
  if (e.ctrlKey && e.shiftKey && !e.altKey && e.code === 'KeyD') return run(() => (app.perfOpen = !app.perfOpen))
  if (e.ctrlKey && !e.shiftKey && !e.altKey && e.code === 'Comma' && !overlayOpen()) return run(() => settings.openSettings())
  if (!e.ctrlKey || e.altKey || !app.repo || overlayOpen()) return
  const action = (e.shiftKey ? CTRL_SHIFT : CTRL)[e.code]
  if (action) run(action)
}
