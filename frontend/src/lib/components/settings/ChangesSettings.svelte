<!-- Settings: the changes list and diffs. -->
<script lang="ts">
  import { t } from '../../i18n/index.svelte'
  import { CONTEXT_LINES, prefs, TAB_SIZES } from '../../prefs.svelte'
  import { settings } from '../../settings.svelte'
  import SettingRow from './SettingRow.svelte'
</script>

<SettingRow label={t('settings.changes.showUntracked')} hint={t('settings.changes.showUntrackedHint')}>
  <input type="checkbox" class="switch" checked={prefs.showUntracked} onchange={(e) => settings.setShowUntracked(e.currentTarget.checked)} aria-label={t('settings.changes.showUntracked')} />
</SettingRow>

<SettingRow label={t('settings.changes.ignoreWhitespace')} hint={t('settings.changes.ignoreWhitespaceHint')}>
  <input
    type="checkbox"
    class="switch"
    checked={prefs.ignoreWhitespace}
    onchange={(e) => settings.setIgnoreWhitespace(e.currentTarget.checked)}
    aria-label={t('settings.changes.ignoreWhitespace')}
  />
</SettingRow>

<SettingRow label={t('settings.changes.contextLines')} hint={t('settings.changes.contextLinesHint')}>
  <select value={prefs.contextLines} onchange={(e) => settings.setContextLines(Number(e.currentTarget.value))} aria-label={t('settings.changes.contextLines')}>
    {#each CONTEXT_LINES as lines (lines)}
      <option value={lines}>{t('settings.changes.contextLinesValue', { lines })}</option>
    {/each}
  </select>
</SettingRow>

<SettingRow label={t('settings.changes.tabSize')}>
  <select value={prefs.tabSize} onchange={(e) => prefs.set('tabSize', Number(e.currentTarget.value))} aria-label={t('settings.changes.tabSize')}>
    {#each TAB_SIZES as size (size)}
      <option value={size}>{t('settings.changes.tabSizeValue', { size })}</option>
    {/each}
  </select>
</SettingRow>

<SettingRow
  label={t('settings.changes.diffTool')}
  hint={settings.info?.diffTool ? t('settings.changes.diffToolFound') : t('settings.changes.diffToolMissing')}
>
  <span class="status" class:ok={settings.info?.diffTool}>{settings.info?.diffTool ? 'WinMerge' : '—'}</span>
</SettingRow>

<style>
  .status {
    color: var(--fg-muted);
  }

  .status.ok {
    color: var(--success);
    font-weight: 600;
  }
</style>
