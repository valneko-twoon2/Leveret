<!-- The current repository's branches, remotes, tags and stashes as a tree under the repository list.
     Click selects, double-click (or Enter) checks out, the right-click menu has the rest. -->
<script lang="ts">
  import { branches } from '../branches.svelte'
  import { formatRelative } from '../format'
  import { t, type MessageKey } from '../i18n/index.svelte'
  import { menu } from '../menu.svelte'
  import { refTree, type RefRow, type RefSection } from '../refTree.svelte'
  import { remotes } from '../remotes.svelte'
  import { stash } from '../stash.svelte'
  import { app } from '../state.svelte'
  import { tags } from '../tags.svelte'
  import Icon from './Icon.svelte'

  const rows = $derived(refTree.rows)
  let tree = $state<HTMLDivElement>()
  /** The row that takes Tab focus (the others are reached with the arrow keys). */
  let focusKey = $state<string | null>(null)
  const tabbable = $derived(rows.some((r) => r.key === focusKey) ? focusKey : (rows[0]?.key ?? null))

  const SECTION_ACTIONS: Partial<Record<RefSection, { icon: 'plus' | 'settings'; label: MessageKey; run: () => unknown }>> = {
    branches: { icon: 'plus', label: 'refTree.newBranch', run: () => branches.openNewBranch() },
    remotes: { icon: 'settings', label: 'refTree.manageRemotes', run: () => remotes.openDialog() },
    tags: { icon: 'plus', label: 'refTree.newTag', run: () => tags.openNewTag() },
  }

  function activate(row: RefRow) {
    refTree.selected = row.key
    if (app.busy !== null && row.kind !== 'section' && row.kind !== 'dir' && row.kind !== 'remote') return
    switch (row.kind) {
      case 'section':
      case 'dir':
      case 'remote':
        refTree.toggle(row)
        break
      case 'branch':
        if (!row.branch.current) void branches.switchBranch(row.branch)
        break
      case 'tag':
        void tags.checkout(row.tag)
        break
      case 'stash':
        void stash.show(row.stash)
        break
    }
  }

  function rowMenu(row: RefRow, e: MouseEvent | KeyboardEvent, anchor?: HTMLElement) {
    refTree.selected = row.key
    switch (row.kind) {
      case 'branch':
        menu.open(e, branches.menuItems(row.branch), anchor)
        break
      case 'tag':
        menu.open(e, tags.menuItems(row.tag), anchor)
        break
      case 'stash':
        menu.open(e, stash.menuItems(row.stash), anchor)
        break
      case 'remote':
        menu.open(
          e,
          [
            ...(row.remote ? [{ label: t('remotes.copyUrl'), action: () => app.copy(row.remote?.fetchUrl ?? '') }] : []),
            { label: t('refTree.manageRemotes'), action: () => remotes.openDialog() },
          ],
          anchor,
        )
        break
    }
  }

  function focusRow(index: number) {
    const row = rows[index]
    if (!row) return
    focusKey = row.key
    tree?.querySelector<HTMLElement>(`[data-key="${CSS.escape(row.key)}"]`)?.focus()
  }

  function onkeydown(e: KeyboardEvent, row: RefRow, index: number) {
    const expandable = row.kind === 'section' || row.kind === 'dir' || row.kind === 'remote'
    switch (e.key) {
      case 'ArrowDown':
        focusRow(Math.min(rows.length - 1, index + 1))
        break
      case 'ArrowUp':
        focusRow(Math.max(0, index - 1))
        break
      case 'Home':
        focusRow(0)
        break
      case 'End':
        focusRow(rows.length - 1)
        break
      case 'ArrowRight':
        if (expandable && !row.open) refTree.setOpen(row, true)
        else focusRow(Math.min(rows.length - 1, index + 1))
        break
      case 'ArrowLeft':
        if (expandable && row.open) refTree.setOpen(row, false)
        else {
          // To the parent: the nearest row above that is shallower.
          const parent = rows.findLastIndex((r, i) => i < index && r.depth < row.depth)
          if (parent >= 0) focusRow(parent)
        }
        break
      case 'Enter':
        activate(row)
        break
      case 'ContextMenu':
        rowMenu(row, e, e.currentTarget as HTMLElement)
        break
      case 'F10':
        if (!e.shiftKey) return
        rowMenu(row, e, e.currentTarget as HTMLElement)
        break
      default:
        return
    }
    e.preventDefault()
  }

  function title(row: RefRow): string {
    switch (row.kind) {
      case 'branch':
        return row.branch.name
      case 'tag':
        return row.tag.subject ? `${row.tag.name}\n${row.tag.subject}` : row.tag.name
      case 'stash':
        return `${row.stash.message}\n${t('refTree.openStash')}`
      case 'remote':
        return row.remote?.fetchUrl ?? row.label
      default:
        return row.label
    }
  }
