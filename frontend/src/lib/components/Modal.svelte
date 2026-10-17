<!-- Dialog shell: backdrop, title bar, Escape to close. Focus stays inside while it is
     open (Tab wraps around) and returns to where it was when it closes. -->
<script lang="ts">
  import type { Snippet } from 'svelte'
  import { t } from '../i18n/index.svelte'
  import Icon from './Icon.svelte'

  interface Props {
    title: string
    width?: number
    /** Called on Escape, backdrop click and the close button. Omit to make the dialog non-dismissable. */
    onclose?: () => void
    /** Close on a click outside the dialog. Off for forms, so a stray click doesn't lose typed text. */
    closeOnBackdrop?: boolean
    children: Snippet
    footer?: Snippet
  }
  let { title, width = 520, onclose, closeOnBackdrop = true, children, footer }: Props = $props()

  let dialog = $state<HTMLDivElement>()
  $effect(() => {
    const opener = document.activeElement instanceof HTMLElement ? document.activeElement : null
    // Focus the first field, else the first button, else the dialog itself.
    const first =
      dialog?.querySelector<HTMLElement>('input, textarea') ?? dialog?.querySelector<HTMLElement>('button:not(.close)')
    ;(first ?? dialog)?.focus()
    return () => {
      if (opener?.isConnected) opener.focus()
    }
  })

  const FOCUSABLE = 'button:not(:disabled), input:not(:disabled), textarea:not(:disabled), select:not(:disabled), [tabindex]:not([tabindex="-1"])'

  /** Keeps Tab / Shift+Tab inside the dialog. */
  function trapTab(e: KeyboardEvent) {
    if (e.key !== 'Tab' || !dialog) return
    const items = [...dialog.querySelectorAll<HTMLElement>(FOCUSABLE)].filter((el) => el.offsetParent !== null)
    if (items.length === 0) return
    const first = items[0]
    const last = items[items.length - 1]
    if (e.shiftKey && (document.activeElement === first || document.activeElement === dialog)) {
      e.preventDefault()
      last.focus()
    } else if (!e.shiftKey && document.activeElement === last) {
      e.preventDefault()
      first.focus()
    }
  }
</script>

<svelte:window
  onkeydown={(e) => {
    if (e.key !== 'Escape' || e.defaultPrevented || !onclose) return
    // Only the topmost dialog closes (dialogs can be stacked).
    const dialogs = document.querySelectorAll('[role="dialog"]')
    if (dialogs[dialogs.length - 1] !== dialog) return
    e.preventDefault()
    onclose()
  }}
/>

<div class="backdrop" role="presentation" onmousedown={() => closeOnBackdrop && onclose?.()}>
  <div
    class="dialog"
    style:width="min({width}px, calc(100vw - 48px))"
    role="dialog"
    aria-modal="true"
    aria-label={title}
    tabindex="-1"
    bind:this={dialog}
    onmousedown={(e) => e.stopPropagation()}
    onkeydown={trapTab}
  >
    <header>
      <h3>{title}</h3>
      {#if onclose}
        <button class="icon-btn close" title={t('common.close')} aria-label={t('common.close')} onclick={onclose}><Icon name="x" /></button>
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
