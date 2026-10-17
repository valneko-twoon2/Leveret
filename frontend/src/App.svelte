<script lang="ts">
  import { onMount } from 'svelte'
  import { getCurrentWebview } from '@tauri-apps/api/webview'
  import { agents } from './lib/agents.svelte'
  import { api, inTauri, syncBackendPerf, useMockBackend } from './lib/api'
  import { runBenchRequest } from './lib/bench'
  import { branches } from './lib/branches.svelte'
  import { runCaptureRequest, runUrlScene } from './lib/capture'
  import { changes } from './lib/changes.svelte'
  import { diffTool } from './lib/difftool.svelte'
  import { afterPaint, perf } from './lib/perf'
  import { github } from './lib/github.svelte'
  import { t } from './lib/i18n/index.svelte'
  import { issues } from './lib/issues.svelte'
  import { layout } from './lib/layout.svelte'
  import { menu } from './lib/menu.svelte'
  import { prefs } from './lib/prefs.svelte'
  import { settings } from './lib/settings.svelte'
  import { tags } from './lib/tags.svelte'
  import { app, describeError } from './lib/state.svelte'
  import AgentDialog from './lib/components/AgentDialog.svelte'
  import BranchesDialog from './lib/components/BranchesDialog.svelte'
  import ChangesPane from './lib/components/ChangesPane.svelte'
  import CloneDialog from './lib/components/CloneDialog.svelte'
  import CommitView from './lib/components/CommitView.svelte'
  import ContextMenu from './lib/components/ContextMenu.svelte'
  import DiffHeader from './lib/components/DiffHeader.svelte'
  import DiffView from './lib/components/DiffView.svelte'
  import ErrorDialog from './lib/components/ErrorDialog.svelte'
  import HistoryPane from './lib/components/HistoryPane.svelte'
  import IssueEditor from './lib/components/IssueEditor.svelte'
  import IssuesPane from './lib/components/IssuesPane.svelte'
  import IssueView from './lib/components/IssueView.svelte'
  import Icon from './lib/components/Icon.svelte'
  import NewBranchDialog from './lib/components/NewBranchDialog.svelte'
  import NewTagDialog from './lib/components/NewTagDialog.svelte'
  import OperationBanner from './lib/components/OperationBanner.svelte'
  import PerfPanel from './lib/components/PerfPanel.svelte'
  import RepoHeader from './lib/components/RepoHeader.svelte'
  import RepoSidebar from './lib/components/RepoSidebar.svelte'
  import RepoTabs from './lib/components/RepoTabs.svelte'
  import SettingsDialog from './lib/components/settings/SettingsDialog.svelte'
  import SignInDialog from './lib/components/SignInDialog.svelte'
  import Splitter from './lib/components/Splitter.svelte'
  import Toasts from './lib/components/Toasts.svelte'
  import Welcome from './lib/components/Welcome.svelte'

  let dragging = $state(false)
  let lastFocusRefresh = 0
  let lastUnexpected = 0

  /** Last line of defence: an unexpected error becomes a toast instead of a broken screen. */
  function reportUnexpected(error: unknown) {
    console.error(error)
    const now = Date.now()
    if (now - lastUnexpected < 2000) return
    lastUnexpected = now
    app.toast(describeError(error).message, 'error')
  }

  /** Startup timings, taken once the first screen is painted. */
  async function recordStartup() {
    await afterPaint()
    perf.record('startup', 'interactive (webview)', performance.now())
    perf.recordPageTimings()
    // From process start; unavailable in the browser preview.
    const uptime = await syncBackendPerf().catch(() => null)
    if (uptime !== null) perf.record('startup', 'interactive', uptime)
  }

  /** With LEVERET_PERF=1, sends the frontend's timings to the backend's perf.jsonl. */
  function forwardTimings(): () => void {
    perf.forward = true
    const timer = setInterval(() => {
      const batch = perf.takeUnsent()
      if (batch.length) void api.perfReport(batch).catch(() => {})
    }, 2000)
    return () => clearInterval(timer)
  }

  // Theme: the user's choice, unless the screenshot / browser preview forces one.
  let forcedTheme = $state<'light' | 'dark' | null>(null)
  $effect(() => {
    const theme = forcedTheme ?? (prefs.theme === 'system' ? null : prefs.theme)
    if (theme) document.documentElement.dataset.theme = theme
    else delete document.documentElement.dataset.theme
  })

  onMount(() => {
    layout.fit(window.innerWidth)
    let unlistenDrop: (() => void) | undefined
    let stopForwarding: (() => void) | undefined
    perf.observeLongTasks()
    const onError = (e: ErrorEvent) => reportUnexpected(e.error ?? e.message)
    const onRejection = (e: PromiseRejectionEvent) => {
      e.preventDefault()
      reportUnexpected(e.reason)
    }
    window.addEventListener('error', onError)
    window.addEventListener('unhandledrejection', onRejection)

    void (async () => {
      const options = await api.startupOptions()
      const params = new URLSearchParams(location.search)
      // Forced theme: `--theme` in the app, `?theme=` in the browser.
      const theme = options.theme ?? params.get('theme')
      if (theme === 'light' || theme === 'dark') forcedTheme = theme
      if (options.mock) await useMockBackend()
      if (options.perfLog) stopForwarding = forwardTimings()
      if (options.capture) {
        await runCaptureRequest(options.capture)
        return
      }
      // A benchmark measures repository screens only, not the network sign-in check.
      if (!options.bench) void github.load()
      await app.init()
      await recordStartup()
      if (options.bench) {
        await runBenchRequest(options.bench)
        return
      }
      const scene = params.get('scene')
      if (scene) {
        await runUrlScene(scene)
        return
      }
      app.startAutoFetch()

      if (inTauri) {
        unlistenDrop = await getCurrentWebview().onDragDropEvent(async (event) => {
          const p = event.payload
          if (p.type === 'enter' || p.type === 'over') dragging = true
          else if (p.type === 'leave') dragging = false
          else if (p.type === 'drop') {
            dragging = false
            for (const path of p.paths) await app.addRepo(path)
          }
        })
      }
    })().catch(reportUnexpected)

    return () => {
      unlistenDrop?.()
      stopForwarding?.()
      window.removeEventListener('error', onError)
      window.removeEventListener('unhandledrejection', onRejection)
    }
  })

  async function saveScreenshot() {
    try {
      const path = await api.captureScreen()
      app.toast(path)
    } catch (e) {
      app.error = describeError(e)
    }
  }

  /** Whether a dialog (or menu) is open, so global shortcuts stay out of its way. */
  function overlayOpen(): boolean {
    return menu.current !== null || document.querySelector('[role="dialog"]') !== null
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'F5') {
      e.preventDefault()
      void app.refresh({ explicit: true })
      return
    }
    if (inTauri && e.ctrlKey && e.shiftKey && e.code === 'KeyS') {
      e.preventDefault()
      void saveScreenshot()
      return
    }
    if (e.ctrlKey && e.shiftKey && !e.altKey && e.code === 'KeyD') {
      e.preventDefault()
      app.perfOpen = !app.perfOpen
      return
    }
    if (e.ctrlKey && !e.shiftKey && !e.altKey && e.code === 'Comma' && !overlayOpen()) {
      e.preventDefault()
      void settings.openSettings()
      return
    }
    if (!e.ctrlKey || e.altKey || !app.repo || overlayOpen()) return
    const shortcuts: Record<string, () => unknown> = e.shiftKey
      ? {
          KeyN: () => app.status?.headOid && branches.openNewBranch(),
          KeyF: () => app.status?.hasRemote && app.remote('fetch'),
          KeyL: () => app.status?.upstream && app.remote('pull'),
          KeyP: () => app.status?.hasRemote && app.status.branch && app.remote('push'),
        }
      : {
          Digit1: () => app.setTab('changes'),
          Digit2: () => app.setTab('history'),
          Digit3: () => app.setTab('issues'),
          KeyB: () => branches.openBranches(),
          KeyF: async () => {
            await app.setTab('history')
            document.getElementById('history-search')?.focus()
          },
        }
    const run = shortcuts[e.code]
    if (run) {
      e.preventDefault()
      void run()
    }
  }

  // Pick up changes made outside Leveret when the window is focused again.
  function onfocus() {
    const now = Date.now()
    if (app.selectedPath && app.busy === null && now - lastFocusRefresh > 1000) {
      lastFocusRefresh = now
      void app.refresh()
    }
  }
