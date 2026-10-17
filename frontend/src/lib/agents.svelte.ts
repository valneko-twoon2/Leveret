// The AI agents dialog: connection info, the access level and what agents
// did (activity reported by `leveret --mcp`). Builds on `app` (state.svelte.ts).
import { api, type AgentAccess, type AgentActivity, type AgentInfo } from './api'
import { t } from './i18n/index.svelte'
import { perf } from './perf'
import { app } from './state.svelte'

class AgentsState {
  agentOpen = $state(false)
  agentInfo = $state.raw<AgentInfo | null>(null)

  async openAgent() {
    this.agentOpen = true
    await perf.view('agent.open', () => this.loadAgentInfo())
  }

  closeAgent() {
    this.agentOpen = false
  }

  async loadAgentInfo() {
    const info = await app.guard(() => api.agentInfo())
    if (info) this.agentInfo = info
  }

  async setAgentAccess(access: AgentAccess) {
    if ((await app.guard(() => api.setAgentAccess(access))) === undefined) return
    const key = { off: 'agent.accessOff', readOnly: 'agent.accessReadOnly', standard: 'agent.accessStandard', full: 'agent.accessFull' } as const
    app.toast(t('agent.accessSaved', { level: t(key[access]) }))
    await this.loadAgentInfo()
  }

  /**
   * An agent changed something (usually through `leveret --mcp`): tell the user and show the result.
   * Subscribed by `app.init`.
   */
  async onAgentActivity(entries: AgentActivity[]) {
    for (const a of entries.slice(-3)) {
      const params = { client: a.client, tool: a.tool }
      app.toast(a.ok ? t('agent.didTool', params) : t('agent.toolFailed', params), a.ok ? 'info' : 'error')
    }
    if (this.agentOpen) void this.loadAgentInfo()
    // Agents may have registered repositories or changed the current one.
    if (app.busy === null) {
      await app.reloadRepos()
      await app.refreshAfterChange()
    }
  }
}

export const agents = new AgentsState()
