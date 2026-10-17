<!-- "New tag" dialog: a name and an optional message (which makes it an annotated tag). -->
<script lang="ts">
  import { t } from '../i18n/index.svelte'
  import { app } from '../state.svelte'
  import { tags } from '../tags.svelte'
  import Modal from './Modal.svelte'

  const request = $derived(tags.newTag)
  const canCreate = $derived(tags.newTagName.trim() !== '' && !tags.newTagError && app.busy === null)

  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && e.ctrlKey) {
      e.preventDefault()
      void tags.create()
    }
  }
</script>

{#snippet footer()}
  <button class="btn" onclick={() => tags.closeNewTag()}>{t('common.cancel')}</button>
  <button class="btn primary" disabled={!canCreate} onclick={() => tags.create()}>{t('tags.submit')}</button>
{/snippet}

<Modal closeOnBackdrop={false} title={t('tags.newTitle')} width={480} onclose={() => tags.closeNewTag()} {footer}>
  <form
    onsubmit={(e) => {
      e.preventDefault()
      void tags.create()
    }}
  >
    <label class="field">
      <span>{t('tags.name')}</span>
      <input
        type="text"
        placeholder="v1.0.0"
        bind:value={tags.newTagName}
        aria-invalid={tags.newTagError !== null}
        spellcheck="false"
        autocomplete="off"
      />
    </label>
    {#if tags.newTagError}
      <p class="error">{tags.newTagError}</p>
    {/if}
    <label class="field">
      <span>{t('tags.message')}</span>
      <textarea rows="4" placeholder={t('tags.messagePlaceholder')} bind:value={tags.newTagMessage} {onkeydown}></textarea>
    </label>
    <p class="from" title={request?.label}>{t('tags.at', { commit: request?.label ?? '' })}</p>
  </form>
</Modal>

<style>
  form {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .field span {
    font-weight: 600;
  }

  textarea {
    resize: vertical;
  }

  input[aria-invalid='true'] {
    border-color: var(--danger);
  }

  .error {
    margin: 0;
    color: var(--danger);
    font-size: 12px;
  }

  .from {
    margin: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--fg-muted);
    font-size: 12px;
  }
</style>
