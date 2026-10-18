// Screenshot scenes of the history and change tools (24 onwards): picked lines,
// stashes, search, file history, annotate, compare, reset, rebase, refs, remotes.
import { blame } from '../blame.svelte'
import { changes } from '../changes.svelte'
import { history } from '../history.svelte'
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
]
