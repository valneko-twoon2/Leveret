<!-- The repository's remotes: see their URLs, add, edit (name, URL) and remove them. -->
<script lang="ts">
  import { t } from '../i18n/index.svelte'
  import { remotes } from '../remotes.svelte'
  import { app } from '../state.svelte'
  import Icon from './Icon.svelte'
  import Modal from './Modal.svelte'

  const draft = $derived(remotes.draft)
</script>

<Modal title={t('remotes.title')} width={620} onclose={() => remotes.closeDialog()}>
  <p class="intro">{t('remotes.intro')}</p>

  <ul>
    {#each remotes.remotes as r (r.name)}
      <li>
        <Icon name="cloud" size={15} />
        <div class="text">
          <span class="name">{r.name}</span>
          <span class="url mono" title={r.fetchUrl}>{r.fetchUrl}</span>
          {#if r.pushUrl}<span class="url mono" title={r.pushUrl}>{t('remotes.pushUrlLabel')}: {r.pushUrl}</span>{/if}
        </div>
        <button class="icon-btn" title={t('remotes.edit')} aria-label={t('remotes.edit')} disabled={app.busy !== null} onclick={() => remotes.startEdit(r)}>
          <Icon name="edit" size={15} />
        </button>
        <button class="icon-btn danger" title={t('remotes.remove')} aria-label={t('remotes.remove')} disabled={app.busy !== null} onclick={() => remotes.remove(r)}>
          <Icon name="trash" size={15} />
        </button>
      </li>
    {:else}
      <li class="none">{remotes.loaded ? t('remotes.none') : t('common.loading')}</li>
    {/each}
  </ul>

  {#if draft}
    <form
      onsubmit={(e) => {
        e.preventDefault()
        void remotes.save()
      }}
    >
      <label class="field">
        <span>{t('remotes.name')}</span>
        <input type="text" bind:value={draft.name} placeholder={t('remotes.namePlaceholder')} spellcheck="false" autocomplete="off" />
      </label>
      <label class="field">
        <span>{t('remotes.url')}</span>
        <input type="text" bind:value={draft.url} placeholder={t('remotes.urlPlaceholder')} spellcheck="false" autocomplete="off" />
      </label>
      <label class="field">
        <span>{t('remotes.pushUrl')}</span>
        <input type="text" bind:value={draft.pushUrl} spellcheck="false" autocomplete="off" />
      </label>
      {#if remotes.draftError}<p class="error">{remotes.draftError}</p>{/if}
      <div class="actions">
        <button type="button" class="btn" onclick={() => remotes.cancelDraft()}>{t('remotes.cancel')}</button>
        <button type="submit" class="btn primary" disabled={!remotes.canSave}>{t('remotes.save')}</button>
      </div>
    </form>
  {/if}

  {#snippet footer()}
    <button class="btn" disabled={draft !== null || app.busy !== null} onclick={() => remotes.startAdd()}>
      <Icon name="plus" size={14} />
      {t('remotes.add')}
    </button>
    <button class="btn primary" onclick={() => remotes.closeDialog()}>{t('common.close')}</button>
  {/snippet}
</Modal>

<style>
  .intro {
    margin: 0 0 10px;
    color: var(--fg-muted);
    font-size: 12px;
  }

  ul {
    margin: 0;
    padding: 0;
    list-style: none;
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }

  li {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    border-bottom: 1px solid var(--border-muted);
  }

  li:last-child {
    border-bottom: none;
  }

  li.none {
    justify-content: center;
    color: var(--fg-subtle);
  }

  .text {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .name {
    font-weight: 600;
  }

  .url {
    overflow: hidden;
    color: var(--fg-muted);
    font-size: 12px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .danger:hover {
    color: var(--danger);
  }

  form {
    display: flex;
    flex-direction: column;
    gap: 10px;
    margin-top: 12px;
    padding: 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-subtle);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .field span {
    font-weight: 600;
  }

  .error {
    margin: 0;
    color: var(--danger);
    font-size: 12px;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
</style>
