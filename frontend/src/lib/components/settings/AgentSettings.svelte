<!-- Settings: what AI agents may do, with a link to the full agents screen. -->
<script lang="ts">
  import { onMount } from 'svelte'
  import { agents } from '../../agents.svelte'
  import type { AgentAccess } from '../../api'
  import { t, type MessageKey } from '../../i18n/index.svelte'
  import Icon from '../Icon.svelte'
  import SettingRow from './SettingRow.svelte'

  const LEVELS: { value: AgentAccess; label: MessageKey; hint: MessageKey }[] = [
    { value: 'off', label: 'agent.accessOff', hint: 'agent.accessOffHint' },
    { value: 'readOnly', label: 'agent.accessReadOnly', hint: 'agent.accessReadOnlyHint' },
    { value: 'standard', label: 'agent.accessStandard', hint: 'agent.accessStandardHint' },
    { value: 'full', label: 'agent.accessFull', hint: 'agent.accessFullHint' },
  ]
  const access = $derived(agents.agentInfo?.access ?? null)

  onMount(() => void agents.loadAgentInfo())
</script>

<SettingRow label={t('settings.agents.access')} hint={t('settings.agents.accessHint')} stacked>
  <div class="levels" role="radiogroup" aria-label={t('settings.agents.access')}>
    {#each LEVELS as level (level.value)}
      <label class="level" class:selected={access === level.value}>
        <input type="radio" name="settings-agent-access" checked={access === level.value} onchange={() => agents.setAgentAccess(level.value)} />
        <span>
          <strong>{t(level.label)}</strong>
          <small>{t(level.hint)}</small>
        </span>
      </label>
    {/each}
  </div>
</SettingRow>

<SettingRow label={t('settings.agents.details')}>
  <button class="btn" onclick={() => agents.openAgent()}><Icon name="agent" size={14} /> {t('settings.agents.openDetails')}</button>
</SettingRow>

<style>
  .levels {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }

  .level {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    cursor: pointer;
  }

  .level.selected {
    border-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 8%, transparent);
  }

  .level span {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .level small {
    color: var(--fg-muted);
    font-size: 12px;
    line-height: 1.45;
  }

  .btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
</style>
