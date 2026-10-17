<!-- Branches and tags. Branches: switch with a click; merge, rebase, branch off or delete from the "..." menu.
     Tags: see TagList. -->
<script lang="ts">
  import type { Branch } from '../api'
  import { branches } from '../branches.svelte'
  import { formatRelative } from '../format'
  import { t } from '../i18n/index.svelte'
  import { menu } from '../menu.svelte'
  import { app } from '../state.svelte'
  import { tags } from '../tags.svelte'
  import Icon from './Icon.svelte'
  import Modal from './Modal.svelte'
  import TagList from './TagList.svelte'

  const filtered = $derived.by(() => {
    const q = branches.branchFilter.trim().toLowerCase()
    return q ? branches.branches.filter((b) => b.name.toLowerCase().includes(q)) : branches.branches
  })
  // Current branch first, then by most recent commit (the backend's order).
  const local = $derived(filtered.filter((b) => b.kind === 'local').sort((a, b) => Number(b.current) - Number(a.current)))
  // A remote branch a local branch already tracks is reached through that local branch.
  const tracked = $derived(new Set(branches.branches.map((b) => b.upstream).filter(Boolean)))
  const remote = $derived(filtered.filter((b) => b.kind === 'remote' && !tracked.has(b.name)))

  function branchMenu(b: Branch, e: MouseEvent | KeyboardEvent, anchor?: HTMLElement) {
    const idle = app.busy === null
    const onBranch = !!app.status?.branch
    menu.open(
      e,
      [
        { label: t('branches.switch'), action: () => branches.switchBranch(b), disabled: !idle || b.current },
        'separator',
        { label: t('branches.merge'), action: () => branches.mergeBranch(b), disabled: !idle || b.current || !onBranch },
        { label: t('branches.rebase'), action: () => branches.rebaseOnto(b), disabled: !idle || b.current || !onBranch },
        { label: t('branches.createFrom'), action: () => branches.openNewBranch(b.fullRef, b.name), disabled: !idle },
        'separator',
        { label: t('branches.copyName'), action: () => app.copy(b.name) },
        ...(b.kind === 'local'
          ? ['separator' as const, { label: t('branches.delete'), action: () => branches.deleteBranch(b), disabled: !idle || b.current, danger: true }]
          : []),
      ],
      anchor,
    )
  }

  const onTags = $derived(branches.dialogTab === 'tags')

  function onkeydown(e: KeyboardEvent) {
    if (!onTags && e.key === 'Enter' && filtered.length > 0) {
      e.preventDefault()
      void branches.switchBranch(local[0] ?? remote[0])
    }
  }
</script>

