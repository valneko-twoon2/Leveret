<script lang="ts">
  import { t } from '../i18n/index.svelte'
  import { app } from '../state.svelte'
  import Modal from './Modal.svelte'

  const request = $derived(app.newBranch)
  const canCreate = $derived(app.newBranchName.trim() !== '' && !app.newBranchError && app.busy === null)

  // Existing names are needed to warn about duplicates (loaded once per dialog).
  let loadedFor: object | null = null
  $effect(() => {
    if (request && request !== loadedFor) {
      loadedFor = request
      if (!app.branchesOpen) void app.loadBranches()
    }
  })
</script>

{#snippet footer()}
  <button class="btn" onclick={() => app.closeNewBranch()}>{t('common.cancel')}</button>
  <button class="btn primary" disabled={!canCreate} onclick={() => app.createBranch()}>{t('newBranch.submit')}</button>
{/snippet}

<Modal title={t('newBranch.title')} width={460} onclose={() => app.closeNewBranch()} {footer}>
  <form
    onsubmit={(e) => {
      e.preventDefault()
      void app.createBranch()
    }}
  >
    <label class="field">
      <span>{t('newBranch.name')}</span>
      <input
        type="text"
        placeholder={t('newBranch.placeholder')}
        bind:value={app.newBranchName}
        aria-invalid={app.newBranchError !== null}
        spellcheck="false"
        autocomplete="off"
      />
    </label>
    {#if app.newBranchError}
      <p class="error">{app.newBranchError}</p>
    {/if}
    <p class="from">{request?.label ? t('newBranch.from', { start: request.label }) : t('newBranch.fromHead')}</p>
    <label class="check">
      <input type="checkbox" bind:checked={app.newBranchCheckout} />
      {t('newBranch.checkout')}
    </label>
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
    color: var(--fg-muted);
    font-size: 12px;
    overflow-wrap: anywhere;
  }

  .check {
    display: flex;
    align-items: center;
    gap: 8px;
  }
</style>
