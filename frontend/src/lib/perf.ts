// Timing of what users wait for. The frontend measures backend round trips
// (`ipc`), screen transitions until the result is painted (`view`), user actions
// such as push (`action`), main-thread stalls (`longtask`) and startup; the
// backend adds git, HTTP and agent tool timings (backend/src/perf.rs).
// Shown in the performance panel (Ctrl+Shift+D) and saved by `leveret --bench`.
import { tick } from 'svelte'

export type PerfKind = 'git' | 'http' | 'tool' | 'startup' | 'ipc' | 'view' | 'action' | 'longtask'

export interface PerfSample {
  seq?: number
  /** Unix milliseconds when the work finished. */
  time: number
  kind: PerfKind
  name: string
  ms: number
  ok: boolean
  source: 'ui' | 'backend'
}

/** What a kind of work should take at most (p95). Actions depend on the network, so they have none. */
export const BUDGET_MS: Record<PerfKind, number | null> = {
  view: 100,
  ipc: 200,
  git: 300,
  http: 1500,
  tool: 1000,
  startup: 1500,
  action: null,
  longtask: 50,
}

export interface PerfStat {
  kind: PerfKind
  name: string
  count: number
  failed: number
  median: number
  p95: number
  max: number
  total: number
  budget: number | null
  overBudget: boolean
}

const CAPACITY = 4000

/** Round trips that only read state. Others (commit, push, clone, ...) run hooks or use the network. */
const READ_COMMAND = /^(get_|list_|select_repo$|agent_info$|github_state$)/

/** The budget for one step, or null when it has none. */
export function budgetFor(kind: PerfKind, name: string): number | null {
  if (kind === 'ipc' && !READ_COMMAND.test(name)) return null
  return BUDGET_MS[kind]
}

/** Resolves once the DOM changes made so far have been painted (or after 250 ms in a hidden window). */
export async function afterPaint(): Promise<void> {
  await tick()
  await Promise.race([
    new Promise<void>((resolve) => requestAnimationFrame(() => setTimeout(resolve, 0))),
    new Promise<void>((resolve) => setTimeout(resolve, 250)),
  ])
}

function round(ms: number): number {
  return Math.round(ms * 10) / 10
}

function percentile(sorted: number[], p: number): number {
  return sorted[Math.min(sorted.length - 1, Math.max(0, Math.ceil((p / 100) * sorted.length) - 1))]
}

/** Per-name statistics, slowest (p95) first. */
export function summarize(samples: readonly PerfSample[]): PerfStat[] {
  const groups = new Map<string, PerfSample[]>()
  for (const s of samples) {
    const key = `${s.kind}\u0000${s.name}`
    const list = groups.get(key)
    if (list) list.push(s)
    else groups.set(key, [s])
  }
  const stats: PerfStat[] = []
  for (const list of groups.values()) {
    const ms = list.map((s) => s.ms).sort((a, b) => a - b)
    const { kind, name } = list[0]
    const budget = budgetFor(kind, name)
    const p95 = percentile(ms, 95)
    stats.push({
      kind,
      name,
      count: ms.length,
      failed: list.filter((s) => !s.ok).length,
      median: round(percentile(ms, 50)),
      p95: round(p95),
      max: round(ms[ms.length - 1]),
      total: round(ms.reduce((a, b) => a + b, 0)),
      budget,
      overBudget: budget !== null && p95 > budget,
    })
  }
  return stats.sort((a, b) => b.p95 - a.p95)
}

class Perf {
  private ui: PerfSample[] = []
  private backend: PerfSample[] = []
  private unsent: PerfSample[] = []
  /** Highest backend sequence number seen. */
  lastSeq = 0
  /** Keep samples for `perf.jsonl` (`LEVERET_PERF=1`). */
  forward = false

  record(kind: PerfKind, name: string, ms: number, ok = true) {
    const sample: PerfSample = { time: Date.now(), kind, name, ms: round(ms), ok, source: 'ui' }
    this.ui.push(sample)
    if (this.ui.length > CAPACITY) this.ui.splice(0, this.ui.length - CAPACITY)
    if (this.forward && this.unsent.length < CAPACITY) this.unsent.push(sample)
    const budget = budgetFor(kind, name)
    if (import.meta.env.DEV && budget !== null && ms > budget) {
      console.warn(`[perf] ${kind} ${name}: ${sample.ms} ms (budget ${budget} ms)`)
    }
  }

  /** Measures `work` from start to finish. */
  async time<T>(kind: PerfKind, name: string, work: () => Promise<T>): Promise<T> {
    const start = performance.now()
    let ok = false
    try {
      const result = await work()
      ok = true
      return result
    } finally {
      this.record(kind, name, performance.now() - start, ok)
    }
  }

  /** Measures a screen transition: `work` plus rendering, until the result is painted. */
  view<T>(name: string, work: () => Promise<T>): Promise<T> {
    return this.time('view', name, async () => {
      const result = await work()
      await afterPaint()
      return result
    })
  }

  addBackend(samples: PerfSample[]) {
    for (const s of samples) {
      this.backend.push({ ...s, source: 'backend' })
      this.lastSeq = Math.max(this.lastSeq, s.seq ?? 0)
    }
    if (this.backend.length > CAPACITY) this.backend.splice(0, this.backend.length - CAPACITY)
  }

  /** Samples measured since the last call, for `perf_report`. */
  takeUnsent(): PerfSample[] {
    return this.unsent.splice(0, this.unsent.length)
  }

  all(): PerfSample[] {
    return [...this.ui, ...this.backend].sort((a, b) => a.time - b.time)
  }

  clear() {
    this.ui = []
    this.backend = []
  }

  /** Records main-thread tasks over 50 ms (where the browser supports it). */
  observeLongTasks() {
    try {
      new PerformanceObserver((list) => {
        for (const e of list.getEntries()) this.record('longtask', 'main thread', e.duration)
      }).observe({ type: 'longtask', buffered: true })
    } catch {
      // Not supported: nothing to observe.
    }
  }

  /** Startup milestones of the webview itself (relative to its navigation start). */
  recordPageTimings() {
    const nav = performance.getEntriesByType('navigation')[0] as PerformanceNavigationTiming | undefined
    if (nav) this.record('startup', 'dom ready (webview)', nav.domContentLoadedEventEnd)
    const paint = performance.getEntriesByName('first-contentful-paint')[0]
    if (paint) this.record('startup', 'first paint (webview)', paint.startTime)
  }
}

export const perf = new Perf()
