<!-- Branch list: switch with a click; merge, rebase, branch off or delete from the "..." menu. -->
<script lang="ts">
  import type { Branch } from '../api'
  import { formatRelative } from '../format'
  import { t } from '../i18n/index.svelte'
  import { menu } from '../menu.svelte'
  import { app } from '../state.svelte'
  import Icon from './Icon.svelte'
  import Modal from './Modal.svelte'

  const filtered = $derived.by(() => {
    const q = app.branchFilter.trim().toLowerCase()
    return q ? app.branches.filter((b) => b.name.toLowerCase().includes(q)) : app.branches
  })
  // Current branch first, then by most recent commit (the backend's order).
  const local = $derived(filtered.filter((b) => b.kind === 'local').sort((a, b) => Number(b.current) - Number(a.current)))
  const remote = $derived(filtered.filter((b) => b.kind === 'remote'))

  function branchMenu(b: Branch, e: MouseEvent | KeyboardEvent, anchor?: HTMLElement) {
    const idle = app.busy === null
    const onBranch = !!app.status?.branch
    menu.open(
      e,
      [
        { label: t('branches.switch'), action: () => app.switchBranch(b), disabled: !idle || b.current },
        'separator',
        { label: t('branches.merge'), action: () => app.mergeBranch(b), disabled: !idle || b.current || !onBranch },
        { label: t('branches.rebase'), action: () => app.rebaseOnto(b), disabled: !idle || b.current || !onBranch },
        { label: t('branches.createFrom'), action: () => app.openNewBranch(b.fullRef, b.name), disabled: !idle },
        'separator',
        { label: t('common.copy'), action: () => app.copy(b.name) },
        ...(b.kind === 'local'
          ? ['separator' as const, { label: t('branches.delete'), action: () => app.deleteBranch(b), disabled: !idle || b.current, danger: true }]
          : []),
      ],
      anchor,
    )
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && filtered.length > 0) {
      e.preventDefault()
      void app.switchBranch(local[0] ?? remote[0])
    }
  }
</script>

{#snippet item(b: Branch)}
  <li class:current={b.current}>
    <button class="row" disabled={app.busy !== null} onclick={() => app.switchBranch(b)} oncontextmenu={(e) => branchMenu(b, e)}>
      <span class="check">{#if b.current}<Icon name="check" size={14} />{/if}</span>
      <span class="name">{b.name}</span>
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

<Modal title={t('branches.title')} width={560} onclose={() => app.closeBranches()}>
  <div class="top">
    <div class="search">
      <Icon name="search" size={14} />
      <input type="text" placeholder={t('branches.filter')} bind:value={app.branchFilter} {onkeydown} aria-label={t('branches.filter')} />
    </div>
    <button class="btn" disabled={app.busy !== null || !app.status?.headOid} onclick={() => app.openNewBranch()}>
      <Icon name="plus" size={14} />
      {t('branches.new')}
    </button>
  </div>

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
    {#if !local.length && !remote.length}
      <div class="none">{t('branches.none')}</div>
    {/if}
  </div>
</Modal>

<style>
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
