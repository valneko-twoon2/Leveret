<!-- Confirms resetting the current branch to a commit, with what happens to the changes. -->
<script lang="ts">
  import type { ResetMode } from '../api'
  import { shortHash } from '../format'
  import { t } from '../i18n/index.svelte'
  import { rewrite } from '../rewrite.svelte'
  import { app } from '../state.svelte'
  import Icon from './Icon.svelte'
  import Modal from './Modal.svelte'

  const target = $derived(rewrite.reset)
  let mode = $state<ResetMode>('keep')
  const branch = $derived(app.status?.branch)
</script>

{#if target}
  {@const { entry, info } = target}
  <Modal title={t('reset.title')} width={520} onclose={() => rewrite.closeReset()} closeOnBackdrop={false}>
    <p class="intro">
      {branch
        ? t('reset.intro', { branch, hash: shortHash(entry.hash), subject: entry.subject })
        : t('reset.introDetached', { hash: shortHash(entry.hash), subject: entry.subject })}
    </p>
    <p class="count">{t('reset.count', { count: info.count })}</p>
    {#if info.pushed > 0}
      <p class="warning"><Icon name="alert" size={14} /> {t('reset.pushed', { count: info.pushed })}</p>
    {/if}

    <fieldset>
      <legend class="sr-only">{t('reset.title')}</legend>
      <label class="choice">
        <input type="radio" name="reset-mode" value="keep" bind:group={mode} />
        <span>
          <strong>{t('reset.keep')}</strong>
          <span class="hint">{t('reset.keepHint')}</span>
        </span>
      </label>
      <label class="choice" class:disabled={info.dirty}>
        <input type="radio" name="reset-mode" value="discard" bind:group={mode} disabled={info.dirty} />
        <span>
          <strong>{t('reset.discard')}</strong>
          <span class="hint">{info.dirty ? t('reset.discardDirty') : t('reset.discardHint')}</span>
        </span>
      </label>
    </fieldset>

    {#snippet footer()}
      <button class="btn" onclick={() => rewrite.closeReset()}>{t('common.cancel')}</button>
      <button class="btn primary" class:danger-fill={mode === 'discard'} disabled={app.busy !== null} onclick={() => rewrite.resetBranch(mode)}>
        {t('reset.button')}
      </button>
    {/snippet}
  </Modal>
{/if}

<style>
  p {
    margin: 0 0 8px;
  }

  .count {
    color: var(--fg-muted);
  }

  .warning {
    display: flex;
    gap: 6px;
    align-items: flex-start;
    padding: 8px 10px;
    border-radius: var(--radius);
    background: color-mix(in srgb, var(--attention) 12%, transparent);
    color: var(--attention);
  }

  .warning :global(svg) {
    flex: none;
    margin-top: 2px;
  }

  fieldset {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin: 12px 0 0;
    padding: 0;
    border: none;
  }

  .choice {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }

  .choice:has(input:checked) {
    border-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 8%, transparent);
  }

  .choice.disabled {
    opacity: 0.6;
  }

  .choice input {
    margin-top: 3px;
  }

  .choice > span {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .hint {
    color: var(--fg-muted);
    font-size: 12px;
  }

  .btn.danger-fill {
    background: var(--danger);
  }

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
  }
</style>
