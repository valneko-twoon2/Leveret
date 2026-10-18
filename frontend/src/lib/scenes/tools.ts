// Screenshot scenes of the history and change tools (24 onwards): picked lines,
// stashes, search, file history, annotate, compare, reset, rebase, refs, remotes.
import { api } from '../api'
import { blame } from '../blame.svelte'
import { changes } from '../changes.svelte'
import { history } from '../history.svelte'
import { rewrite } from '../rewrite.svelte'
import { stash } from '../stash.svelte'
import { app } from '../state.svelte'
import { type Scene, openRepo, closeOverlays } from './helpers'

export const toolScenes: Scene[] = [
  {
    id: '24-partial-commit',
    run: async () => {
      closeOverlays()
      await openRepo('Leveret')
      await app.setTab('changes')
      const file = app.files.find((f) => f.kind === 'modified') ?? app.files[0]
      if (!file) return
      await changes.selectFile(file.path)
      // Leave the new doc comment and the else branch out of the commit.
      changes.picker.setLines([[0, 2], [0, 8], [0, 9], [0, 10]], false)
    },
  },
  {
    id: '25-stash',
    run: async () => {
      closeOverlays()
      await openRepo('Leveret')
      await app.setTab('changes')
      changes.picks.clear()
      if (stash.stashes[0]) await stash.open(stash.stashes[0])
      await stash.selectFile('src/ui/diff.css')
    },
  },
  {
    id: '26-history-search',
    run: async () => {
      closeOverlays()
      await openRepo('Leveret')
      await app.setTab('history')
      await history.setSearchField('message')
      await history.setFilter('差分', true)
    },
  },
  {
    id: '27-file-history',
    run: async () => {
      closeOverlays()
      await openRepo('Leveret')
      history.logFilter = ''
      await history.showFileHistory('src/main.rs')
    },
  },
  {
    id: '28-blame',
    run: async () => {
      closeOverlays()
      await openRepo('Leveret')
      await app.setTab('changes')
      await blame.open('src/git/branch.rs')
    },
  },
  {
    id: '29-compare',
    run: async () => {
      closeOverlays()
      await openRepo('Leveret')
      await app.setTab('history')
      history.logFilter = ''
      history.file = null
      await history.setScope('head')
      const [newer, older] = [history.log[0], history.log[4]]
      if (newer && older) await history.compareCommits(newer.hash, older.hash)
    },
  },
  {
    id: '30-reset',
    run: async () => {
      closeOverlays()
      await openRepo('Leveret')
      await app.setTab('history')
      await history.closeComparison()
      const target = history.log[3]
      if (target) await rewrite.openReset(target)
    },
  },
  {
    id: '31-rebase',
    // Makes commits first (the sample history has a merge near the top).
    mockOnly: true,
    run: async () => {
      closeOverlays()
      await openRepo('Leveret')
      const repo = app.selectedPath
      if (!repo) return
      const subjects = ['ブランチ一覧に並べ替えを追加', '並べ替えの設定を保存', 'typo を修正', 'テストを追加']
      for (const [i, summary] of subjects.entries()) {
        const file = app.files[i % app.files.length]
        if (file) await api.commit(repo, { summary, description: '', paths: [file.path] })
      }
      await app.setTab('history')
      await app.refreshAfterChange()
      const from = history.log[subjects.length - 1]
      if (!from) return
      await rewrite.openRebase(from)
      // Newest first: move "typo" right above the change it fixes, squash it, reword the newest.
      rewrite.moveRow(1, 1)
      rewrite.setAction(2, 'fixup')
      rewrite.setAction(0, 'reword')
      rewrite.setMessage(0, 'ブランチ一覧のテストを追加\n\n並べ替えの設定が保存されることも確認する。')
    },
  },
]
