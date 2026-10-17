<!-- The settings screen: sections on the left, the selected section on the right. -->
<script lang="ts">
  import { t } from '../../i18n/index.svelte'
  import { SECTIONS, settings, type Section } from '../../settings.svelte'
  import Icon, { type IconName } from '../Icon.svelte'
  import Modal from '../Modal.svelte'
  import AboutSettings from './AboutSettings.svelte'
  import AdvancedSettings from './AdvancedSettings.svelte'
  import AgentSettings from './AgentSettings.svelte'
  import ChangesSettings from './ChangesSettings.svelte'
  import GeneralSettings from './GeneralSettings.svelte'
  import GitSettings from './GitSettings.svelte'
  import LicenseSettings from './LicenseSettings.svelte'
  import RemoteSettings from './RemoteSettings.svelte'

  const ICONS: Record<Section, IconName> = {
    general: 'settings',
    git: 'branch',
    changes: 'file',
    remote: 'cloud',
    agents: 'agent',
    advanced: 'sliders',
    about: 'info',
    licenses: 'license',
  }
</script>

<Modal title={t('settings.title')} width={860} onclose={() => settings.close()}>
  <div class="layout">
    <nav aria-label={t('settings.title')}>
      {#each SECTIONS as section (section)}
        <button class:active={settings.section === section} aria-current={settings.section === section ? 'page' : undefined} onclick={() => (settings.section = section)}>
          <Icon name={ICONS[section]} size={15} />
          {t(`settings.sections.${section}`)}
        </button>
      {/each}
    </nav>
    <section class="content">
      <h3>{t(`settings.sections.${settings.section}`)}</h3>
      {#if settings.section === 'general'}
        <GeneralSettings />
      {:else if settings.section === 'git'}
        <GitSettings />
      {:else if settings.section === 'changes'}
        <ChangesSettings />
      {:else if settings.section === 'remote'}
        <RemoteSettings />
      {:else if settings.section === 'agents'}
        <AgentSettings />
      {:else if settings.section === 'advanced'}
        <AdvancedSettings />
      {:else if settings.section === 'about'}
        <AboutSettings />
      {:else}
        <LicenseSettings />
      {/if}
    </section>
  </div>
</Modal>

<style>
  .layout {
    display: flex;
    gap: 20px;
    height: min(560px, 68vh);
    margin: -4px 0;
  }

  nav {
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 2px;
    width: 190px;
    padding-right: 12px;
    border-right: 1px solid var(--border-muted);
  }

  nav button {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 32px;
    padding: 0 10px;
    border: none;
    border-radius: var(--radius);
    background: transparent;
    color: var(--fg-muted);
    text-align: left;
  }

  nav button:hover {
    background: var(--bg-hover);
    color: var(--fg);
  }

  nav button.active {
    background: var(--bg-selected);
    color: var(--fg);
    font-weight: 600;
  }

  .content {
    flex: 1;
    min-width: 0;
    overflow-y: auto;
    padding-right: 4px;
  }

  h3 {
    margin: 0 0 4px;
    font-size: 16px;
  }
</style>
