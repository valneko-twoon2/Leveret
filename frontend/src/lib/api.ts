// Typed wrappers around the backend commands, combined from one file per area
// (repoApi, historyApi, refApi, appApi; issues are in issueApi). The transport
// (and the mock routing) is in ipc.ts.
import { appApi } from './appApi'
import { historyApi } from './historyApi'
import { refApi } from './refApi'
import { repoApi } from './repoApi'

export * from './appApi'
export * from './historyApi'
export * from './ipc'
export * from './refApi'
export * from './repoApi'

export const api = { ...repoApi, ...historyApi, ...refApi, ...appApi }
