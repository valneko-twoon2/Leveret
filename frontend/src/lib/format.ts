import { i18n, t, type MessageKey } from './i18n/index.svelte'

export function formatDateTime(unixSeconds: number): string {
  return new Intl.DateTimeFormat(i18n.locale, { dateStyle: 'medium', timeStyle: 'short' }).format(unixSeconds * 1000)
}

const UNITS: [Intl.RelativeTimeFormatUnit, number][] = [
  ['year', 365 * 24 * 3600],
  ['month', 30 * 24 * 3600],
  ['week', 7 * 24 * 3600],
  ['day', 24 * 3600],
  ['hour', 3600],
  ['minute', 60],
]

export function formatRelative(unixSeconds: number, now = Date.now() / 1000): string {
  const diff = unixSeconds - now
  const abs = Math.abs(diff)
  if (abs < 60) return t('time.justNow')
  const rtf = new Intl.RelativeTimeFormat(i18n.locale, { numeric: 'auto' })
  for (const [unit, secs] of UNITS) {
    if (abs >= secs) return rtf.format(Math.round(diff / secs), unit)
  }
  return t('time.justNow')
}

export const shortHash = (hash: string) => hash.slice(0, 7)

/** Splits a path into directory (with trailing slash) and file name. */
export function splitPath(path: string): [string, string] {
  const i = path.lastIndexOf('/')
  return i < 0 ? ['', path] : [path.slice(0, i + 1), path.slice(i + 1)]
}

const PHASES: Record<string, MessageKey> = {
  'Counting objects': 'phase.counting',
  'Compressing objects': 'phase.compressing',
  'Writing objects': 'phase.writing',
  'Receiving objects': 'phase.receiving',
  'Resolving deltas': 'phase.resolving',
  'Updating files': 'phase.updating',
  'Enumerating objects': 'phase.enumerating',
}

/** A git progress phase ("Receiving objects") in the UI language; unknown ones as they are. */
export function phaseLabel(phase: string): string {
  const key = PHASES[phase]
  return key ? t(key) : phase
}

/** A byte count for people: `812 B`, `34.5 KB`, `2.1 MB`. */
export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
}
