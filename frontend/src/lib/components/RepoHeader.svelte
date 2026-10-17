<script lang="ts">
  import type { RemoteOp } from '../api'
  import { formatRelative } from '../format'
  import { t } from '../i18n/index.svelte'
  import { layout } from '../layout.svelte'
  import { menu } from '../menu.svelte'
  import { app } from '../state.svelte'
  import Icon, { type IconName } from './Icon.svelte'

  const status = $derived(app.status)
  const branchLabel = $derived(status ? (status.branch ?? t('repo.detached')) : '')
  const remoteBusy = $derived(app.busy === 'fetch' || app.busy === 'pull' || app.busy === 'push' || app.busy === 'sync')

  interface Action {
    op: RemoteOp
    label: string
    detail: string
    icon: IconName
    hint: string
  }

  /** The most useful remote action right now, like GitHub Desktop's toolbar button. */
  const action = $derived.by((): Action | null => {
    if (!status?.hasRemote) return null
    const fetched = status.lastFetched
      ? t('remote.lastFetched', { time: formatRelative(status.lastFetched) })
      : t('remote.neverFetched')
    const fetch: Action = { op: 'fetch', label: t('remote.fetch'), detail: fetched, icon: 'refresh', hint: t('remote.fetchHint') }
    if (!status.branch || !status.headOid) return fetch
    if (!status.upstream) {
      return { op: 'push', label: t('remote.publish'), detail: t('repo.noUpstream'), icon: 'arrowUp', hint: t('remote.publishHint') }
    }
    if (status.behind > 0 && status.ahead > 0) {
      return { op: 'sync', label: `${t('remote.sync')} ↓${status.behind} ↑${status.ahead}`, detail: fetched, icon: 'sync', hint: t('remote.syncHint') }
    }
    if (status.behind > 0) {
      return { op: 'pull', label: `${t('remote.pull')} ↓${status.behind}`, detail: fetched, icon: 'arrowDown', hint: t('remote.pullHint', { count: status.behind }) }
    }
    if (status.ahead > 0) {
      return { op: 'push', label: `${t('remote.push')} ↑${status.ahead}`, detail: fetched, icon: 'arrowUp', hint: t('remote.pushHint', { count: status.ahead }) }
    }
    return fetch
  })

  const busyLabel = $derived.by(() => {
    switch (app.busy) {
      case 'fetch':
        return t('remote.fetching')
      case 'pull':
        return t('remote.pulling')
      case 'push':
        return t('remote.pushing')
      case 'sync':
        return t('remote.syncing')
      default:
        return ''
    }
  })

  function moreMenu(e: MouseEvent) {
    const st = app.status
    const idle = app.busy === null
    const onBranch = !!st?.branch
    menu.open(
      e,
      [
        { label: t('remote.fetch'), hint: t('shortcuts.fetch'), action: () => app.remote('fetch'), disabled: !idle || !st?.hasRemote },
        { label: t('remote.pull'), hint: t('shortcuts.pull'), action: () => app.remote('pull'), disabled: !idle || !st?.upstream || !onBranch },
        {
          label: st?.upstream ? t('remote.push') : t('remote.publish'),
          hint: t('shortcuts.push'),
          action: () => app.remote('push'),
          disabled: !idle || !st?.hasRemote || !onBranch || !st.headOid,
        },
        { label: t('remote.sync'), action: () => app.remote('sync'), disabled: !idle || !st?.upstream || !onBranch },
        'separator',
        { label: t('branches.new'), hint: t('shortcuts.newBranch'), action: () => app.openNewBranch(), disabled: !idle || !st?.headOid },
        { label: t('stash.save'), action: () => app.stashSave(), disabled: !idle || app.files.length === 0 },
        { label: t('commit.undo'), action: () => app.undoLastCommit(), disabled: !idle || !app.canRewriteHead },
        'separator',
        { label: t('common.openInExplorer'), action: () => app.showInExplorer() },
      ],
      e.currentTarget as HTMLElement,
    )
  }
</script>

<header class="repo-header">
  <div class="cell repo" style:width="{layout.sizes.column}px" title={app.repo?.path}>
    <Icon name="repo" />
    <div class="text">
      <span class="label">{t('repos.title')}</span>
      <span class="value">{app.repo?.name ?? ''}</span>
    </div>
  </div>

  <button class="cell branch" title={`${t('branches.open')} (${t('shortcuts.branches')})`} onclick={() => app.openBranches()}>
    <Icon name="branch" />
    <div class="text">
      <span class="label">{t('repo.branch')}</span>
      <span class="value">{branchLabel}</span>
    </div>
    {#if app.operation}
      <span class="pill warn">{app.operationName()}</span>
    {/if}
    <Icon name="chevronDown" size={14} />
  </button>

  {#if action}
    <button
      class="cell remote"
      title={action.hint}
      disabled={app.busy !== null}
      onclick={() => app.remote(action.op)}
    >
      <Icon name={remoteBusy ? 'refresh' : action.icon} />
      <div class="text">
        <span class="value">{remoteBusy ? busyLabel : action.label}</span>
        <span class="label">
          {#if remoteBusy && app.remoteProgress}
            {app.remoteProgress.phase} {app.remoteProgress.percent}%
          {:else}
            {action.detail}
          {/if}
        </span>
      </div>
    </button>
  {:else if status}
    <div class="cell remote static">
      <Icon name="cloud" />
      <div class="text">
        <span class="value muted">{t('remote.noRemote')}</span>
        {#if !status.headOid}<span class="label">{t('repo.noCommits')}</span>{/if}
      </div>
    </div>
  {/if}

  <div class="spacer"></div>

  <button class="icon-btn" title={t('common.more')} aria-label={t('common.more')} onclick={moreMenu}>
    <Icon name="more" size={18} />
  </button>
  <button class="icon-btn refresh" title={`${t('common.refresh')} (F5)`} aria-label={t('common.refresh')} onclick={() => app.refresh()}>
    <Icon name="refresh" />
  </button>

  {#if remoteBusy}
    <div class="progress" class:indeterminate={!app.remoteProgress}>
      <div class="bar" style:width="{app.remoteProgress?.percent ?? 30}%"></div>
    </div>
  {/if}
</header>

<style>
  .repo-header {
    position: relative;
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
    border: none;
    border-right: 1px solid var(--border-muted);
    background: transparent;
    color: var(--fg-muted);
    text-align: left;
  }

  button.cell:hover:not(:disabled) {
    background: var(--bg-hover);
  }

  button.cell:disabled {
    opacity: 1;
  }

  .repo {
    flex: none;
  }

  .branch {
    min-width: 200px;
    max-width: 360px;
  }

  .remote {
    min-width: 190px;
    max-width: 320px;
  }

  .text {
    flex: 1;
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

  .muted {
    color: var(--fg-muted);
    font-weight: 400;
  }

  .pill {
    flex: none;
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

  .icon-btn {
    align-self: center;
  }

  .refresh {
    margin: 0 10px 0 2px;
  }

  .progress {
    position: absolute;
    left: 0;
    right: 0;
    bottom: -1px;
    height: 2px;
    overflow: hidden;
  }

  .bar {
    height: 100%;
    background: var(--accent);
    transition: width 0.2s;
  }

  .indeterminate .bar {
    animation: slide 1.2s ease-in-out infinite;
  }

  @keyframes slide {
    from {
      transform: translateX(-100%);
    }
    to {
      transform: translateX(340%);
    }
  }
</style>
