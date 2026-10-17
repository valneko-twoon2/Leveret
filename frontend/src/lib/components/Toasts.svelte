<script lang="ts">
  import { t } from '../i18n/index.svelte'
  import { app } from '../state.svelte'
  import Icon from './Icon.svelte'
</script>

<div class="toasts" aria-live="polite">
  {#each app.toasts as toast (toast.id)}
    <div class="toast {toast.kind}">
      <Icon name={toast.kind === 'error' ? 'alert' : 'check'} size={15} />
      <span>{toast.message}</span>
      <button class="icon-btn" title={t('common.close')} aria-label={t('common.close')} onclick={() => app.dismissToast(toast.id)}>
        <Icon name="x" size={13} />
      </button>
    </div>
  {/each}
</div>

<style>
  .toasts {
    position: fixed;
    right: 16px;
    bottom: 16px;
    z-index: 90;
    display: flex;
    flex-direction: column;
    gap: 8px;
    pointer-events: none;
  }

  .toast {
    display: flex;
    align-items: center;
    gap: 8px;
    max-width: 420px;
    padding: 6px 6px 6px 12px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg);
    box-shadow: var(--shadow);
    pointer-events: auto;
  }

  .toast.info :global(svg:first-child) {
    color: var(--success);
  }

  .toast.error :global(svg:first-child) {
    color: var(--danger);
  }

  span {
    flex: 1;
    overflow-wrap: anywhere;
  }
</style>
