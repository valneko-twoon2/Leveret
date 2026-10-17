<!-- AI agent integration: access level, how to connect MCP clients, the tools and recent agent activity. -->
<script lang="ts">
  import type { AgentAccess, AgentLevel } from '../api'
  import { formatDateTime } from '../format'
  import { t, type MessageKey } from '../i18n/index.svelte'
  import { app } from '../state.svelte'
  import Icon from './Icon.svelte'
  import Modal from './Modal.svelte'

  type Client = 'claudeCode' | 'codex' | 'vscode' | 'claudeDesktop' | 'other'

  const info = $derived(app.agentInfo)
  let client = $state<Client>('claudeCode')

  const levels: { value: AgentAccess; label: MessageKey; hint: MessageKey }[] = [
    { value: 'off', label: 'agent.accessOff', hint: 'agent.accessOffHint' },
    { value: 'readOnly', label: 'agent.accessReadOnly', hint: 'agent.accessReadOnlyHint' },
    { value: 'standard', label: 'agent.accessStandard', hint: 'agent.accessStandardHint' },
    { value: 'full', label: 'agent.accessFull', hint: 'agent.accessFullHint' },
  ]

  const clients: { value: Client; label: string; intro: MessageKey }[] = [
    { value: 'claudeCode', label: 'Claude Code', intro: 'agent.claudeCode' },
    { value: 'codex', label: 'Codex', intro: 'agent.codex' },
    { value: 'vscode', label: 'GitHub Copilot (VS Code)', intro: 'agent.vscode' },
    { value: 'claudeDesktop', label: 'Claude Desktop', intro: 'agent.claudeDesktop' },
    { value: 'other', label: 'MCP', intro: 'agent.other' },
  ]

  const levelLabel: Record<AgentLevel, MessageKey> = {
    read: 'agent.levelRead',
    write: 'agent.levelWrite',
    destructive: 'agent.levelDestructive',
  }

  function allowed(level: AgentLevel): boolean {
    const access = info?.access ?? 'off'
    if (level === 'read') return access !== 'off'
    if (level === 'write') return access === 'standard' || access === 'full'
    return access === 'full'
  }

  /** Configuration text for the selected client (code, so not translated). */
  const snippet = $derived.by(() => {
    const exe = info?.exePath ?? 'Leveret.exe'
    switch (client) {
      case 'claudeCode':
        return `claude mcp add --scope user leveret -- "${exe}" --mcp`
      case 'codex':
        // TOML literal string: backslashes need no escaping.
        return `[mcp_servers.leveret]\ncommand = '${exe}'\nargs = ["--mcp"]`
      case 'vscode':
        return JSON.stringify({ servers: { leveret: { type: 'stdio', command: exe, args: ['--mcp'] } } }, null, 2)
      case 'claudeDesktop':
        return JSON.stringify({ mcpServers: { leveret: { command: exe, args: ['--mcp'] } } }, null, 2)
      case 'other':
        return `"${exe}" --mcp`
    }
  })

  const activity = $derived([...(info?.activity ?? [])].reverse())
  const repoName = (path: string | null) => (path ? (path.split(/[\\/]/).filter(Boolean).pop() ?? path) : '')
</script>

