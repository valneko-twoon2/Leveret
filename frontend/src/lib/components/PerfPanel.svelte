<!-- Performance panel (Ctrl+Shift+D): timings of screen transitions, backend round trips,
     git processes and GitHub requests, slowest first. Not modal, so the app can be used
     while it is open. -->
<script lang="ts">
  import { onMount } from 'svelte'
  import { syncBackendPerf } from '../api'
  import { t, type MessageKey } from '../i18n/index.svelte'
  import { perf, summarize, type PerfKind, type PerfStat } from '../perf'
  import { app } from '../state.svelte'
  import Icon from './Icon.svelte'

  type Filter = 'all' | PerfKind

  const filters: { value: Filter; label: MessageKey }[] = [
    { value: 'all', label: 'perf.all' },
    { value: 'view', label: 'perf.kind.view' },
    { value: 'ipc', label: 'perf.kind.ipc' },
    { value: 'git', label: 'perf.kind.git' },
    { value: 'http', label: 'perf.kind.http' },
    { value: 'action', label: 'perf.kind.action' },
    { value: 'startup', label: 'perf.kind.startup' },
    { value: 'longtask', label: 'perf.kind.longtask' },
    { value: 'tool', label: 'perf.kind.tool' },
  ]

  let filter = $state<Filter>('all')
  let stats = $state.raw<PerfStat[]>([])
  const shown = $derived(filter === 'all' ? stats : stats.filter((s) => s.kind === filter))

  async function update() {
    // Without a backend (browser preview) only the frontend's own samples are shown.
    await syncBackendPerf().catch(() => {})
    stats = summarize(perf.all())
  }

  onMount(() => {
    void update()
    const timer = setInterval(() => void update(), 1000)
    return () => clearInterval(timer)
  })

  function clear() {
    perf.clear()
    stats = []
  }

  function copy() {
    void app.copy(JSON.stringify({ summary: stats, samples: perf.all() }, null, 2))
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.stopPropagation()
      app.perfOpen = false
    }
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<aside class="panel" aria-label={t('perf.title')} {onkeydown}>
  <header>
    <strong>{t('perf.title')}</strong>
    <select bind:value={filter} aria-label={t('perf.filter')}>
      {#each filters as f (f.value)}
        <option value={f.value}>{t(f.label)}</option>
      {/each}
    </select>
    <span class="spacer"></span>
    <button class="icon-btn" title={t('perf.copy')} aria-label={t('perf.copy')} onclick={copy}><Icon name="copy" /></button>
    <button class="icon-btn" title={t('perf.clear')} aria-label={t('perf.clear')} onclick={clear}><Icon name="refresh" /></button>
    <button class="icon-btn" title={t('perf.close')} aria-label={t('perf.close')} onclick={() => (app.perfOpen = false)}><Icon name="x" /></button>
  </header>
  <div class="table">
    {#if shown.length === 0}
      <p class="empty">{t('perf.empty')}</p>
    {:else}
      <table>
        <thead>
          <tr>
            <th>{t('perf.kindColumn')}</th>
            <th class="name">{t('perf.name')}</th>
            <th>{t('perf.count')}</th>
            <th>{t('perf.median')}</th>
            <th>p95</th>
            <th>{t('perf.max')}</th>
          </tr>
        </thead>
        <tbody>
          {#each shown as s (s.kind + s.name)}
            <tr class:over={s.overBudget} title={s.budget !== null ? t('perf.budget', { ms: s.budget }) : undefined}>
              <td><span class="kind {s.kind}">{t(`perf.kind.${s.kind}` as MessageKey)}</span></td>
              <td class="name">{s.name}{#if s.failed}<span class="failed">{t('perf.failed', { count: s.failed })}</span>{/if}</td>
              <td>{s.count}</td>
              <td>{s.median}</td>
              <td class="p95">{s.p95}</td>
              <td>{s.max}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </div>
  <footer>{t('perf.hint')}</footer>
</aside>

<style>
  .panel {
    position: fixed;
    right: 16px;
    bottom: 16px;
    z-index: 70;
    width: min(620px, calc(100vw - 32px));
    max-height: min(480px, calc(100vh - 32px));
    display: flex;
    flex-direction: column;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg);
    box-shadow: var(--shadow);
    font-size: 12px;
  }

  header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 10px;
    border-bottom: 1px solid var(--border);
  }

  header select {
    font-size: 12px;
  }

  .spacer {
    flex: 1;
  }

  .icon-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 26px;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--fg-muted);
  }

  .icon-btn:hover {
    background: var(--bg-hover);
    color: var(--fg);
  }

  .table {
    flex: 1;
    min-height: 0;
    overflow: auto;
  }

  table {
    width: 100%;
    border-collapse: collapse;
    font-variant-numeric: tabular-nums;
  }

  th,
  td {
    padding: 4px 10px;
    text-align: right;
    white-space: nowrap;
  }

  th {
    position: sticky;
    top: 0;
    background: var(--bg-subtle);
    color: var(--fg-muted);
    font-weight: 600;
  }

  th:first-child,
  td:first-child,
  .name {
    text-align: left;
  }

  td.name {
    max-width: 260px;
    overflow: hidden;
    text-overflow: ellipsis;
    font-family: var(--font-mono);
  }

  tbody tr:nth-child(even) {
    background: var(--bg-subtle);
  }

  tr.over .p95 {
    color: var(--danger);
    font-weight: 600;
  }

  .kind {
    padding: 1px 6px;
    border-radius: 999px;
    background: var(--bg-muted);
    color: var(--fg-muted);
  }

  .kind.view {
    background: color-mix(in srgb, var(--accent) 16%, transparent);
    color: var(--accent);
  }

  .kind.git,
  .kind.http {
    background: color-mix(in srgb, var(--success) 16%, transparent);
    color: var(--success);
  }

  .kind.longtask {
    background: color-mix(in srgb, var(--warning) 20%, transparent);
    color: var(--warning);
  }

  .failed {
    margin-left: 6px;
    color: var(--danger);
    font-family: var(--font-ui);
  }

  .empty {
    margin: 0;
    padding: 24px;
    color: var(--fg-muted);
    text-align: center;
  }

  footer {
    padding: 6px 10px;
    border-top: 1px solid var(--border);
    color: var(--fg-subtle);
  }
</style>
