<!-- Shown while a merge / rebase / revert / cherry-pick is stopped, or files are conflicted. -->
<script lang="ts">
  import { changes } from '../changes.svelte'
  import { t } from '../i18n/index.svelte'
  import { app } from '../state.svelte'
  import Icon from './Icon.svelte'

  const op = $derived(app.operation)
  const conflicts = $derived(app.conflicted.length)
  const idle = $derived(app.busy === null)
</script>

{#if op || conflicts}
  <div class="banner" class:resolved={conflicts === 0} role="status">
    <Icon name={conflicts ? 'alert' : 'check'} />
    <div class="text">
      <strong>
        {#if op}
          {app.operationName()}
          {#if op.kind === 'rebase' && op.step && op.total}<span class="step">{t('operation.step', { step: op.step, total: op.total })}</span>{/if}
        {/if}
        {#if op?.kind === 'rebase' && op.branch}<span class="sub">{t('operation.rebaseOf', { branch: op.branch })}</span>{/if}
      </strong>
      <span>
        {#if conflicts}
          {t('operation.conflicts', { count: conflicts })}
        {:else if op?.kind === 'merge'}
          {t('operation.mergeHint')}
        {:else}
          {t('operation.resolved')}
        {/if}
      </span>
    </div>
    {#if conflicts}
      <button class="btn" disabled={!idle} onclick={() => changes.markResolved(app.conflicted.map((f) => f.path))}>
        {t('discard.markResolved')} ({conflicts})
      </button>
    {/if}
    {#if op}
      {#if op.kind !== 'merge'}
        <button class="btn" disabled={!idle} onclick={() => app.skipOperation()}>{t('operation.skip')}</button>
      {/if}
      <button class="btn danger" disabled={!idle} onclick={() => app.abortOperation()}>{t('operation.abort')}</button>
      <button class="btn primary" disabled={!idle || conflicts > 0} onclick={() => app.continueOperation()}>
        {t('operation.continue')}
      </button>
    {/if}
  </div>
{/if}

<style>
  .banner {
    flex: none;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 12px 8px 16px;
    border-bottom: 1px solid color-mix(in srgb, var(--attention) 40%, transparent);
    background: color-mix(in srgb, var(--attention) 12%, var(--bg));
    color: var(--attention);
  }

  .banner.resolved {
    border-bottom-color: color-mix(in srgb, var(--success) 40%, transparent);
    background: color-mix(in srgb, var(--success) 10%, var(--bg));
    color: var(--success);
  }

  .text {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    color: var(--fg);
    line-height: 1.35;
  }

  .text span {
    color: var(--fg-muted);
    font-size: 12px;
  }

  .step,
  .sub {
    margin-left: 8px;
    font-weight: 400;
  }

  .btn {
    flex: none;
  }
</style>
