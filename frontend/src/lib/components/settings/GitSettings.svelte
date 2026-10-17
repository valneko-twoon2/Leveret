<!-- Settings: the user's global git config (identity, default branch, pull, line endings). -->
<script lang="ts">
  import { isValidBranchName } from '../../branches.svelte'
  import { t } from '../../i18n/index.svelte'
  import { settings } from '../../settings.svelte'
  import SettingRow from './SettingRow.svelte'

  const git = $derived(settings.info?.git ?? null)
  let name = $state('')
  let email = $state('')
  let branch = $state('')

  // Fill the fields once the settings have loaded (and after each save).
  $effect(() => {
    if (!git) return
    name = git.userName
    email = git.userEmail
    branch = git.defaultBranch
  })

  const emailOk = $derived(email.trim() === '' || /^[^\s@]+@[^\s@]+$/.test(email.trim()))
  const identityChanged = $derived(!!git && (name.trim() !== git.userName || email.trim() !== git.userEmail))
  const branchOk = $derived(branch.trim() === '' || isValidBranchName(branch.trim()))

  async function saveIdentity() {
    if (!git || !emailOk) return
    if (name.trim() !== git.userName) await settings.saveGit('user.name', name)
    if (email.trim() !== git.userEmail) await settings.saveGit('user.email', email)
  }
</script>

{#if !git}
  <p class="loading">{t('common.loading')}</p>
{:else}
  <SettingRow label={t('settings.git.identity')} hint={t('settings.git.identityHint')} stacked>
    <form
      class="identity"
      onsubmit={(e) => {
        e.preventDefault()
        void saveIdentity()
      }}
    >
      <label>
        <span>{t('settings.git.userName')}</span>
        <input type="text" bind:value={name} placeholder={t('settings.git.unset')} autocomplete="off" spellcheck="false" />
      </label>
      <label>
        <span>{t('settings.git.userEmail')}</span>
        <input type="email" bind:value={email} placeholder={t('settings.git.unset')} aria-invalid={!emailOk} autocomplete="off" spellcheck="false" />
      </label>
      <button class="btn primary" type="submit" disabled={!identityChanged || !emailOk}>{t('settings.git.save')}</button>
    </form>
    {#if !emailOk}
      <p class="error">{t('settings.git.invalidEmail')}</p>
    {:else}
      <p class="note">{t('settings.git.emailHint')}</p>
    {/if}
  </SettingRow>

  <SettingRow label={t('settings.git.defaultBranch')}>
    <input class="branch" type="text" bind:value={branch} placeholder="master" aria-invalid={!branchOk} spellcheck="false" />
    <button class="btn" disabled={!branchOk || branch.trim() === git.defaultBranch} onclick={() => settings.saveGit('init.defaultBranch', branch)}>
      {t('settings.git.save')}
    </button>
  </SettingRow>

  <SettingRow label={t('settings.git.pull')}>
    <select value={git.pullRebase === 'true' ? 'true' : 'false'} onchange={(e) => settings.saveGit('pull.rebase', e.currentTarget.value === 'true' ? 'true' : '')} aria-label={t('settings.git.pull')}>
      <option value="false">{t('settings.git.pullMerge')}</option>
      <option value="true">{t('settings.git.pullRebase')}</option>
    </select>
  </SettingRow>

  <SettingRow label={t('settings.git.autocrlf')}>
    <select value={git.autocrlf} onchange={(e) => settings.saveGit('core.autocrlf', e.currentTarget.value)} aria-label={t('settings.git.autocrlf')}>
      <option value="">{t('settings.git.unset')}</option>
      <option value="true">{t('settings.git.autocrlfTrue')}</option>
      <option value="input">{t('settings.git.autocrlfInput')}</option>
      <option value="false">{t('settings.git.autocrlfFalse')}</option>
    </select>
  </SettingRow>

  <SettingRow label={t('settings.git.version')}>
    <span class="mono">{git.version}</span>
  </SettingRow>
{/if}

<style>
  .loading {
    color: var(--fg-muted);
  }

  .identity {
    display: flex;
    align-items: flex-end;
    gap: 8px;
  }

  .identity label {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 12px;
    color: var(--fg-muted);
  }

  .branch {
    width: 180px;
  }

  input[aria-invalid='true'] {
    border-color: var(--danger);
  }

  .error,
  .note {
    margin: 0;
    font-size: 12px;
  }

  .error {
    color: var(--danger);
  }

  .note {
    color: var(--fg-muted);
  }

  .mono {
    color: var(--fg-muted);
  }
</style>
