<!-- Renders the open context menu (menu.svelte.ts). Closes on outside click, Escape and resize. -->
<script lang="ts">
  import { tick } from 'svelte'
  import { menu, type MenuAction } from '../menu.svelte'

  let element = $state<HTMLDivElement>()
  let left = $state(0)
  let top = $state(0)

  // Keep the menu inside the window and focus its first item.
  $effect(() => {
    const m = menu.current
    if (!m || !element) return
    const r = element.getBoundingClientRect()
    left = Math.max(4, Math.min(m.x, window.innerWidth - r.width - 4))
    top = m.y + r.height > window.innerHeight - 4 ? Math.max(4, m.y - r.height) : m.y
    void tick().then(() => element?.querySelector<HTMLButtonElement>('button:not(:disabled)')?.focus())
  })

  // Close when the window really changes size (some hosts fire resize events without a change).
  let size = `${window.innerWidth}x${window.innerHeight}`
  function onresize() {
    const now = `${window.innerWidth}x${window.innerHeight}`
    if (now !== size) menu.close()
    size = now
  }

  function run(item: MenuAction) {
    menu.close()
    void Promise.resolve().then(item.action)
  }

  function onkeydown(e: KeyboardEvent) {
    if (!element) return
    const buttons = [...element.querySelectorAll<HTMLButtonElement>('button:not(:disabled)')]
    const i = buttons.indexOf(document.activeElement as HTMLButtonElement)
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault()
      const next = e.key === 'ArrowDown' ? (i + 1) % buttons.length : (i - 1 + buttons.length) % buttons.length
      buttons[next]?.focus()
    } else if (e.key === 'Escape' || e.key === 'Tab') {
      e.preventDefault()
      menu.close()
    }
  }
</script>

<svelte:window onresize={onresize} />

{#if menu.current}
  <div
    class="backdrop"
    role="presentation"
    onmousedown={() => menu.close()}
    oncontextmenu={(e) => {
      e.preventDefault()
      menu.close()
    }}
  ></div>
  <div class="menu" role="menu" tabindex="-1" bind:this={element} style:left="{left}px" style:top="{top}px" {onkeydown}>
    {#each menu.current.items as item, i (i)}
      {#if item === 'separator'}
        <div class="separator" role="separator"></div>
      {:else}
        <button role="menuitem" class:danger={item.danger} disabled={item.disabled} onclick={() => run(item)}>
          <span class="label">{item.label}</span>
          {#if item.hint}<span class="hint">{item.hint}</span>{/if}
        </button>
      {/if}
    {/each}
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 98;
  }

  .menu {
    position: fixed;
    z-index: 99;
    min-width: 200px;
    max-width: 420px;
    padding: 4px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg);
    box-shadow: var(--shadow);
    outline: none;
  }

  button {
    display: flex;
    align-items: center;
    gap: 24px;
    width: 100%;
    height: 28px;
    padding: 0 10px;
    border: none;
    border-radius: 4px;
    background: transparent;
    text-align: left;
    outline: none;
  }

  button:hover:not(:disabled),
  button:focus-visible {
    background: var(--bg-selected-strong);
    color: var(--fg-on-accent);
  }

  button:disabled {
    color: var(--fg-subtle);
  }

  button.danger:not(:disabled) {
    color: var(--danger);
  }

  button.danger:hover:not(:disabled),
  button.danger:focus-visible {
    background: var(--danger);
    color: var(--fg-on-accent);
  }

  .label {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .hint {
    font-size: 11px;
    opacity: 0.7;
  }

  .separator {
    height: 1px;
    margin: 4px 6px;
    background: var(--border-muted);
  }
</style>
