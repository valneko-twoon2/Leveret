<!-- Left column of the History tab: the commit list with its graph, search, a file's history and a context menu. -->
<script lang="ts">
  import type { LogEntry, LogScope, Tag } from '../api'
  import { branches } from '../branches.svelte'
  import { changes } from '../changes.svelte'
  import { formatDateTime, formatRelative, shortHash } from '../format'
  import { history, type SearchField } from '../history.svelte'
  import { t } from '../i18n/index.svelte'
  import { menu } from '../menu.svelte'
  import { app } from '../state.svelte'
  import { tags } from '../tags.svelte'
  import GraphCell from './GraphCell.svelte'
  import Icon from './Icon.svelte'
  import RefBadge from './RefBadge.svelte'
  import VirtualList from './VirtualList.svelte'

  const ROW = 52
  const MAX_LANES = 8
  const FIELDS: SearchField[] = ['message', 'author', 'content', 'path']
  const lanes = $derived(Math.min(MAX_LANES, Math.max(1, ...history.log.map((c) => c.graph.width))))
  const list = $derived(history.log)
  // Search results skip commits, so the graph only makes sense for the whole log.
  const showGraph = $derived(history.query === null)
  const selectedIndex = $derived(list.findIndex((c) => c.hash === history.selectedCommit))

  function onSearchKey(e: KeyboardEvent) {
    if (e.key === 'Escape' && history.logFilter) {
      e.preventDefault()
      void history.setFilter('', true)
    } else if (e.key === 'Enter') {
      e.preventDefault()
      void history.setFilter(history.logFilter, true)
    }
  }

  /** A tag shown on a commit, for the tag actions (which only need its name and commit). */
  function tagOf(c: LogEntry, name: string): Tag {
    return { name, target: c.hash, annotated: false, subject: c.subject, time: c.authorTime }
  }

  function commitMenu(c: LogEntry, e: MouseEvent | KeyboardEvent) {
    const idle = app.busy === null
    const isHead = c.hash === app.status?.headOid
    const busyOp = !idle || app.operation !== null
    menu.open(e, [
      { label: t('history.createBranch'), hint: isHead ? t('shortcuts.newBranch') : undefined, action: () => branches.openNewBranch(c.hash, shortHash(c.hash)), disabled: !idle },
      { label: t('history.checkout'), action: () => history.checkoutCommit(c), disabled: busyOp || isHead },
      { label: t('tags.createHere'), action: () => tags.openNewTag(c.hash, c.subject), disabled: !idle },
      ...c.refs
        .filter((r) => r.kind === 'tag')
        .flatMap((r) => [
          { label: t('tags.pushOne', { tag: r.name }), action: () => tags.push(tagOf(c, r.name)), disabled: !idle || !app.status?.hasRemote },
          { label: t('tags.deleteOne', { tag: r.name }), action: () => tags.remove(tagOf(c, r.name), false), disabled: !idle, danger: true },
        ]),
      'separator',
      { label: t('history.revert'), action: () => history.revertCommit(c), disabled: busyOp },
      { label: t('history.cherryPick'), action: () => history.cherryPick(c), disabled: busyOp || isHead },
      ...(isHead ? [{ label: t('commit.undo'), action: () => changes.undoLastCommit(), disabled: !idle || !app.canRewriteHead }] : []),
      'separator',
      { label: t('history.copySha'), action: () => app.copy(c.hash) },
      { label: t('history.copySubject'), action: () => app.copy(c.subject) },
    ])
  }
</script>

