<script lang="ts">
  import { agents } from '../agents.svelte'
  import { confirmDialog, pickFolder } from '../dialogs'
  import { settings } from '../settings.svelte'
  import { t } from '../i18n/index.svelte'
  import { menu } from '../menu.svelte'
  import { app } from '../state.svelte'
  import type { RepoEntry } from '../api'
  import AccountBox from './AccountBox.svelte'
  import Icon from './Icon.svelte'
  import Logo from './Logo.svelte'
  import RefTree from './RefTree.svelte'

  async function add() {
    const path = await pickFolder(t('repos.addDialogTitle'))
    if (path) await app.addRepo(path)
  }

  async function remove(repo: RepoEntry, e?: MouseEvent) {
    e?.stopPropagation()
    if (await confirmDialog(t('repos.removeConfirm', { name: repo.name }), t('repos.remove'))) {
      await app.removeRepo(repo.path)
    }
  }

  async function repoMenu(repo: RepoEntry, e: MouseEvent) {
    menu.open(e, [
      { label: t('common.openInExplorer'), action: () => openInExplorer(repo) },
      { label: t('common.copyPath'), action: () => app.copy(repo.path) },
      'separator',
      { label: t('repos.remove'), action: () => remove(repo), danger: true },
    ])
  }

  async function openInExplorer(repo: RepoEntry) {
    if (repo.path !== app.selectedPath) await app.selectRepo(repo.path)
    await app.showInExplorer()
  }
</script>

<aside class="sidebar">
  <header>
    <Logo size={22} />
    <span class="title">{t('app.name')}</span>
    <button class="icon-btn" title={t('agent.open')} aria-label={t('agent.open')} onclick={() => agents.openAgent()}>
      <Icon name="agent" />
    </button>
    <button class="icon-btn" title={t('repos.add')} aria-label={t('repos.add')} onclick={add}>
      <Icon name="plus" />
    </button>
    <button class="icon-btn" title={`${t('settings.open')} (${t('shortcuts.settings')})`} aria-label={t('settings.open')} onclick={() => settings.openSettings()}>
      <Icon name="settings" />
    </button>
  </header>

  <ul class:compact={!!app.repo}>
    {#each app.repos as repo (repo.path)}
      <li class:selected={repo.path === app.selectedPath}>
        <button class="repo" title={repo.path} onclick={() => app.selectRepo(repo.path)} oncontextmenu={(e) => repoMenu(repo, e)}>
          <Icon name="repo" size={15} />
          <span class="name">{repo.name}</span>
        </button>
        <button class="remove" title={t('repos.remove')} aria-label={t('repos.remove')} onclick={(e) => remove(repo, e)}>
          <Icon name="x" size={13} />
        </button>
      </li>
    {:else}
      {#if app.loaded}
        <li class="none">{t('repos.empty')}</li>
      {/if}
    {/each}
  </ul>

  {#if app.repo}
    <div class="refs"><RefTree /></div>
  {/if}

  <AccountBox />
</aside>

<style>
  .sidebar {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    background: var(--bg-sidebar);
  }

  header {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 48px;
    padding: 0 8px 0 14px;
    border-bottom: 1px solid var(--border-muted);
  }

  .title {
    flex: 1;
    font-weight: 600;
  }

  ul {
    flex: 1;
    margin: 0;
    padding: 6px;
    list-style: none;
    overflow-y: auto;
  }

  ul.compact {
    flex: 0 1 auto;
    max-height: 34%;
    min-height: 48px;
  }

  .refs {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    border-top: 1px solid var(--border-muted);
  }

  .repo {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    height: 30px;
    padding: 0 30px 0 10px;
    border: none;
    border-radius: var(--radius);
    background: transparent;
    color: var(--fg-muted);
    text-align: left;
  }

  li {
    position: relative;
  }

  .repo:hover {
    background: var(--bg-hover);
    color: var(--fg);
  }

  li.selected .repo {
    background: var(--bg-selected);
    color: var(--fg);
    font-weight: 600;
  }

  .name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .remove {
    position: absolute;
    top: 5px;
    right: 6px;
    display: none;
    padding: 0;
    border: none;
    background: transparent;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    border-radius: 4px;
    color: var(--fg-subtle);
  }

  /* Also on keyboard focus, so the button can be reached with Tab (not after a mouse click). */
  li:hover .remove,
  li:has(:focus-visible) .remove {
    display: inline-flex;
  }

  .remove:hover {
    background: var(--bg-hover);
    color: var(--danger);
  }

  .none {
    padding: 10px;
    color: var(--fg-subtle);
  }
</style>
