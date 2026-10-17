<!-- Left column of the Issues tab: local and GitHub issues with filters and search. -->
<script lang="ts">
  import { formatDateTime, formatRelative } from '../format'
  import { github } from '../github.svelte'
  import { t } from '../i18n/index.svelte'
  import type { Issue, IssueStateFilter } from '../issueApi'
  import { issueKey, issues, type IssueFilter } from '../issues.svelte'
  import Icon from './Icon.svelte'
  import IssueLabels from './IssueLabels.svelte'
  import VirtualList from './VirtualList.svelte'

  const ROW = 56
  const list = $derived(issues.list)
  const selectedIndex = $derived(list.findIndex((i) => issueKey(i) === issues.selected))
  const sources = $derived(issues.sources)

  function clearOnEscape(e: KeyboardEvent) {
    if (e.key === 'Escape' && issues.query) {
      e.preventDefault()
      issues.query = ''
    }
  }

  function idLabel(i: Issue): string {
    return i.source === 'local' ? t('issues.localId', { id: i.id }) : `#${i.id}`
  }
</script>

<div class="pane">
  <div class="toolbar">
    <select bind:value={issues.filter} aria-label={t('issues.source')}>
      <option value={'all' satisfies IssueFilter}>{t('issues.sourceAll')}</option>
      <option value={'local' satisfies IssueFilter}>{t('issues.sourceLocal')}</option>
      <option value={'github' satisfies IssueFilter} disabled={!sources?.githubRepo}>{t('issues.sourceGithub')}</option>
    </select>
    <select
      value={issues.stateFilter}
      onchange={(e) => issues.setStateFilter(e.currentTarget.value as IssueStateFilter)}
      aria-label={t('issues.state')}
    >
      <option value="open">{t('issues.stateOpen')}</option>
      <option value="closed">{t('issues.stateClosed')}</option>
      <option value="all">{t('issues.stateAll')}</option>
    </select>
    <span class="spacer"></span>
    <button class="icon-btn" title={t('common.refresh')} aria-label={t('common.refresh')} disabled={issues.loading} onclick={() => issues.load()}>
      <Icon name="refresh" size={15} />
    </button>
    <button class="btn primary small" onclick={() => issues.openNew()}><Icon name="plus" size={14} /> {t('issues.new')}</button>
  </div>
  <div class="toolbar">
    <div class="search">
      <Icon name="search" size={13} />
      <input
        type="text"
        placeholder={t('issues.search')}
        bind:value={issues.query}
        onkeydown={clearOnEscape}
        aria-label={t('issues.search')}
        spellcheck="false"
      />
    </div>
  </div>

  {#if sources && !sources.githubRepo}
    <p class="notice">{t('issues.noGithubRemote')}</p>
  {:else if sources && !sources.signedIn}
    <p class="notice">
      {t('issues.signInHint')}
      <button class="link" onclick={() => github.openSignIn()}>{t('github.signIn')}</button>
    </p>
  {:else if issues.githubError}
    <p class="notice error">{issues.githubError}</p>
  {/if}

  <div class="list">
    {#if issues.loading && !issues.loaded}
      <div class="empty">{t('common.loading')}</div>
    {:else if list.length === 0}
      <div class="empty">{issues.query ? t('issues.noMatches') : t('issues.empty')}</div>
    {:else}
      <VirtualList items={list} rowHeight={ROW} {selectedIndex} label={t('tabs.issues')} onselect={(i) => list[i] && issues.select(list[i])}>
        {#snippet row(issue)}
          <div class="issue">
            <span class="state" class:closed={issue.state === 'closed'}>
              <Icon name={issue.state === 'closed' ? 'issueClosed' : 'issue'} size={16} />
            </span>
            <div class="text">
              <div class="title">
                <span class="summary" title={issue.title}>{issue.title}</span>
                {#if issue.labels.length}<span class="labels"><IssueLabels labels={issue.labels} max={2} /></span>{/if}
              </div>
              <div class="meta" title={formatDateTime(issue.updatedAt)}>
                {#if issue.source === 'local'}<span class="local"><Icon name="laptop" size={12} /> {idLabel(issue)}</span>{:else}<span>{idLabel(issue)}</span>{/if}
                {#if issue.author}<span>{issue.author}</span>{/if}
                {#if issue.comments}<span><Icon name="comment" size={12} /> {issue.comments}</span>{/if}
                <span>{formatRelative(issue.updatedAt)}</span>
              </div>
            </div>
          </div>
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
    height: 24px;
    padding: 0 4px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg);
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

  .icon-btn:hover:not(:disabled) {
    background: var(--bg-hover);
    color: var(--fg);
  }

  .small {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: 24px;
    padding: 0 8px;
    font-size: 12px;
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

  .notice {
    margin: 0;
    padding: 6px 10px;
    border-bottom: 1px solid var(--border-muted);
    background: var(--bg-subtle);
    color: var(--fg-muted);
    font-size: 12px;
  }

  .notice.error {
    color: var(--danger);
  }

  .link {
    padding: 0;
    border: none;
    background: none;
    color: var(--accent);
    font-size: 12px;
    text-decoration: underline;
  }

  .list {
    flex: 1;
    min-height: 0;
  }

  .issue {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    width: 100%;
    height: 100%;
    padding: 9px 12px 0;
  }

  .state {
    flex: none;
    color: var(--success);
  }

  .state.closed {
    color: var(--fg-subtle);
  }

  .text {
    flex: 1;
    min-width: 0;
  }

  .title {
    display: flex;
    align-items: center;
    gap: 4px;
    min-width: 0;
  }

  /* The title keeps its full width (up to the row); labels get what is left. */
  .summary {
    flex: none;
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 500;
  }

  .labels {
    flex: 1 1 0;
    display: flex;
    gap: 4px;
    min-width: 0;
    overflow: hidden;
  }

  .meta {
    display: flex;
    gap: 8px;
    font-size: 12px;
    color: var(--fg-muted);
    overflow: hidden;
    white-space: nowrap;
  }

  .meta span {
    display: inline-flex;
    align-items: center;
    gap: 3px;
  }

  .local {
    color: var(--attention);
  }
</style>
