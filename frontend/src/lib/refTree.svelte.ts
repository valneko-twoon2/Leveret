// The tree of branches, remotes, tags and stashes under the repository list: which
// parts are expanded, the selected row, and the visible rows built from the stores.
import type { Branch, Remote, StashEntry, Tag } from './api'
import { branches } from './branches.svelte'
import { t } from './i18n/index.svelte'
import { perf } from './perf'
import { remotes } from './remotes.svelte'
import { stash } from './stash.svelte'
import { tags } from './tags.svelte'

const STORAGE_KEY = 'leveret.refTree'

export type RefSection = 'branches' | 'remotes' | 'tags' | 'stashes'

export type RefRow = { key: string; depth: number; label: string } & (
  | { kind: 'section'; section: RefSection; count: number; open: boolean }
  | { kind: 'dir'; open: boolean }
  | { kind: 'remote'; remote: Remote | null; open: boolean }
  | { kind: 'branch'; branch: Branch }
  | { kind: 'tag'; tag: Tag }
  | { kind: 'stash'; stash: StashEntry }
)

interface Node<T> {
  label: string
  /** Path from the root, e.g. `feature/login`; the key of a folder. */
  path: string
  /** null for a folder. */
  item: T | null
  children: Node<T>[]
}

/** Groups `items` into folders by the `/` in their names (folders first, then by name). */
function build<T>(items: T[], nameOf: (item: T) => string): Node<T>[] {
  const root: Node<T> = { label: '', path: '', item: null, children: [] }
  for (const item of items) {
    const parts = nameOf(item).split('/')
    let node = root
    parts.forEach((label, i) => {
      const path = parts.slice(0, i + 1).join('/')
      let child = node.children.find((c) => c.label === label)
      if (!child) {
        child = { label, path, item: null, children: [] }
        node.children.push(child)
      }
      if (i === parts.length - 1) child.item = item
      node = child
    })
  }
  const sort = (nodes: Node<T>[]) => {
    nodes.sort((a, b) => Number(a.children.length === 0) - Number(b.children.length === 0) || a.label.localeCompare(b.label, undefined, { numeric: true }))
    nodes.forEach((n) => sort(n.children))
  }
  sort(root.children)
  return root.children
}

/** The remote a remote branch (`origin/feature/x`) belongs to: the longest configured name that prefixes it. */
export function remoteOf(branch: Branch, names: string[]): string {
  const match = names.filter((n) => branch.name.startsWith(`${n}/`)).sort((a, b) => b.length - a.length)[0]
  return match ?? branch.name.split('/')[0]
}

class RefTreeState {
  /** Expanded parts by key (`section:branches`, `dir:branches:feature`, `remote:origin`). */
  private openKeys = $state<Record<string, boolean>>(this.loadOpen())
  /** The selected row (not necessarily what is checked out). */
  selected = $state<string | null>(null)

  rows = $derived.by((): RefRow[] => {
    const out: RefRow[] = []
    const push = (row: RefRow) => void out.push(row)
    const folders = <T,>(nodes: Node<T>[], scope: string, depth: number, leaf: (item: T, node: Node<T>, depth: number) => void) => {
      for (const node of nodes) {
        if (node.item !== null && node.children.length === 0) {
          leaf(node.item, node, depth)
          continue
        }
        const key = `dir:${scope}:${node.path}`
        const open = this.isOpen(key, true)
        push({ kind: 'dir', key, depth, label: node.label, open })
        if (open) folders(node.children, scope, depth + 1, leaf)
      }
    }
    const section = (name: RefSection, count: number): boolean => {
      const key = `section:${name}`
      const open = this.isOpen(key, name === 'branches')
      push({ kind: 'section', key, depth: 0, label: t(`refTree.${name}`), section: name, count, open })
      return open
    }

    const local = branches.branches.filter((b) => b.kind === 'local')
    if (section('branches', local.length)) {
      folders(build(local, (b) => b.name), 'branches', 1, (branch, node, depth) =>
        push({ kind: 'branch', key: `branch:${branch.fullRef}`, depth, label: node.label, branch }),
      )
    }

    const remoteBranches = branches.branches.filter((b) => b.kind === 'remote')
    const names = remotes.remotes.map((r) => r.name)
    const groups = new Map<string, Branch[]>(remotes.remotes.map((r) => [r.name, []]))
    for (const b of remoteBranches) {
      const owner = remoteOf(b, names)
      groups.set(owner, [...(groups.get(owner) ?? []), b])
    }
    if (section('remotes', groups.size)) {
      for (const [name, list] of [...groups].sort(([a], [b]) => a.localeCompare(b))) {
        const key = `remote:${name}`
        const open = this.isOpen(key, false)
        push({ kind: 'remote', key, depth: 1, label: name, remote: remotes.remotes.find((r) => r.name === name) ?? null, open })
        if (!open) continue
        folders(build(list, (b) => b.name.slice(name.length + 1)), `remote:${name}`, 2, (branch, node, depth) =>
          push({ kind: 'branch', key: `branch:${branch.fullRef}`, depth, label: node.label, branch }),
        )
      }
    }

    if (section('tags', tags.tags.length)) {
      folders(build(tags.tags, (tag) => tag.name), 'tags', 1, (tag, node, depth) =>
        push({ kind: 'tag', key: `tag:${tag.name}`, depth, label: node.label, tag }),
      )
    }

    if (section('stashes', stash.stashes.length)) {
      for (const s of stash.stashes) push({ kind: 'stash', key: `stash:${s.oid}`, depth: 1, label: s.message, stash: s })
    }
    return out
  })

  isOpen(key: string, fallback: boolean): boolean {
    return this.openKeys[key] ?? fallback
  }

  toggle(row: RefRow) {
    if (row.kind !== 'section' && row.kind !== 'dir' && row.kind !== 'remote') return
    this.setOpen(row, !row.open)
  }

  setOpen(row: RefRow, open: boolean) {
    this.openKey(row.key, open)
  }

  openKey(key: string, open: boolean) {
    this.openKeys[key] = open
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(this.openKeys))
    } catch {
      // Remembering is a convenience only.
    }
  }

  private loadOpen(): Record<string, boolean> {
    try {
      const value: unknown = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? '{}')
      if (!value || typeof value !== 'object') return {}
      return Object.fromEntries(Object.entries(value).filter(([, v]) => typeof v === 'boolean'))
    } catch {
      return {}
    }
  }

  /** Loads what the tree shows (branches, remotes and tags; stashes load with every refresh). */
  async load() {
    await perf.view('sidebar.refs', () => Promise.all([branches.loadBranches(), remotes.load(), tags.load()]))
  }
}

export const refTree = new RefTreeState()
