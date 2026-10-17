<!-- Left column of the History tab: the commit list with its graph. -->
<script lang="ts">
  import type { LogScope } from '../api'
  import { formatDateTime, formatRelative } from '../format'
  import { t } from '../i18n/index.svelte'
  import { app } from '../state.svelte'
  import GraphCell from './GraphCell.svelte'
  import RefBadge from './RefBadge.svelte'
  import VirtualList from './VirtualList.svelte'

  const ROW = 52
  const MAX_LANES = 8
  const lanes = $derived(Math.min(MAX_LANES, Math.max(1, ...app.log.map((c) => c.graph.width))))
  const selectedIndex = $derived(app.log.findIndex((c) => c.hash === app.selectedCommit))
</script>

<div class="pane">
  <div class="toolbar">
    <select
      value={app.scope}
      onchange={(e) => app.setScope(e.currentTarget.value as LogScope)}
      aria-label={t('tabs.history')}
    >
      <option value="head">{t('history.scopeHead')}</option>
      <option value="all">{t('history.scopeAll')}</option>
    </select>
  </div>

  <div class="list">
    {#if !app.logLoading && app.log.length === 0}
      <div class="empty">{t('history.empty')}</div>
    {:else}
      <VirtualList
        items={app.log}
        rowHeight={ROW}
        {selectedIndex}
        label={t('tabs.history')}
        onselect={(i) => app.selectCommit(app.log[i].hash)}
        onreachend={() => app.loadMoreLog()}
      >
        {#snippet row(c)}
          <div class="commit">
            <GraphCell graph={c.graph} {lanes} height={ROW} merge={c.parents.length > 1} />
            <div class="text">
              <div class="subject">
                {#each c.refs as r (r.kind + r.name)}<RefBadge label={r} />{/each}
                <span class="summary">{c.subject}</span>
              </div>
              <div class="meta" title={formatDateTime(c.authorTime)}>
                {c.authorName} · {formatRelative(c.authorTime)}
              </div>
            </div>
          </div>
        {/snippet}
        {#snippet footer()}
          {#if app.logLoading && app.log.length > 0}
            <div class="more">{t('history.loadingMore')}</div>
          {/if}
        {/snippet}
      </VirtualList>
    {/if}
  </div>
</div>

<style>
  .pane {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
  }

  .toolbar {
    display: flex;
    align-items: center;
    height: 34px;
    padding: 0 8px;
    border-bottom: 1px solid var(--border-muted);
  }

  select {
    height: 24px;
    padding: 0 4px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg);
  }

  .list {
    flex: 1;
    min-height: 0;
  }

  .commit {
    display: flex;
    align-items: center;
    width: 100%;
    height: 100%;
    padding-right: 12px;
  }

  .text {
    flex: 1;
    min-width: 0;
    padding-left: 4px;
  }

  .subject {
    display: flex;
    align-items: center;
    gap: 4px;
    min-width: 0;
  }

  .summary {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 500;
  }

  .meta {
    font-size: 12px;
    color: var(--fg-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .more {
    padding: 8px;
    text-align: center;
    color: var(--fg-muted);
  }
</style>
