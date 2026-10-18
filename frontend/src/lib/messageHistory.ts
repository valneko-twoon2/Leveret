// Commit messages recently committed in Leveret, per repository, offered again
// from the commit form. Kept in localStorage like the display preferences.

const STORAGE_KEY = 'leveret.commitMessages'
const MAX = 20

export interface RecentMessage {
  summary: string
  description: string
}

function readAll(): Record<string, RecentMessage[]> {
  try {
    const value: unknown = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? '{}')
    return value && typeof value === 'object' ? (value as Record<string, RecentMessage[]>) : {}
  } catch {
    return {}
  }
}

function writeAll(all: Record<string, RecentMessage[]>) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(all))
  } catch {
    // Not remembered; the form still works.
  }
}

/** Newest first. */
export function recentMessages(repo: string): RecentMessage[] {
  const list = readAll()[repo]
  return Array.isArray(list)
    ? list.filter((m) => typeof m?.summary === 'string' && typeof m.description === 'string')
    : []
}

export function rememberMessage(repo: string, message: RecentMessage) {
  const all = readAll()
  const same = (m: RecentMessage) => m.summary === message.summary && m.description === message.description
  all[repo] = [message, ...recentMessages(repo).filter((m) => !same(m))].slice(0, MAX)
  writeAll(all)
}

export function forgetMessages(repo: string) {
  const all = readAll()
  delete all[repo]
  writeAll(all)
}
