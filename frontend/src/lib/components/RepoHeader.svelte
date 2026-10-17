<script lang="ts">
  import { t } from '../i18n/index.svelte'
  import { app } from '../state.svelte'
  import Icon from './Icon.svelte'

  const status = $derived(app.status)
  const branchLabel = $derived(status ? (status.branch ?? t('repo.detached')) : '')
</script>

<header class="repo-header">
  <div class="cell repo">
    <Icon name="repo" />
    <div class="text">
      <span class="label">{t('repos.title')}</span>
      <span class="value">{app.repo?.name ?? ''}</span>
    </div>
  </div>

  <div class="cell branch">
    <Icon name="branch" />
    <div class="text">
      <span class="label">{t('repo.branch')}</span>
      <span class="value">{branchLabel}</span>
    </div>
    {#if status?.merging}
      <span class="pill warn">{t('repo.merging')}</span>
    {/if}
  </div>

  <div class="cell sync">
    <Icon name="cloud" />
    <div class="text">
      <span class="label">{status?.upstream ?? t('repo.noUpstream')}</span>
      <span class="value counts">
        {#if status && !status.headOid}
          <span class="muted">{t('repo.noCommits')}</span>
        {:else if status?.upstream}
          <span title={t('repo.ahead', { count: status.ahead })}><Icon name="arrowUp" size={13} />{status.ahead}</span>
          <span title={t('repo.behind', { count: status.behind })}><Icon name="arrowDown" size={13} />{status.behind}</span>
        {/if}
      </span>
    </div>
  </div>

  <div class="spacer"></div>

  <button class="icon-btn refresh" title={`${t('common.refresh')} (F5)`} aria-label={t('common.refresh')} onclick={() => app.refresh()}>
    <Icon name="refresh" />
  </button>
</header>

<style>
  .repo-header {
    display: flex;
    align-items: stretch;
    height: 48px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-subtle);
  }

  .cell {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
    padding: 0 16px;
    border-right: 1px solid var(--border-muted);
    color: var(--fg-muted);
  }

  .repo {
    width: 340px;
    flex: none;
  }

  .branch {
    min-width: 200px;
  }

  .text {
    display: flex;
    flex-direction: column;
    min-width: 0;
    line-height: 1.25;
  }

  .label {
    font-size: 11px;
    color: var(--fg-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .value {
    font-weight: 600;
    color: var(--fg);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .counts {
    display: flex;
    gap: 10px;
    font-weight: 500;
  }

  .counts span {
    display: inline-flex;
    align-items: center;
    gap: 2px;
  }

  .muted {
    color: var(--fg-muted);
    font-weight: 400;
  }

  .pill {
    padding: 1px 8px;
    border-radius: 999px;
    font-size: 11px;
    font-weight: 600;
  }

  .pill.warn {
    background: color-mix(in srgb, var(--attention) 18%, transparent);
    color: var(--attention);
  }

  .spacer {
    flex: 1;
  }

  .refresh {
    align-self: center;
    margin-right: 10px;
  }
</style>
