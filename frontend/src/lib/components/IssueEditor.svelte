<!-- Dialog for a new issue (saved locally or created on GitHub) or for editing one. -->
<script lang="ts">
  import { t } from '../i18n/index.svelte'
  import { issues } from '../issues.svelte'
  import Modal from './Modal.svelte'

  const draft = $derived(issues.draft)
  const title = $derived(draft?.title.trim() ?? '')
  const canSave = $derived(title !== '' && title.length <= 256 && !issues.saving)

  function save(e?: Event) {
    e?.preventDefault()
    if (canSave) void issues.saveDraft()
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === 'Enter' && e.ctrlKey) save(e)
  }
</script>

{#snippet footer()}
  <button class="btn" onclick={() => issues.closeDraft()}>{t('common.cancel')}</button>
  <button class="btn primary" disabled={!canSave} onclick={() => save()} title={t('shortcuts.submit')}>
    {draft?.mode === 'edit' ? t('issues.save') : draft?.source === 'github' ? t('issues.createOnGithub') : t('issues.createLocal')}
  </button>
{/snippet}

{#if draft}
  <Modal closeOnBackdrop={false} title={draft.mode === 'edit' ? t('issues.editTitle') : t('issues.newTitle')} width={620} onclose={() => issues.closeDraft()} {footer}>
    <form onsubmit={save}>
      <label class="field">
        <span>{t('issues.titleLabel')}</span>
        <!-- svelte-ignore a11y_autofocus -->
        <input type="text" bind:value={draft.title} maxlength="256" autofocus autocomplete="off" onkeydown={onKey} />
      </label>
      <label class="field">
        <span>{t('issues.bodyLabel')}</span>
        <textarea rows="10" bind:value={draft.body} placeholder={t('issues.bodyPlaceholder')} onkeydown={onKey}></textarea>
      </label>
      {#if draft.mode === 'new'}
        <fieldset>
          <legend>{t('issues.destination')}</legend>
          <label class="choice">
            <input type="radio" bind:group={draft.source} value="local" />
            <span><strong>{t('issues.destLocal')}</strong><small>{t('issues.destLocalHint')}</small></span>
          </label>
          <label class="choice" class:disabled={!issues.canUseGithub}>
            <input type="radio" bind:group={draft.source} value="github" disabled={!issues.canUseGithub} />
            <span>
              <strong>{issues.sources?.githubRepo ? t('issues.destGithub', { repo: issues.sources.githubRepo }) : t('issues.sourceGithub')}</strong>
              <small>{issues.canUseGithub ? t('issues.destGithubHint') : issues.sources?.githubRepo ? t('issues.signInHint') : t('issues.noGithubRemote')}</small>
            </span>
          </label>
        </fieldset>
      {/if}
    </form>
  </Modal>
{/if}

<style>
  form {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .field span,
  legend {
    font-weight: 600;
  }

  textarea {
    resize: vertical;
    min-height: 120px;
  }

  fieldset {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin: 0;
    padding: 0;
    border: none;
  }

  legend {
    margin-bottom: 4px;
    padding: 0;
  }

  .choice {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 8px 10px;
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

  .choice span {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .choice small {
    color: var(--fg-muted);
    font-size: 12px;
  }
</style>
