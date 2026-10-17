// Native dialogs in Tauri, browser fallbacks in the UI preview.
import { ask, open } from '@tauri-apps/plugin-dialog'
import { inTauri } from './api'
import { t } from './i18n/index.svelte'

export async function pickFolder(title: string): Promise<string | null> {
  if (!inTauri) return window.prompt(title, 'E:\\Code\\sample-repo')
  const picked = await open({ directory: true, multiple: false, title })
  return typeof picked === 'string' ? picked : null
}

/**
 * Asks the user to confirm. `title` also labels the confirm button (a verb such as
 * "ブランチを削除"), so the choice reads clearly instead of a bare yes / no.
 */
export async function confirmDialog(message: string, title: string): Promise<boolean> {
  if (!inTauri) return window.confirm(message)
  return ask(message, { title, kind: 'warning', okLabel: title, cancelLabel: t('common.cancel') })
}
