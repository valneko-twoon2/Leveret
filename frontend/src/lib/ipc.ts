// Transport to the backend. Outside of Tauri (plain browser, e.g.
// `scripts/preview.bat`) or with `--mock`, calls go to the in-memory mock.
// The typed wrappers live in the area files combined by api.ts.
import { invoke, isTauri } from '@tauri-apps/api/core'
import { perf, type PerfSample } from './perf'

/** Error shape returned by the backend (see backend/src/error.rs). */
export interface AppError {
  code: string
  detail: string
}

export function isAppError(e: unknown): e is AppError {
  return typeof e === 'object' && e !== null && 'code' in e && 'detail' in e
}

export interface PerfSnapshot {
  samples: PerfSample[]
  /** Milliseconds since the backend process started. */
  uptimeMs: number
}

export const inTauri = isTauri()
let mock: typeof import('./mock/index') | null = null

/** Routes all following calls to the mock backend. */
export async function useMockBackend(): Promise<void> {
  mock ??= await import('./mock/index')
}

export function usingMock(): boolean {
  return mock !== null
}

/** The mock backend while it is in use (for event subscriptions). */
export function mockBackend(): typeof import('./mock/index') | null {
  return mock
}

/** Commands that are part of the measuring itself. */
const UNTIMED = new Set(['perf_samples', 'perf_report'])

/** Calls a backend command, timing the round trip. */
export function call<T>(cmd: string, args: Record<string, unknown> = {}): Promise<T> {
  const send = () => (mock ? (mock.handle(cmd, args) as Promise<T>) : invoke<T>(cmd, args))
  return UNTIMED.has(cmd) ? send() : perf.time('ipc', cmd, send)
}

/** Subscribes to a backend event. Returns an unsubscribe function. */
export async function listenTo<T>(event: string, handler: (payload: T) => void): Promise<() => void> {
  const { listen } = await import('@tauri-apps/api/event')
  return listen<T>(event, (e) => handler(e.payload))
}

/** Pulls the backend's new timing samples into `perf`. Returns the backend uptime. */
export async function syncBackendPerf(): Promise<number> {
  const snapshot = await call<PerfSnapshot>('perf_samples', { after: perf.lastSeq })
  perf.addBackend(snapshot.samples)
  return snapshot.uptimeMs
}
