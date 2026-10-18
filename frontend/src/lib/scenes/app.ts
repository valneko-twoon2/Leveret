// Screenshot scenes of the main screens (01-23).
import { agents } from '../agents.svelte'
import { api } from '../api'
import { branches } from '../branches.svelte'
import { changes } from '../changes.svelte'
import { github } from '../github.svelte'
import { history } from '../history.svelte'
import { t } from '../i18n/index.svelte'
import { issues } from '../issues.svelte'
import { menu } from '../menu.svelte'
import { settings } from '../settings.svelte'
import { stash } from '../stash.svelte'
import { app } from '../state.svelte'
import { tags } from '../tags.svelte'
import { type Scene, settle, openRepo, closeOverlays, openContextMenuOnSelectedRow } from './helpers'

export const appScenes: Scene[] = [
] = [
  {
    id: '01-welcome',
    mockOnly: true,
    run: async () => {
      const mock = await import('../mock/index')
      mock.reset({ empty: true })
      await app.init()
      mock.reset()
    },
  },
  {
    id: '02-changes',
    run: async () => {
      await app.init()
      await openRepo('Leveret')
      await app.setTab('changes')
      const file = app.files.find((f) => f.kind === 'modified') ?? app.files[0]
      if (file) await changes.selectFile(file.path)
    },
  },
  {
    id: '03-commit-message',
    run: async () => {
      await app.setTab('changes')
      const untracked = app.files.find((f) => f.kind === 'untracked')
      if (untracked) {
        if (!changes.excluded.has(untracked.path)) changes.toggleFile(untracked.path)
        await changes.selectFile(untracked.path)
      }
      changes.summary = 'ブランチ作成ダイアログを追加'
      changes.description = 'ログ画面の右クリックメニューから新しいブランチを作成できるようにした。\n作成後にそのままチェックアウトすることもできる。'
    },
  },
  {
    id: '04-history',
    run: async () => {
      changes.summary = ''
      changes.description = ''
      await app.setTab('history')
      await history.setScope('head')
      const merge = history.log.find((c) => c.parents.length > 1)
      await history.selectCommit((merge ? history.log[history.log.indexOf(merge) + 1] : history.log[1])?.hash ?? null)
    },
  },
  {
    id: '05-history-all-branches',
    run: async () => {
      await app.setTab('history')
      await history.setScope('all')
      await history.selectCommit(history.log[0]?.hash ?? null)
    },
  },
  {
    id: '06-error',
    run: async () => {
      await app.setTab('changes')
      app.error = {
        message: t('errors.identityMissing'),
        code: 'identityMissing',
        detail: 'Author identity unknown\n\n*** Please tell me who you are.',
      }
    },
  },
  {
    id: '07-sign-in',
    mockOnly: true,
    run: async () => {
      app.error = null
      await github.load()
      github.openSignIn()
    },
  },
  {
    id: '08-clone',
    mockOnly: true,
    run: async () => {
      github.closeSignIn()
      const mock = await import('../mock/index')
      mock.reset({ signedIn: true })
      await github.load()
      await github.openClone()
      github.selectedRepo = github.repos.find((r) => r.private)?.fullName ?? null
    },
  },
  {
    id: '09-cloning',
    mockOnly: true,
    run: async () => {
      // Show the progress state without actually waiting for a clone.
      github.cloning = true
      github.progress = { phase: 'Receiving objects', percent: 62 }
    },
  },
  {
    id: '10-branches',
    run: async () => {
      closeOverlays()
      await openRepo('Leveret')
      await app.setTab('changes')
      await branches.openBranches()
    },
  },
  {
    id: '11-new-branch',
    run: async () => {
      closeOverlays()
      branches.openNewBranch()
      branches.newBranchName = 'feature/stash-ui'
    },
  },
  {
    id: '12-history-menu',
    run: async () => {
      closeOverlays()
      await openRepo('Leveret')
      await app.setTab('history')
      await history.setScope('head')
      await history.selectCommit(history.log[1]?.hash ?? null)
      await settle()
      openContextMenuOnSelectedRow()
    },
  },
  {
    id: '13-rebase-conflict',
    mockOnly: true,
    run: async () => {
      closeOverlays()
      await openRepo('Leveret')
      // Straight through the API: the confirmation dialog can't be answered off-screen.
      if (app.selectedPath && !app.operation) await api.rebaseBranch(app.selectedPath, 'refs/heads/feature/rebase')
      await app.setTab('changes')
      await app.refresh()
      const conflicted = app.files.find((f) => f.kind === 'conflicted')
      if (conflicted) await changes.selectFile(conflicted.path)
    },
  },
  {
    id: '14-pushing',
    mockOnly: true,
    run: async () => {
      closeOverlays()
      await openRepo('Leveret')
      if (app.selectedPath && app.operation) await api.abortOperation(app.selectedPath)
      await app.refresh()
      // Show the progress state without waiting for a real push.
      app.busy = 'push'
      app.remoteProgress = { repo: app.selectedPath ?? '', operation: 'push', phase: 'Writing objects', percent: 45 }
    },
  },
  {
    id: '15-agents',
    run: async () => {
      closeOverlays()
      app.busy = null
      app.remoteProgress = null
      await agents.openAgent()
    },
  },
  {
    id: '16-performance',
    run: async () => {
      closeOverlays()
      await openRepo('Leveret')
      await app.setTab('history')
      app.perfOpen = true
    },
  },
  {
    id: '17-issues',
    run: async () => {
      closeOverlays()
      await openRepo('Leveret')
      await app.setTab('issues')
      const issue = issues.list.find((i) => i.source === 'github' && i.comments > 1) ?? issues.list[0]
      if (issue) await issues.select(issue)
    },
  },
  {
    id: '18-new-issue',
    run: async () => {
      closeOverlays()
      await openRepo('Leveret')
      await app.setTab('issues')
      issues.openNew()
      if (issues.draft) {
        issues.draft.title = 'コミット前にフックの結果を表示する'
        issues.draft.body = 'pre-commit フックが失敗したとき、出力をそのままダイアログに出したい。\n\n- 失敗した行を強調する\n- もう一度実行するボタン'
      }
    },
  },
  {
    id: '19-tags',
    run: async () => {
      closeOverlays()
      await openRepo('Leveret')
      await app.setTab('history')
      await branches.openBranches('tags')
    },
  },
  {
    id: '20-new-tag',
    run: async () => {
      closeOverlays()
      await openRepo('Leveret')
      await app.setTab('history')
      const c = history.log[0]
      tags.openNewTag(c?.hash, c?.subject)
      tags.newTagName = 'v0.2.0'
      tags.newTagMessage = 'WinMerge 連携とタグに対応'
    },
  },
  {
    id: '21-settings',
    run: async () => {
      closeOverlays()
      await openRepo('Leveret')
      await settings.openSettings('general')
    },
  },
  {
    id: '22-settings-git',
    run: async () => {
      closeOverlays()
      await openRepo('Leveret')
      await settings.openSettings('git')
    },
  },
  {
    id: '23-licenses',
    run: async () => {
      closeOverlays()
      await openRepo('Leveret')
      await settings.openSettings('licenses')
      await new Promise((r) => setTimeout(r, 300))
    },
  },
]
