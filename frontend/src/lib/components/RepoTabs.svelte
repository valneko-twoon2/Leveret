<!-- The tabs above the left column: changes, history and issues. -->
<script lang="ts">
  import { t, type MessageKey } from '../i18n/index.svelte'
  import { issues } from '../issues.svelte'
  import { app, type Tab } from '../state.svelte'

  const tabs: { id: Tab; label: MessageKey; shortcut: MessageKey }[] = [
    { id: 'changes', label: 'tabs.changes', shortcut: 'shortcuts.changes' },
    { id: 'history', label: 'tabs.history', shortcut: 'shortcuts.history' },
    { id: 'issues', label: 'tabs.issues', shortcut: 'shortcuts.issues' },
  ]

  function count(tab: Tab): number {
    if (tab === 'changes') return app.files.length
    // Open issues once they have been loaded (the tab doesn't load them by itself).
    if (tab === 'issues') return issues.openCount ?? 0
    return 0
  }
</script>

<div class="tabs" role="tablist">
  {#each tabs as tab (tab.id)}
    {@const n = count(tab.id)}
    <button
      role="tab"
      class:active={app.tab === tab.id}
      aria-selected={app.tab === tab.id}
      title={t(tab.shortcut)}
      onclick={() => app.setTab(tab.id)}
    >
      {t(tab.label)}
      {#if n}<span class="count">{n}</span>{/if}
    </button>
  {/each}
</div>

<style>
  .tabs {
    display: flex;
    border-bottom: 1px solid var(--border);
  }

  .tabs button {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    height: 36px;
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

  .count {
    min-width: 18px;
    padding: 0 5px;
    border-radius: 999px;
    background: var(--bg-muted);
    font-size: 11px;
    line-height: 18px;
  }
</style>
