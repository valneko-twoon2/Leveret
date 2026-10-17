// Mock commands outside a repository: AI agents, perf samples, the GitHub
// account and cloning.
import type { AgentAccess, AgentInfo, DeviceCode, GithubState, PerfSnapshot, PollResult } from '../api'
import { AGENT_ACTIVITY, AGENT_TOOLS, GITHUB_REPOS, MOCK_ACCOUNT, mockPerfSamples } from './data'
import { repoCommands } from './repoCommands'
import { cloneListeners, db, delay, fail, UNHANDLED } from './state'

/** Handles one command of this group; `UNHANDLED` for any other command. */
export async function appCommands(cmd: string, args: Record<string, unknown>): Promise<unknown> {
  switch (cmd) {
    case 'agent_info':
      return {
        exePath: 'C:\\Users\\valneko-twoon2\\Apps\\Leveret\\Leveret.exe',
        access: db.agentAccess,
        tools: AGENT_TOOLS,
        activity: AGENT_ACTIVITY,
      } satisfies AgentInfo
    case 'set_agent_access':
      db.agentAccess = args.access as AgentAccess
      return null
    case 'agent_call':
      return { text: `mock: ${String(args.name)}`, isError: false }
    case 'perf_samples': {
      const after = Number(args.after)
      return { samples: mockPerfSamples().filter((s) => (s.seq ?? 0) > after), uptimeMs: performance.now() } satisfies PerfSnapshot
    }
    case 'perf_report':
      return null
    case 'github_state':
      return { account: db.account, deviceFlow: true } satisfies GithubState
    case 'github_start_sign_in':
      db.pollCount = 0
      return {
        deviceCode: 'mockdevicecode',
        userCode: 'WDJB-MJHT',
        verificationUri: 'https://github.com/login/device',
        expiresIn: 900,
        interval: 1,
      } satisfies DeviceCode
    case 'github_poll_sign_in':
      if (++db.pollCount < 3) return { status: 'pending' } satisfies PollResult
      db.account = MOCK_ACCOUNT
      return { status: 'success', account: db.account } satisfies PollResult
    case 'github_sign_in_with_token':
      if (!String(args.token).trim()) fail('invalidInput', 'token')
      db.account = MOCK_ACCOUNT
      return db.account
    case 'github_sign_out':
      db.account = null
      return null
    case 'github_list_repos':
      if (!db.account) fail('githubNotSignedIn')
      return GITHUB_REPOS
    case 'open_github_url':
      console.info(`mock: would open ${String(args.url)}`)
      return null
    case 'default_clone_dir':
      return 'C:\\Users\\valneko-twoon2\\Documents\\GitHub'
    case 'clone_repo': {
      const name = String(args.name)
      const path = `${String(args.parent).replace(/[\\/]+$/, '')}\\${name}`
      if (db.repos.some((r) => r.entry.path.toLowerCase() === path.toLowerCase())) fail('cloneTargetExists', path)
      const phases: [string, number][] = [
        ['Counting objects', 5],
        ['Receiving objects', 30],
        ['Receiving objects', 60],
        ['Receiving objects', 80],
        ['Resolving deltas', 95],
        ['Updating files', 100],
      ]
      for (const [phase, percent] of phases) {
        await delay(250)
        for (const l of cloneListeners) l({ phase, percent })
      }
      return repoCommands('add_repo', { path })
    }
    default:
      return UNHANDLED
  }
}
