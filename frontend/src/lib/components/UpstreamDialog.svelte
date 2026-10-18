<!-- Sets which remote branch a local branch tracks (what pull and push use), or removes it. -->
<script lang="ts">
  import { branches } from '../branches.svelte'
  import { t } from '../i18n/index.svelte'
  import { app } from '../state.svelte'
  import Modal from './Modal.svelte'

  const target = $derived(branches.upstreamFor)
  const remoteBranches = $derived(branches.branches.filter((b) => b.kind === 'remote'))
</script>

{#if target}
  <Modal title={t('upstream.title')} width={460} closeOnBackdrop={false} onclose={() => branches.closeUpstream()}>
    <p class="intro">{t('upstream.intro', { branch: target.name })}</p>
    <label class="field">
      <select bind:value={branches.upstreamChoice}>
        <option value="">{t('upstream.none')}</option>
        {#each remoteBranches as b (b.fullRef)}
          <option value={b.fullRef}>{b.name}</option>
        {/each}
      </select>
    </label>

    {#snippet footer()}
      <button class="btn" onclick={() => branches.closeUpstream()}>{t('common.cancel')}</button>
      <button class="btn primary" disabled={app.busy !== null} onclick={() => branches.saveUpstream()}>{t('upstream.submit')}</button>
    {/snippet}
  </Modal>
{/if}

<style>
  .intro {
    margin: 0 0 10px;
    color: var(--fg-muted);
    font-size: 12px;
  }

  select {
    width: 100%;
    height: 30px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg);
  }
</style>
