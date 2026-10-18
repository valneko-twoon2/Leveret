<script lang="ts">
  import { branches } from '../branches.svelte'
  import { t } from '../i18n/index.svelte'
  import { app } from '../state.svelte'
  import Modal from './Modal.svelte'

  const request = $derived(branches.newBranch)
  const renaming = $derived(request?.rename != null)
  const canCreate = $derived(branches.newBranchName.trim() !== '' && !branches.newBranchError && app.busy === null)

  // Existing names are needed to warn about duplicates (loaded once per dialog).
  let loadedFor: object | null = null
  $effect(() => {
    if (request && request !== loadedFor) {
      loadedFor = request
      if (!branches.branchesOpen) void branches.loadBranches()
    }
  })
</script>

{#snippet footer()}
  <button class="btn" onclick={() => branches.closeNewBranch()}>{t('common.cancel')}</button>
  <button class="btn primary" disabled={!canCreate} onclick={() => branches.createBranch()}>
    {t(renaming ? 'newBranch.renameSubmit' : 'newBranch.submit')}
  </button>
{/snippet}

<Modal closeOnBackdrop={false} title={t(renaming ? 'newBranch.renameTitle' : 'newBranch.title')} width={460} onclose={() => branches.closeNewBranch()} {footer}>
  <form
    onsubmit={(e) => {
      e.preventDefault()
      void branches.createBranch()
    }}
  >
    <label class="field">
      <span>{t('newBranch.name')}</span>
      <input
        type="text"
        placeholder={t('newBranch.placeholder')}
        bind:value={branches.newBranchName}
        aria-invalid={branches.newBranchError !== null}
        spellcheck="false"
        autocomplete="off"
      />
    </label>
    {#if branches.newBranchError}
      <p class="error">{branches.newBranchError}</p>
    {/if}
    {#if renaming}
      <p class="from">{t('newBranch.renameNote')}</p>
    {:else}
      <p class="from">{request?.label ? t('newBranch.from', { start: request.label }) : t('newBranch.fromHead')}</p>
      <label class="check">
        <input type="checkbox" bind:checked={branches.newBranchCheckout} />
        {t('newBranch.checkout')}
      </label>
    {/if}
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
