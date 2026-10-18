// Shared pieces of the screenshot scenes (see capture.ts).
import { tick } from 'svelte'
import { agents } from '../agents.svelte'
import { blame } from '../blame.svelte'
import { branches } from '../branches.svelte'
import { github } from '../github.svelte'
import { issues } from '../issues.svelte'
import { menu } from '../menu.svelte'
import { settings } from '../settings.svelte'
import { stash } from '../stash.svelte'
import { app } from '../state.svelte'
import { tags } from '../tags.svelte'

export interface Scene {
  id: string
  /** Scenes that need the mock data set (e.g. an empty repository list). */
  mockOnly?: boolean
  run: () => Promise<void>
}

export async function openRepo(name: string) {
  const repo = app.repos.find((r) => r.name === name) ?? app.repos[0]
  if (repo) await app.selectRepo(repo.path)
}

/** Closes dialogs and menus left open by the previous scene. */
export function closeOverlays() {
  menu.close()
  stash.close()
  blame.close()
  tags.closeNewTag()
  settings.close()
  agents.closeAgent()
  app.perfOpen = false
  issues.closeDraft()
  app.error = null
  branches.closeNewBranch()
  branches.closeBranches()
  github.closeSignIn()
  github.cloning = false
  github.progress = null
  github.closeClone()
}

export function openContextMenuOnSelectedRow() {
  const row = document.querySelector<HTMLElement>('.row.selected')
  if (!row) return
  const r = row.getBoundingClientRect()
  row.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, clientX: r.left + 140, clientY: r.top + r.height / 2 }))
}

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
