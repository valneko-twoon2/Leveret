import ja from './locales/ja'

/** Message catalog shape, derived from the Japanese catalog. */
type Catalog<T> = { [K in keyof T]: T[K] extends string ? string : Catalog<T[K]> }
export type Messages = Catalog<typeof ja>

/** All dotted message keys, e.g. `commit.button`. */
type Keys<T, P extends string = ''> = {
  [K in keyof T & string]: T[K] extends string ? `${P}${K}` : Keys<T[K], `${P}${K}.`>
}[keyof T & string]
export type MessageKey = Keys<Messages>

// To add a language: create locales/<code>.ts exporting a `Messages` object
// and register it here.
const catalogs = { ja } satisfies Record<string, Messages>
export type Locale = keyof typeof catalogs
export const locales = Object.keys(catalogs) as Locale[]

const STORAGE_KEY = 'leveret.locale'

function initialLocale(): Locale {
  try {
    const saved = localStorage.getItem(STORAGE_KEY)
    if (saved && saved in catalogs) return saved as Locale
  } catch {
    // storage unavailable
  }
  for (const lang of navigator.languages) {
    const code = lang.split('-')[0]
    if (code in catalogs) return code as Locale
  }
  return 'ja'
}

let current = $state<Locale>(initialLocale())

export const i18n = {
  get locale(): Locale {
    return current
  },
  set locale(value: Locale) {
    current = value
    document.documentElement.lang = value
    try {
      localStorage.setItem(STORAGE_KEY, value)
    } catch {
      // storage unavailable
    }
  },
}

function lookup(catalog: Messages, key: string): string | undefined {
  let node: unknown = catalog
  for (const part of key.split('.')) {
    if (typeof node !== 'object' || node === null) return undefined
    node = (node as Record<string, unknown>)[part]
  }
  return typeof node === 'string' ? node : undefined
}

/** Translates `key`, replacing `{name}` placeholders with `params`. */
export function t(key: MessageKey, params?: Record<string, string | number>): string {
  const text = lookup(catalogs[current], key) ?? lookup(catalogs.ja, key) ?? key
  if (!params) return text
  return text.replace(/\{(\w+)\}/g, (m, name: string) => (name in params ? String(params[name]) : m))
}