</script>

<svelte:window {onkeydown} {onfocus} onresize={() => layout.fit(window.innerWidth)} />

{#snippet crashed(error: unknown, reset: () => void)}
  <div class="crashed">
    <Icon name="alert" size={32} />
    <strong>{t('crash.title')}</strong>
    <span>{t('crash.body')}</span>
    <code>{describeError(error).detail}</code>
    <button
      class="btn primary"
      onclick={() => {
        reset()
        void app.refresh()
      }}>{t('crash.retry')}</button
    >
  </div>
{/snippet}

<div class="layout">
  <div class="sidebar" style:width="{layout.sizes.sidebar}px">
    <RepoSidebar />
  </div>
  <Splitter pane="sidebar" />

  <main>
    <svelte:boundary onerror={(e) => console.error(e)} failed={crashed}>
      {#if app.loaded && !app.repo}
        <Welcome />
      {:else if app.repo}
        <RepoHeader />
        <OperationBanner />
        <div class="body">
          <section class="column" style:width="{layout.sizes.column}px">
            <RepoTabs />
            <div class="column-body">
              {#if app.tab === 'changes'}
                <ChangesPane />
              {:else if app.tab === 'history'}
                <HistoryPane />
              {:else}
                <IssuesPane />
              {/if}
            </div>
          </section>
          <Splitter pane="column" />

          <section class="main">
            {#if app.tab === 'changes'}
              {#if changes.selectedChange}
                {@const file = changes.selectedChange}
                <DiffHeader path={file.path} origPath={file.origPath} kind={file.kind} height={37} onopen={() => diffTool.openWorking(file)} />
                <div class="main-body"><DiffView diff={changes.workingDiff} /></div>
              {:else if app.status}
                <div class="empty">
                  <Icon name="file" size={32} />
                  {app.files.length ? t('changes.selectFile') : t('changes.noneBody')}
                </div>
              {/if}
            {:else if app.tab === 'history'}
              <CommitView />
            {:else}
              <IssueView />
            {/if}
          </section>
        </div>
      {/if}
    </svelte:boundary>
  </main>
</div>

{#if dragging}
  <div class="drop"><div>{t('repos.dropHint')}</div></div>
{/if}

{#if branches.branchesOpen}
  <BranchesDialog />
{/if}
{#if branches.newBranch}
  <NewBranchDialog />
{/if}

{#if tags.newTag}
  <NewTagDialog />
{/if}
{#if issues.draft}
  <IssueEditor />
{/if}
{#if settings.open}
  <SettingsDialog />
{/if}
{#if agents.agentOpen}
  <AgentDialog />
{/if}
{#if github.cloneOpen}
  <CloneDialog />
{/if}
{#if github.signInOpen}
  <SignInDialog />
{/if}
{#if app.perfOpen}
  <PerfPanel />
{/if}
<ErrorDialog />
<ContextMenu />
<Toasts />

<style>
  .layout {
    display: flex;
    height: 100%;
  }

  .sidebar {
    flex: none;
    min-width: 0;
    min-height: 0;
  }

  main {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
  }

  .body {
    flex: 1;
    display: flex;
    min-height: 0;
  }

  .column {
    display: flex;
    flex-direction: column;
    flex: none;
    min-width: 0;
  }

  .column-body {
    flex: 1;
    min-height: 0;
  }

  .main {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
  }

  .main-body {
    flex: 1;
    min-height: 0;
  }

  .crashed {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 10px;
    padding: 24px;
    color: var(--fg-muted);
    text-align: center;
  }

  .crashed strong {
    color: var(--fg);
    font-size: 15px;
  }

  .crashed code {
    max-width: 640px;
    max-height: 120px;
    overflow: auto;
    font-size: 12px;
    color: var(--fg-subtle);
    white-space: pre-wrap;
  }

  .drop {
    position: fixed;
    inset: 0;
    z-index: 80;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
    background: color-mix(in srgb, var(--accent) 12%, transparent);
    pointer-events: none;
  }

  .drop div {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    border: 3px dashed var(--accent);
    border-radius: 16px;
    font-size: 18px;
    font-weight: 600;
    color: var(--accent);
  }
</style>
