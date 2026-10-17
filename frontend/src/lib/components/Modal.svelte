<!-- Dialog shell: backdrop, title bar, Escape to close. -->
<script lang="ts">
  import type { Snippet } from 'svelte'
  import Icon from './Icon.svelte'

  interface Props {
    title: string
    width?: number
    /** Called on Escape, backdrop click and the close button. Omit to make the dialog non-dismissable. */
    onclose?: () => void
    children: Snippet
    footer?: Snippet
  }
  let { title, width = 520, onclose, children, footer }: Props = $props()

  let dialog = $state<HTMLDivElement>()
  $effect(() => {
    // Focus the first field, else the first button, else the dialog itself.
    const first =
      dialog?.querySelector<HTMLElement>('input, textarea') ?? dialog?.querySelector<HTMLElement>('button:not(.close)')
    ;(first ?? dialog)?.focus()
  })
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && onclose?.()} />

<div class="backdrop" role="presentation" onmousedown={() => onclose?.()}>
  <div
    class="dialog"
    style:width="min({width}px, calc(100vw - 48px))"
    role="dialog"
    aria-modal="true"
    aria-label={title}
    tabindex="-1"
    bind:this={dialog}
    onmousedown={(e) => e.stopPropagation()}
  >
    <header>
      <h3>{title}</h3>
      {#if onclose}
        <button class="icon-btn close" aria-label="close" onclick={onclose}><Icon name="x" /></button>
      {/if}
    </header>
    <div class="content">{@render children()}</div>
    {#if footer}
      <footer>{@render footer()}</footer>
    {/if}
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 95;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(0, 0, 0, 0.35);
  }

  .dialog {
    display: flex;
    flex-direction: column;
    max-height: calc(100vh - 48px);
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--bg);
    box-shadow: var(--shadow);
    outline: none;
  }

  header {
    display: flex;
    align-items: center;
    padding: 12px 12px 12px 20px;
    border-bottom: 1px solid var(--border-muted);
  }

  h3 {
    flex: 1;
    margin: 0;
    font-size: 15px;
  }

  .content {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 16px 20px;
  }

  footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 12px 20px;
    border-top: 1px solid var(--border-muted);
  }
</style>