{#snippet item(b: Branch)}
  <li class:current={b.current}>
    <button class="row" disabled={app.busy !== null} onclick={() => branches.switchBranch(b)} oncontextmenu={(e) => branchMenu(b, e)}>
      <span class="check">{#if b.current}<Icon name="check" size={14} />{/if}</span>
      <span class="name" title={b.name}>{b.name}</span>
      {#if b.upstreamGone}<span class="tag">{t('branches.gone')}</span>{/if}
      {#if b.ahead || b.behind}
        <span class="counts">
          {#if b.behind}<span><Icon name="arrowDown" size={12} />{b.behind}</span>{/if}
          {#if b.ahead}<span><Icon name="arrowUp" size={12} />{b.ahead}</span>{/if}
        </span>
      {/if}
      <span class="time">{b.time ? formatRelative(b.time) : ''}</span>
    </button>
    <button
      class="icon-btn more"
      title={t('common.more')}
      aria-label={t('common.more')}
      onclick={(e) => branchMenu(b, e, e.currentTarget as HTMLElement)}
    >
      <Icon name="more" size={16} />
    </button>
  </li>
{/snippet}

<Modal title={t(onTags ? 'tags.title' : 'branches.title')} width={600} onclose={() => branches.closeBranches()}>
  <div class="tabs" role="tablist">
    <button role="tab" aria-selected={!onTags} class:active={!onTags} onclick={() => branches.setDialogTab('branches')}>
      <Icon name="branch" size={14} />
      {t('branches.tabBranches')}
    </button>
    <button role="tab" aria-selected={onTags} class:active={onTags} onclick={() => branches.setDialogTab('tags')}>
      <Icon name="tag" size={14} />
      {t('branches.tabTags')}
    </button>
  </div>
  <div class="top">
    <div class="search">
      <Icon name="search" size={14} />
      <input
        type="text"
        placeholder={t(onTags ? 'tags.filter' : 'branches.filter')}
        bind:value={branches.branchFilter}
        {onkeydown}
        aria-label={t(onTags ? 'tags.filter' : 'branches.filter')}
      />
    </div>
    {#if onTags}
      <button class="btn" disabled={app.busy !== null || !app.status?.headOid} onclick={() => tags.openNewTag()}>
        <Icon name="plus" size={14} />
        {t('tags.new')}
      </button>
    {:else}
      <button class="btn" disabled={app.busy !== null || !app.status?.headOid} onclick={() => branches.openNewBranch()}>
        <Icon name="plus" size={14} />
        {t('branches.new')}
      </button>
    {/if}
  </div>

  {#if onTags}
    <div class="lists"><TagList filter={branches.branchFilter} /></div>
  {:else}
    <div class="lists">
      {#if local.length}
        <h4>{t('branches.local')}</h4>
        <ul>
          {#each local as b (b.fullRef)}{@render item(b)}{/each}
        </ul>
      {/if}
      {#if remote.length}
        <h4>{t('branches.remote')}</h4>
        <ul>
          {#each remote as b (b.fullRef)}{@render item(b)}{/each}
        </ul>
      {/if}
      {#if !branches.branchesLoaded}
        <div class="none">{t('common.loading')}</div>
      {:else if !local.length && !remote.length}
        <div class="none">{t('branches.none')}</div>
      {/if}
    </div>
  {/if}
</Modal>

<style>
  .tabs {
    display: flex;
    gap: 4px;
    margin: -4px 0 10px;
    border-bottom: 1px solid var(--border);
  }

  .tabs button {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 32px;
    padding: 0 12px;
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

  .top {
    display: flex;
    gap: 8px;
    margin-bottom: 10px;
  }

  .search {
    flex: 1;
    position: relative;
    display: flex;
    align-items: center;
    color: var(--fg-muted);
  }

  .search :global(svg) {
    position: absolute;
    left: 9px;
  }

  .search input {
    width: 100%;
    padding-left: 28px;
  }

  .top .btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .lists {
    height: min(420px, 55vh);
    overflow-y: auto;
    margin: 0 -8px;
  }

  h4 {
    margin: 8px 8px 4px;
    font-size: 11px;
    font-weight: 600;
    color: var(--fg-muted);
  }

  ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  li {
    position: relative;
    display: flex;
    align-items: center;
  }

  .row {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    height: 32px;
    padding: 0 40px 0 8px;
    border: none;
    border-radius: var(--radius);
    background: transparent;
    text-align: left;
  }

  .row:hover:not(:disabled),
  .row:focus-visible {
    background: var(--bg-hover);
  }

  li.current .row {
    font-weight: 600;
  }

  .check {
    display: inline-flex;
    width: 14px;
    color: var(--accent);
  }

  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .tag {
    padding: 0 6px;
    border-radius: 999px;
    background: var(--bg-muted);
    color: var(--fg-muted);
    font-size: 11px;
    font-weight: 400;
  }

  .counts {
    display: inline-flex;
    gap: 6px;
    color: var(--fg-muted);
    font-size: 12px;
    font-weight: 400;
  }

  .counts span {
    display: inline-flex;
    align-items: center;
  }

  .time {
    flex: none;
    width: 90px;
    text-align: right;
    color: var(--fg-subtle);
    font-size: 12px;
    font-weight: 400;
  }

  .more {
    position: absolute;
    right: 6px;
  }

  .none {
    padding: 24px;
    text-align: center;
    color: var(--fg-muted);
  }
</style>
