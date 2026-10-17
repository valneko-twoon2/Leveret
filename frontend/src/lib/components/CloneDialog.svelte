<script lang="ts">
  import { pickFolder } from '../dialogs'
  import { formatRelative } from '../format'
  import { github } from '../github.svelte'
  import { t } from '../i18n/index.svelte'
  import Icon from './Icon.svelte'
  import Modal from './Modal.svelte'
  import VirtualList from './VirtualList.svelte'

  const target = $derived(
    github.cloneSource && github.parentDir.trim()
      ? `${github.parentDir.trim().replace(/[\\/]+$/, '')}\\${github.cloneSource.name}`
      : '',
  )
  const selectedIndex = $derived(github.filteredRepos.findIndex((r) => r.fullName === github.selectedRepo))

  async function browse() {
    const dir = await pickFolder(t('clone.browseTitle'))
    if (dir) github.parentDir = dir
  }
</script>

<Modal title={t('clone.title')} width={620} onclose={github.cloning ? undefined : () => github.closeClone()}>
  <div class="tabs" role="tablist">
    <button role="tab" class:active={github.cloneTab === 'github'} aria-selected={github.cloneTab === 'github'} onclick={() => (github.cloneTab = 'github')}>
      {t('clone.githubTab')}
    </button>
    <button role="tab" class:active={github.cloneTab === 'url'} aria-selected={github.cloneTab === 'url'} onclick={() => (github.cloneTab = 'url')}>
      {t('clone.urlTab')}
    </button>
  </div>

  {#if github.cloneTab === 'github'}
    {#if !github.account}
      <div class="empty signin">
        <span>{t('clone.signInFirst')}</span>
        <button class="btn primary" onclick={() => github.openSignIn()}>
          <Icon name="user" size={14} />
          {t('github.signIn')}
        </button>
      </div>
    {:else}
      <div class="search">
        <Icon name="search" size={14} />
        <input type="text" placeholder={t('clone.filter')} aria-label={t('clone.filter')} bind:value={github.filter} />
      </div>
      <div class="repo-list">
        {#if github.reposLoading}
          <div class="empty">{t('clone.loading')}</div>
        {:else if github.filteredRepos.length === 0}
          <div class="empty">{t('clone.noRepos')}</div>
        {:else}
          <VirtualList
            items={github.filteredRepos}
            rowHeight={44}
            {selectedIndex}
            label={t('clone.githubTab')}
            onselect={(i) => (github.selectedRepo = github.filteredRepos[i].fullName)}
          >
            {#snippet row(repo)}
              <div class="repo">
                <div class="names">
                  <span class="name" title={repo.fullName}><span class="owner">{repo.owner}/</span>{repo.name}</span>
                  {#if repo.private}<span class="private" title={t('clone.private')}><Icon name="lock" size={11} /></span>{/if}
                </div>
                <div class="meta" title={repo.description ?? undefined}>
                  {repo.description ?? ''}
                  {#if repo.pushedAt}<span class="when">· {formatRelative(Date.parse(repo.pushedAt) / 1000)}</span>{/if}
                </div>
              </div>
            {/snippet}
          </VirtualList>
        {/if}
      </div>
    {/if}
  {:else}
    <label class="field">
      <span>{t('clone.url')}</span>
      <input type="text" spellcheck="false" placeholder={t('clone.urlPlaceholder')} bind:value={github.url} />
    </label>
  {/if}

  <label class="field">
    <span>{t('clone.localPath')}</span>
    <div class="path-row">
      <input type="text" spellcheck="false" bind:value={github.parentDir} />
      <button class="btn" onclick={browse}><Icon name="folder" size={14} />{t('clone.browse')}</button>
    </div>
    {#if target}<small class="mono" title={target}>{target}</small>{/if}
  </label>

  {#if github.progress}
    <div class="progress" aria-live="polite">
      <div class="bar"><div class="fill" style:width="{github.progress.percent}%"></div></div>
      <small>{github.progress.phase || t('clone.cloning')} {github.progress.percent}%</small>
    </div>
  {/if}

  {#snippet footer()}
    <button class="btn" disabled={github.cloning} onclick={() => github.closeClone()}>{t('common.cancel')}</button>
    <button class="btn primary" disabled={!github.cloneSource || !github.parentDir.trim() || github.cloning} onclick={() => github.clone()}>
      <Icon name="download" size={14} />
      {github.cloning ? t('clone.cloning') : t('clone.submit')}
    </button>
  {/snippet}
</Modal>

<style>
  .tabs {
    display: flex;
    gap: 16px;
    margin: -4px 0 14px;
    border-bottom: 1px solid var(--border-muted);
  }

  .tabs button {
    height: 32px;
    padding: 0 2px;
    border: none;
    border-bottom: 2px solid transparent;
    background: none;
    color: var(--fg-muted);
  }

  .tabs button.active {
    border-bottom-color: var(--attention);
    color: var(--fg);
    font-weight: 600;
  }

  .search {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 8px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--fg-subtle);
  }

  .search:focus-within {
    border-color: var(--accent);
  }

  .search input {
    flex: 1;
    border: none;
    box-shadow: none !important;
    padding-left: 0;
  }

  .repo-list {
    height: 260px;
    margin-top: 8px;
    border: 1px solid var(--border-muted);
    border-radius: var(--radius);
    overflow: hidden;
  }

  .repo {
    width: 100%;
    min-width: 0;
    padding: 0 12px;
  }

  .names {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .name {
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .owner {
    font-weight: 400;
    color: var(--fg-muted);
  }

  .private {
    display: inline-flex;
    color: var(--warning);
  }

  .meta {
    font-size: 12px;
    color: var(--fg-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .when {
    color: var(--fg-subtle);
  }

  .signin {
    height: 200px;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-top: 14px;
  }

  .field > span {
    font-weight: 600;
  }

  .field small {
    color: var(--fg-subtle);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .path-row {
    display: flex;
    gap: 6px;
  }

  .path-row input {
    flex: 1;
    min-width: 0;
  }

  .progress {
    margin-top: 14px;
  }

  .bar {
    height: 6px;
    border-radius: 3px;
    background: var(--bg-muted);
    overflow: hidden;
  }

  .fill {
    height: 100%;
    background: var(--accent);
    transition: width 0.2s;
  }

  .progress small {
    color: var(--fg-muted);
  }
</style>
