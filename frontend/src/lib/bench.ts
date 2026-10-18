// Benchmark for `leveret --bench <dir> [--runs N]` (run by scripts/benchmark.ps1).
// Walks through the screens of every registered repository the way a user would,
// `runs` times, and saves every timing (frontend and backend) to `<dir>/perf.json`.
// The steps go through the same store methods as the UI, so their `perf.view`
// spans are what gets measured. Add a step here for every new screen.
import { agents } from './agents.svelte'
import { api, syncBackendPerf, usingMock, type BenchRequest } from './api'
import { blame } from './blame.svelte'
import { branches } from './branches.svelte'
import { settings } from './settings.svelte'
import { changes } from './changes.svelte'
import { history } from './history.svelte'
import { issues } from './issues.svelte'
import { perf, summarize } from './perf'
import { stash } from './stash.svelte'
import { app } from './state.svelte'

/** Commits and files opened per repository and run. */
const PICKS = 8

async function benchRepo(path: string) {
  // Start from nothing so the first screen is measured with a cold view.
  await app.selectRepo(null)
  await app.selectRepo(path)

  for (const f of app.files.slice(0, PICKS)) await changes.selectFile(f.path)
  await app.refresh()

  await app.setTab('history')
  for (const c of history.log.slice(1, PICKS + 1)) await history.selectCommit(c.hash)
  const files = history.detail?.files ?? []
  for (const f of files.slice(1, PICKS)) await history.selectCommitFile(f.path)

  const list = document.querySelector<HTMLElement>('.column-body .viewport')
  if (list) {
    for (let i = 0; i < 5; i++) {
      await perf.view('history.scroll', async () => {
        list.scrollTop += list.clientHeight * 3
      })
    }
    list.scrollTop = 0
  }
  await history.setFilter('fix', true)
  await history.setFilter('', true)
  const file = history.detail?.files[0]?.path
  if (file) await history.showFileHistory(file)
  await history.closeFileHistory()
  if (file) await blame.open(file, history.selectedCommit)
  blame.close()
  await history.loadMoreLog()
  await history.setScope('all')
  await history.setScope('head')

  await branches.openBranches()
  await branches.setDialogTab('tags')
  branches.closeBranches()
  await settings.openSettings()
  settings.close()
  await agents.openAgent()
  agents.closeAgent()
  await app.setTab('issues')
  for (const i of issues.list.slice(0, PICKS)) await issues.select(i)
  await app.setTab('changes')
  if (stash.stashes[0]) await stash.open(stash.stashes[0])
  stash.close()

  if (app.status?.hasRemote) await app.remote('fetch', { quiet: true })
}

/** Runs the benchmark, saves the report and exits the app. */
export async function runBenchRequest(req: BenchRequest): Promise<void> {
  let code = 0
  try {
    const repos = app.repos.map((r) => r.path)
    if (repos.length === 0) throw new Error('no repositories are registered')
    for (let run = 0; run < req.runs; run++) {
      for (const path of repos) await benchRepo(path)
    }
    await syncBackendPerf()
    const samples = perf.all()
    const report = {
      version: 1,
      createdAt: new Date().toISOString(),
      mock: usingMock(),
      runs: req.runs,
      repos: repos.length,
      userAgent: navigator.userAgent,
      summary: summarize(samples),
      samples,
    }
    console.info(`saved ${await api.benchSave(JSON.stringify(report))}`)
  } catch (e) {
    console.error('benchmark failed', e)
    code = 1
  }
  await api.exitApp(code)
}
