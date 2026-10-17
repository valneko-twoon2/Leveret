<!-- Settings: background fetch, the clone folder and the GitHub account. -->
<script lang="ts">
  import { github } from '../../github.svelte'
  import { t } from '../../i18n/index.svelte'
  import { AUTO_FETCH_MINUTES, prefs } from '../../prefs.svelte'
  import { settings } from '../../settings.svelte'
  import Icon from '../Icon.svelte'
  import SettingRow from './SettingRow.svelte'
</script>

<SettingRow label={t('settings.remote.autoFetch')} hint={t('settings.remote.autoFetchHint')}>
  <select value={prefs.autoFetchMinutes} onchange={(e) => prefs.set('autoFetchMinutes', Number(e.currentTarget.value))} aria-label={t('settings.remote.autoFetch')}>
    {#each AUTO_FETCH_MINUTES as minutes (minutes)}
      <option value={minutes}>{minutes ? t('settings.remote.autoFetchMinutes', { minutes }) : t('settings.remote.autoFetchOff')}</option>
    {/each}
  </select>
</SettingRow>

<SettingRow label={t('settings.remote.cloneDir')} hint={t('settings.remote.cloneDirHint')} stacked>
  <div class="folder">
    <Icon name="folder" size={15} />
    <span class="path mono" title={settings.info?.cloneDir}>{settings.info?.cloneDir ?? ''}</span>
    <button class="btn" onclick={() => settings.chooseCloneDir()}>{t('settings.remote.change')}</button>
    <button class="btn" onclick={() => settings.resetCloneDir()}>{t('settings.remote.reset')}</button>
  </div>
</SettingRow>

<SettingRow label={t('settings.remote.account')} hint={github.account ? t('settings.remote.signedInAs', { login: github.account.login }) : t('settings.remote.notSignedIn')}>
  {#if github.account}
    <button class="btn" onclick={() => github.signOut()}><Icon name="logout" size={14} /> {t('github.signOut')}</button>
  {:else}
    <button class="btn primary" onclick={() => github.openSignIn()}><Icon name="user" size={14} /> {t('github.signIn')}</button>
  {/if}
</SettingRow>

<style>
  .folder {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--fg-muted);
  }

  .path {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--fg);
    font-size: 12px;
  }

  .btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
</style>
