<!-- Settings: theme and language. -->
<script lang="ts">
  import { i18n, locales, t, type Locale } from '../../i18n/index.svelte'
  import { prefs, type Theme } from '../../prefs.svelte'
  import SettingRow from './SettingRow.svelte'

  const THEMES: { value: Theme; label: 'settings.themeSystem' | 'settings.themeLight' | 'settings.themeDark' }[] = [
    { value: 'system', label: 'settings.themeSystem' },
    { value: 'light', label: 'settings.themeLight' },
    { value: 'dark', label: 'settings.themeDark' },
  ]
  const LANGUAGE_NAMES: Record<string, string> = { ja: '日本語' }
</script>

<SettingRow label={t('settings.theme')} hint={t('settings.themeHint')}>
  <div class="segmented" role="radiogroup" aria-label={t('settings.theme')}>
    {#each THEMES as theme (theme.value)}
      <button role="radio" aria-checked={prefs.theme === theme.value} class:active={prefs.theme === theme.value} onclick={() => prefs.set('theme', theme.value)}>
        {t(theme.label)}
      </button>
    {/each}
  </div>
</SettingRow>

<SettingRow label={t('settings.language')} hint={t('settings.languageHint')}>
  <select
    value={i18n.locale}
    disabled={locales.length < 2}
    onchange={(e) => (i18n.locale = e.currentTarget.value as Locale)}
    aria-label={t('settings.language')}
  >
    {#each locales as locale (locale)}
      <option value={locale}>{LANGUAGE_NAMES[locale] ?? locale}</option>
    {/each}
  </select>
</SettingRow>

<style>
  .segmented {
    display: inline-flex;
    padding: 2px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-subtle);
  }

  .segmented button {
    height: 26px;
    padding: 0 10px;
    border: none;
    border-radius: calc(var(--radius) - 2px);
    background: transparent;
    color: var(--fg-muted);
    white-space: nowrap;
  }

  .segmented button.active {
    background: var(--bg);
    color: var(--fg);
    font-weight: 600;
    box-shadow: 0 1px 2px rgb(0 0 0 / 0.12);
  }
</style>
