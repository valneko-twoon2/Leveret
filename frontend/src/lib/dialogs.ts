// Native dialogs in Tauri, browser fallbacks in the UI preview.
import { ask, open } from '@tauri-apps/plugin-dialog'
import { inTauri } from './api'

export async function pickFolder(title: string): Promise<string | null> {
  if (!inTauri) return window.prompt(title, 'E:\\Code\\sample-repo')
  const picked = await open({ directory: true, multiple: false, title })
  return typeof picked === 'string' ? picked : null
}

export async function confirmDialog(message: string, title: string): Promise<boolean> {
  if (!inTauri) return window.confirm(message)
  return ask(message, { title, kind: 'warning' })
}
