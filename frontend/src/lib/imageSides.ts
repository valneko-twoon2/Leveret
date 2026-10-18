// Where the two pictures of an image diff are, for each place that shows a diff.
import type { ChangeKind, ImageSides } from './api'

interface Changed {
  path: string
  origPath: string | null
  kind: ChangeKind
}

/** Working tree file against HEAD (`head` is null in a repository without commits). */
export function workingImages(file: Changed, head: string | null): ImageSides {
  const existed = head !== null && file.kind !== 'added' && file.kind !== 'untracked'
  return {
    old: existed ? { rev: head, path: file.origPath ?? file.path } : null,
    new: file.kind === 'deleted' ? null : { rev: null, path: file.path },
  }
}

/** A file between two commits (`base` is null for a root commit). */
export function commitImages(file: Changed, base: string | null, target: string): ImageSides {
  return {
    old: base !== null && file.kind !== 'added' && file.kind !== 'untracked' ? { rev: base, path: file.origPath ?? file.path } : null,
    new: file.kind === 'deleted' ? null : { rev: target, path: file.path },
  }
}