<Modal title={t('agent.title')} width={720} onclose={() => app.closeAgent()}>
  <p class="intro">{t('agent.intro')}</p>

  <h4>{t('agent.access')}</h4>
  <div class="levels" role="radiogroup" aria-label={t('agent.access')}>
    {#each levels as level (level.value)}
      <label class="level" class:selected={info?.access === level.value}>
        <input
          type="radio"
          name="agent-access"
          value={level.value}
          checked={info?.access === level.value}
          disabled={!info}
          onchange={() => app.setAgentAccess(level.value)}
        />
        <span class="level-text">
          <strong>{t(level.label)}</strong>
          <span>{t(level.hint)}</span>
        </span>
      </label>
    {/each}
  </div>

  <h4>{t('agent.connect')}</h4>
  <p class="muted">{t('agent.connectIntro')}</p>
  <div class="clients" role="tablist">
    {#each clients as c (c.value)}
      <button role="tab" class:active={client === c.value} aria-selected={client === c.value} onclick={() => (client = c.value)}>
        {c.label}
      </button>
    {/each}
  </div>
  <p class="muted small">{t(clients.find((c) => c.value === client)?.intro ?? 'agent.other')}</p>
  <div class="snippet">
    <pre class="mono">{snippet}</pre>
    <button class="icon-btn" title={t('common.copy')} aria-label={t('common.copy')} onclick={() => app.copy(snippet)}>
      <Icon name="copy" size={15} />
    </button>
  </div>
  <p class="muted small">{t('agent.exeNote')}</p>

  <h4>{t('agent.tools', { count: info?.tools.length ?? 0 })}</h4>
  <ul class="tools">
    {#each info?.tools ?? [] as tool (tool.name)}
      <li class:disabled={!allowed(tool.level)} title={tool.title}>
        <span class="mono">{tool.name}</span>
        <span class="badge {tool.level}">{t(levelLabel[tool.level])}</span>
      </li>
    {/each}
  </ul>

  <h4>{t('agent.activity')}</h4>
  {#if activity.length}
    <ul class="activity">
      {#each activity as a (a.time + a.tool)}
        <li class:failed={!a.ok}>
          <Icon name={a.ok ? 'check' : 'alert'} size={14} />
          <span class="when">{formatDateTime(Math.floor(a.time / 1000))}</span>
          <span class="who">{a.client}</span>
          <span class="mono what">{a.tool}</span>
          <span class="where">{repoName(a.repo)}</span>
          <span class="message" title={a.message}>{a.message}</span>
        </li>
      {/each}
    </ul>
  {:else}
    <p class="muted">{t('agent.noActivity')}</p>
  {/if}
</Modal>

<style>
  .intro {
    margin: 0 0 4px;
    color: var(--fg-muted);
  }

  h4 {
    margin: 18px 0 8px;
    font-size: 13px;
  }

  .muted {
    margin: 0 0 8px;
    color: var(--fg-muted);
  }

  .small {
    font-size: 12px;
  }

  .levels {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 8px;
  }

  .level {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 8px 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    cursor: pointer;
  }

  .level.selected {
    border-color: var(--accent);
    background: var(--bg-selected);
  }

  .level input {
    margin-top: 3px;
  }

  .level-text {
    display: flex;
    flex-direction: column;
    line-height: 1.4;
  }

  .level-text span {
    color: var(--fg-muted);
    font-size: 12px;
  }

  .clients {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin-bottom: 8px;
  }

  .clients button {
    height: 26px;
    padding: 0 10px;
    border: 1px solid var(--border);
    border-radius: 999px;
    background: transparent;
    font-size: 12px;
  }

  .clients button.active {
    border-color: var(--accent);
    background: var(--accent);
    color: var(--fg-on-accent);
  }

  .snippet {
    position: relative;
    margin-bottom: 6px;
  }

  pre {
    margin: 0;
    padding: 10px 40px 10px 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-subtle);
    font-size: 12px;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    user-select: text;
  }

  .snippet .icon-btn {
    position: absolute;
    top: 6px;
    right: 6px;
  }

  .tools {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: 4px 12px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .tools li {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 6px;
    font-size: 12px;
  }

  .tools li.disabled {
    opacity: 0.4;
  }

  .badge {
    flex: none;
    padding: 0 6px;
    border-radius: 999px;
    font-size: 10px;
    line-height: 16px;
    background: var(--bg-muted);
    color: var(--fg-muted);
  }

  .badge.write {
    background: color-mix(in srgb, var(--accent) 15%, transparent);
    color: var(--accent);
  }

  .badge.destructive {
    background: color-mix(in srgb, var(--danger) 15%, transparent);
    color: var(--danger);
  }

  .activity {
    margin: 0;
    padding: 0;
    list-style: none;
    font-size: 12px;
  }

  .activity li {
    display: grid;
    grid-template-columns: 16px 120px 130px 130px 90px minmax(0, 1fr);
    align-items: center;
    gap: 8px;
    height: 26px;
    color: var(--success);
  }

  .activity li.failed {
    color: var(--danger);
  }

  .activity li > span {
    color: var(--fg);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .activity .when,
  .activity .where,
  .activity .message {
    color: var(--fg-muted);
  }
</style>
