<!-- Settings: the app's name, version, author and where its source and licenses are. -->
<script lang="ts">
  import { api } from '../../api'
  import { t } from '../../i18n/index.svelte'
  import { settings } from '../../settings.svelte'
  import { app } from '../../state.svelte'
  import Icon from '../Icon.svelte'
  import Logo from '../Logo.svelte'
  import SettingRow from './SettingRow.svelte'

  const SOURCE = 'https://github.com/valneko-twoon2/Leveret'
</script>

<div class="hero">
  <Logo size={56} />
  <div>
    <div class="name">{t('app.name')}</div>
    <div class="tagline">{t('settings.about.tagline')}</div>
    <div class="version">{t('settings.about.version', { version: settings.info?.version ?? '' })}</div>
  </div>
</div>

<SettingRow label={t('settings.about.author')}>
  <button class="btn" onclick={() => app.guard(() => api.openGithubUrl(SOURCE))}><Icon name="external" size={14} /> {t('settings.about.source')}</button>
</SettingRow>

<SettingRow label={t('settings.about.license')} hint={t('settings.about.licenseHint')}>
  <button class="btn" onclick={() => (settings.section = 'licenses')}><Icon name="license" size={14} /> {t('settings.sections.licenses')}</button>
</SettingRow>

<style>
  .hero {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 12px 0 16px;
    border-bottom: 1px solid var(--border-muted);
  }

  .name {
    font-size: 22px;
    font-weight: 700;
  }

  .tagline {
    color: var(--fg-muted);
  }

  .version {
    margin-top: 4px;
    color: var(--fg-subtle);
    font-size: 12px;
  }

  .btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
</style>
