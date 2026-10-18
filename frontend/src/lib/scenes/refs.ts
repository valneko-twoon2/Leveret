// Screenshot scenes of the sidebar tree and the remote / branch dialogs (32-).
import { branches } from '../branches.svelte'
import { history } from '../history.svelte'
import { refTree } from '../refTree.svelte'
import { remotes } from '../remotes.svelte'
import { app } from '../state.svelte'
import { type Scene, closeOverlays, openContextMenuOnSelectedRow, openRepo, settle } from './helpers'

export const refScenes: Scene[] = [
  {
    id: '32-ref-tree',
    run: async () => {
      closeOverlays()
      await openRepo('Leveret')
      await app.setTab('history')
      for (const key of ['section:remotes', 'remote:origin', 'section:tags']) refTree.openKey(key, true)
      refTree.selected = 'branch:refs/heads/feature/branch-dialog'
      await settle()
      openContextMenuOnSelectedRow()
    },
  },
  {
    id: '33-remotes',
    run: async () => {
      closeOverlays()
      await openRepo('Leveret')
      await remotes.openDialog()
      remotes.startAdd()
      if (remotes.draft) Object.assign(remotes.draft, { name: 'upstream', url: 'https://github.com/example/leveret.git' })
    },
  },
  {
    id: '34-upstream',
    run: async () => {
      closeOverlays()
      await openRepo('Leveret')
      await history.closeComparison()
      await branches.loadBranches()
      const b = branches.branches.find((x) => x.name === 'fix/sjis-diff')
      if (b) await branches.openUpstream(b)
    },
  },
]
