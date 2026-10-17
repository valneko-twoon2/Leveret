<script lang="ts">
  import { onMount } from 'svelte'
  import { getCurrentWebview } from '@tauri-apps/api/webview'
  import { api, inTauri, useMockBackend } from './lib/api'
  import { runCaptureRequest, runUrlScene } from './lib/capture'
  import { github } from './lib/github.svelte'
  import CloneDialog from './lib/components/CloneDialog.svelte'
  import SignInDialog from './lib/components/SignInDialog.svelte'
  import { t } from './lib/i18n/index.svelte'
  import { app, describeError } from './lib/state.svelte'
  import ChangesPane from './lib/components/ChangesPane.svelte'
  import CommitView from './lib/components/CommitView.svelte'
  import DiffView from './lib/components/DiffView.svelte'
  import ErrorDialog from './lib/components/ErrorDialog.svelte'
  import FileRow from './lib/components/FileRow.svelte'
  import HistoryPane from './lib/components/HistoryPane.svelte'
  import Icon from './lib/components/Icon.svelte'
  import RepoHeader from './lib/components/RepoHeader.svelte'
  import RepoSidebar from './lib/components/RepoSidebar.svelte'
  import Toasts from './lib/components/Toasts.svelte'
  import Welcome from './lib/components/Welcome.svelte'

  let dragging = $state(false)
  let lastFocusRefresh = 0

  onMount(() => {
    let unlistenDrop: (() => void) | undefined

    void (async () => {
      const options = await api.startupOptions()
      const params = new URLSearchParams(location.search)
      // Forced theme: `--theme` in the app, `?theme=` in the browser.
      const theme = options.theme ?? params.get('theme')
      if (theme === 'light' || theme === 'dark') document.documentElement.dataset.theme = theme
      if (options.mock) await useMockBackend()
      if (options.capture) {
        await runCaptureRequest(options.capture)
        return
      }
      void github.load()
      await app.init()
      const scene = params.get('scene')
      if (scene) await runUrlScene(scene)

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
    })()

    return () => unlistenDrop?.()
  })

  async function saveScreenshot() {
    try {
      const path = await api.captureScreen()
      app.toast(path)
    } catch (e) {
      app.error = describeError(e)
    }
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'F5') {
      e.preventDefault()
      void app.refresh()
    } else if (inTauri && e.ctrlKey && e.shiftKey && e.code === 'KeyS') {
      e.preventDefault()
      void saveScreenshot()
    }
  }

  // Pick up changes made outside Leveret when the window is focused again.
  function onfocus() {
    const now = Date.now()
    if (app.selectedPath && now - lastFocusRefresh > 1000) {
      lastFocusRefresh = now
      void app.refresh()
    }
  }
</script>

<svelte:window {onkeydown} {onfocus} />

<div class="layout">
  <RepoSidebar />

  <main>
    {#if app.loaded && !app.repo}
      <Welcome />
    {:else if app.repo}
      <RepoHeader />
      <div class="body">
        <section class="column">
          <div class="tabs" role="tablist">
            <button role="tab" class:active={app.tab === 'changes'} aria-selected={app.tab === 'changes'} onclick={() => app.setTab('changes')}>
              {t('tabs.changes')}
              {#if app.files.length}<span class="count">{app.files.length}</span>{/if}
            </button>
            <button role="tab" class:active={app.tab === 'history'} aria-selected={app.tab === 'history'} onclick={() => app.setTab('history')}>
              {t('tabs.history')}
            </button>
          </div>
          <div class="column-body">
            {#if app.tab === 'changes'}
              <ChangesPane />
            {:else}
              <HistoryPane />
            {/if}
          </div>
        </section>

        <section class="main">
          {#if app.tab === 'changes'}
            {#if app.selectedChange}
              <div class="file-header">
                <FileRow path={app.selectedChange.path} origPath={app.selectedChange.origPath} kind={app.selectedChange.kind} />
              </div>
              <div class="main-body"><DiffView diff={app.workingDiff} /></div>
            {:else if app.status}
              <div class="empty">
                <Icon name="file" size={32} />
                {app.files.length ? t('changes.selectFile') : t('changes.noneBody')}
              </div>
            {/if}
          {:else}
            <CommitView />
          {/if}
        </section>
      </div>
    {/if}
  </main>
</div>

{#if dragging}
  <div class="drop"><div>{t('repos.dropHint')}</div></div>
{/if}

{#if github.cloneOpen}
  <CloneDialog />
{/if}
{#if github.signInOpen}
  <SignInDialog />
{/if}
<ErrorDialog />
<Toasts />

<style>
  .layout {
    display: grid;
    grid-template-columns: 220px 1fr;
    height: 100%;
  }

  main {
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
    width: 340px;
    flex: none;
    border-right: 1px solid var(--border);
  }

  .tabs {
    display: flex;
    border-bottom: 1px solid var(--border);
  }

  .tabs button {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    height: 36px;
    border: none;
    border-bottom: 2px solid transparent;
    background: transparent;
    color: var(--fg-muted);
  }

  .tabs button:hover {
    color: var(--fg);
  }

  .tabs button.active {
    border-bottom-color: var(--attention);
    color: var(--fg);
    font-weight: 600;
  }

  .count {
    min-width: 18px;
    padding: 0 5px;
    border-radius: 999px;
    background: var(--bg-muted);
    font-size: 11px;
    line-height: 18px;
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

  .file-header {
    flex: none;
    height: 34px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-subtle);
  }

  .main-body {
    flex: 1;
    min-height: 0;
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