<div class="pane">
  <div class="toolbar">
    <select value={history.scope} onchange={(e) => history.setScope(e.currentTarget.value as LogScope)} aria-label={t('tabs.history')}>
      <option value="head">{t('history.scopeHead')}</option>
      <option value="all">{t('history.scopeAll')}</option>
    </select>
    <select
      class="field"
      value={history.searchField}
      onchange={(e) => history.setSearchField(e.currentTarget.value as SearchField)}
      title={t(`history.fieldHint.${history.searchField}`)}
      aria-label={t('history.searchIn')}
    >
      {#each FIELDS as f (f)}<option value={f}>{t(`history.field.${f}`)}</option>{/each}
    </select>
    <div class="search">
      <Icon name="search" size={13} />
      <input
        id="history-search"
        type="text"
        placeholder={t('history.search')}
        title={`${t(`history.fieldHint.${history.searchField}`)} (${t('shortcuts.search')})`}
        value={history.logFilter}
        oninput={(e) => history.setFilter(e.currentTarget.value)}
        onkeydown={onSearchKey}
        aria-label={t('history.search')}
        spellcheck="false"
      />
      {#if history.logFilter.trim()}
        <span class="matches">
          {history.logLoading
            ? t('history.searching')
            : t(history.logHasMore ? 'history.matchesMore' : 'history.matches', { count: list.length })}
        </span>
      {/if}
    </div>
  </div>
  {#if history.file}
    <div class="file-history">
      <Icon name="file" size={13} />
      <span class="label">{t('history.fileHistory')}:</span>
      <span class="path" title={history.file}><bdi>{history.file}</bdi></span>
      <button class="icon-btn" title={t('history.fileHistoryClose')} aria-label={t('history.fileHistoryClose')} onclick={() => history.closeFileHistory()}>
        <Icon name="x" size={14} />
      </button>
    </div>
  {/if}

  <div class="list">
    {#if history.logLoading && history.log.length === 0}
      <div class="empty">{t('common.loading')}</div>
    {:else if history.log.length === 0}
      <div class="empty">{t('history.empty')}</div>
    {:else if list.length === 0}
      <div class="empty">{t('history.noMatches')}</div>
    {:else}
      <VirtualList
        items={list}
        rowHeight={ROW}
        {selectedIndex}
        label={t('tabs.history')}
        onselect={(i) => list[i] && history.selectCommit(list[i].hash)}
        onreachend={() => history.loadMoreLog()}
        oncontext={(i, e) => list[i] && commitMenu(list[i], e)}
      >
        {#snippet row(c)}
          <div class="commit" class:plain={!showGraph}>
            {#if showGraph}<GraphCell graph={c.graph} {lanes} height={ROW} merge={c.parents.length > 1} />{/if}
            <div class="text">
              <div class="subject">
                {#each c.refs as r (r.kind + r.name)}<RefBadge label={r} />{/each}
                <span class="summary" title={c.subject}>{c.subject}</span>
              </div>
              <div class="meta" title={formatDateTime(c.authorTime)}>
                {c.authorName} · {formatRelative(c.authorTime)}{#if !showGraph} · <span class="mono">{shortHash(c.hash)}</span>{/if}
              </div>
            </div>
          </div>
        {/snippet}
        {#snippet footer()}
          {#if history.logLoading && history.log.length > 0}
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
    gap: 6px;
    height: 34px;
    padding: 0 8px;
    border-bottom: 1px solid var(--border-muted);
  }

  select {
    flex: none;
    max-width: 112px;
    height: 24px;
    padding: 0 4px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg);
  }

  .file-history {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 30px;
    padding: 0 6px 0 10px;
    border-bottom: 1px solid var(--border-muted);
    background: color-mix(in srgb, var(--accent) 8%, transparent);
    color: var(--fg-muted);
    font-size: 12px;
  }

  .file-history .label {
    flex: none;
  }

  .file-history .path {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    color: var(--fg);
    text-overflow: ellipsis;
    white-space: nowrap;
    direction: rtl;
    text-align: left;
  }

  .search {
    flex: 1;
    position: relative;
    display: flex;
    align-items: center;
    min-width: 0;
    color: var(--fg-muted);
  }

  .search :global(svg) {
    position: absolute;
    left: 7px;
  }

  .search input {
    width: 100%;
    height: 24px;
    padding: 0 6px 0 24px;
    font-size: 12px;
  }

  .matches {
    position: absolute;
    right: 8px;
    font-size: 11px;
    color: var(--fg-subtle);
    pointer-events: none;
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

  .commit.plain {
    padding-left: 12px;
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