</script>

<div class="tree" role="tree" aria-label={t('refTree.label')} bind:this={tree}>
  {#each rows as row, i (row.key)}
    {@const action = row.kind === 'section' ? SECTION_ACTIONS[row.section] : undefined}
    <div class="line" class:header={row.kind === 'section'}>
      <button
        class="row"
        class:selected={refTree.selected === row.key}
        class:current={row.kind === 'branch' && row.branch.current}
        role="treeitem"
        aria-level={row.depth + 1}
        aria-selected={refTree.selected === row.key}
        aria-expanded={row.kind === 'section' || row.kind === 'dir' || row.kind === 'remote' ? row.open : undefined}
        tabindex={row.key === tabbable ? 0 : -1}
        data-key={row.key}
        title={title(row)}
        style:padding-left="{6 + row.depth * 12}px"
        onfocus={() => (focusKey = row.key)}
        onclick={() => {
          refTree.selected = row.key
          if (row.kind === 'section' || row.kind === 'dir' || row.kind === 'remote') refTree.toggle(row)
        }}
        ondblclick={() => activate(row)}
        oncontextmenu={(e) => rowMenu(row, e)}
        onkeydown={(e) => onkeydown(e, row, i)}
      >
        {#if row.kind === 'section' || row.kind === 'dir' || row.kind === 'remote'}
          <span class="chevron" class:open={row.open}><Icon name="chevronDown" size={12} /></span>
        {:else}
          <span class="chevron"></span>
        {/if}
        {#if row.kind === 'section'}
          <span class="label strong">{row.label}</span>
          <span class="count">{row.count}</span>
        {:else}
          {#if row.kind === 'dir'}
            <Icon name="folder" size={13} />
          {:else if row.kind === 'remote'}
            <Icon name="cloud" size={13} />
          {:else if row.kind === 'tag'}
            <Icon name="tag" size={13} />
          {:else if row.kind === 'stash'}
            <Icon name="archive" size={13} />
          {:else}
            <Icon name={row.branch.current ? 'check' : 'branch'} size={13} />
          {/if}
          <span class="label">{row.label}</span>
          {#if row.kind === 'branch' && (row.branch.ahead || row.branch.behind)}
            <span class="counts">
              {#if row.branch.behind}<span><Icon name="arrowDown" size={11} />{row.branch.behind}</span>{/if}
              {#if row.branch.ahead}<span><Icon name="arrowUp" size={11} />{row.branch.ahead}</span>{/if}
            </span>
          {:else if row.kind === 'branch' && row.branch.upstreamGone}
            <span class="gone">{t('branches.gone')}</span>
          {:else if row.kind === 'stash'}
            <span class="time">{formatRelative(row.stash.time)}</span>
          {/if}
        {/if}
      </button>
      {#if action}
        <button class="add" title={t(action.label)} aria-label={t(action.label)} tabindex="-1" onclick={action.run}>
          <Icon name={action.icon} size={13} />
        </button>
      {/if}
    </div>
  {/each}
</div>

<style>
  .tree {
    padding: 4px 6px 8px;
  }

  .line {
    position: relative;
  }

  .line.header {
    margin-top: 4px;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 5px;
    width: 100%;
    height: 24px;
    padding-right: 6px;
    border: none;
    border-radius: var(--radius);
    background: transparent;
    color: var(--fg-muted);
    font-size: 12px;
    text-align: left;
  }

  .row:hover {
    background: var(--bg-hover);
    color: var(--fg);
  }

  .row.selected {
    background: var(--bg-selected);
    color: var(--fg);
  }

  .row.current {
    color: var(--fg);
    font-weight: 600;
  }

  .chevron {
    flex: none;
    display: inline-flex;
    width: 12px;
    transform: rotate(-90deg);
    transition: transform 0.1s;
  }

  .chevron.open {
    transform: none;
  }

  .label {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .strong {
    color: var(--fg);
    font-weight: 600;
  }

  .count {
    color: var(--fg-subtle);
    font-size: 11px;
  }

  .counts,
  .time,
  .gone {
    flex: none;
    color: var(--fg-subtle);
    font-size: 11px;
  }

  .counts {
    display: inline-flex;
    gap: 4px;
  }

  .counts span {
    display: inline-flex;
    align-items: center;
  }

  .gone {
    color: var(--attention);
  }

  .add {
    position: absolute;
    top: 2px;
    right: 4px;
    display: none;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    padding: 0;
    border: none;
    border-radius: 4px;
    background: transparent;
    color: var(--fg-subtle);
  }

  .add:hover {
    background: var(--bg-hover);
    color: var(--fg);
  }

  .line:hover .add,
  .line:has(:focus-visible) .add {
    display: inline-flex;
  }

  .line.header:hover .count,
  .line.header:has(:focus-visible) .count {
    visibility: hidden;
  }
</style>
