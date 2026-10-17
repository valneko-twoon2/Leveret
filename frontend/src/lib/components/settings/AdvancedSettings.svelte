<!-- Settings: the performance panel, the app's folders and resetting display settings. -->
<script lang="ts">
  import { t } from '../../i18n/index.svelte'
  import { settings } from '../../settings.svelte'
  import { app } from '../../state.svelte'
  import Icon from '../Icon.svelte'
  import SettingRow from './SettingRow.svelte'
</script>

<SettingRow label={t('settings.advanced.perfPanel')} hint={t('settings.advanced.perfPanelHint')}>
  <input type="checkbox" class="switch" bind:checked={app.perfOpen} aria-label={t('settings.advanced.perfPanel')} />
</SettingRow>

<SettingRow label={t('settings.advanced.folders')} stacked>
  <div class="folders">
    {#each [{ which: 'config', label: t('settings.advanced.configFolder'), path: settings.info?.configDir }, { which: 'logs', label: t('settings.advanced.logsFolder'), path: settings.info?.logsDir }] as f (f.which)}
      <div class="folder">
        <span class="name">{f.label}</span>
        <span class="path mono" title={f.path}>{f.path ?? ''}</span>
        <button class="btn" onclick={() => settings.openFolder(f.which as 'config' | 'logs')}><Icon name="folder" size={14} /> {t('settings.advanced.open')}</button>
      </div>
    {/each}
  </div>
</SettingRow>

<SettingRow label={t('settings.advanced.reset')} hint={t('settings.advanced.resetHint')}>
  <button class="btn danger" onclick={() => settings.resetDisplay()}>{t('settings.advanced.reset')}</button>
</SettingRow>

<style>
  .folders {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .folder {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .name {
    flex: none;
    width: 100px;
    color: var(--fg-muted);
  }

  .path {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 12px;
  }

  .btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .danger {
    color: var(--danger);
  }
</style>
