<!-- Left column of the Changes tab: changed files with checkboxes, stashes, and the commit form. -->
<script lang="ts">
  import type { FileChange } from '../api'
  import { changes } from '../changes.svelte'
  import { diffTool } from '../difftool.svelte'
  import { formatRelative } from '../format'
  import { t } from '../i18n/index.svelte'
  import { menu, type MenuItem } from '../menu.svelte'
  import { prefs } from '../prefs.svelte'
  import { app } from '../state.svelte'
  import CommitForm from './CommitForm.svelte'
  import FileRow from './FileRow.svelte'
  import Icon from './Icon.svelte'
  import VirtualList from './VirtualList.svelte'

  const all = $derived(app.files.length > 0 && changes.includedFiles.length === app.files.length)
  const some = $derived(changes.includedFiles.length > 0 && !all)
  const selectedIndex = $derived(app.files.findIndex((f) => f.path === changes.selectedFile))
  const untrackedCount = $derived(app.files.filter((f) => f.kind === 'untracked').length)
  let stashOpen = $state(false)

  function extension(path: string): string | null {
    const name = path.split('/').pop() ?? path
    const dot = name.lastIndexOf('.')
    return dot > 0 && dot < name.length - 1 ? name.slice(dot + 1) : null
  }

  function fileMenu(file: FileChange, e: MouseEvent | KeyboardEvent) {
    const idle = app.busy === null
    const checked = changes.includedFiles.filter((f) => f.kind !== 'conflicted').map((f) => f.path)
    const ext = extension(file.path)
    const items: MenuItem[] = []
    if (diffTool.available) {
      const merge = file.kind === 'conflicted'
      items.push({ label: t(merge ? 'diff.mergeInTool' : 'diff.openInTool'), action: () => diffTool.openWorking(file) }, 'separator')
    }
    if (file.kind === 'conflicted') {
      items.push(
        { label: t('discard.markResolved'), action: () => changes.markResolved([file.path]), disabled: !idle },
        { label: t('discard.useOurs'), action: () => changes.takeSide([file.path], 'ours'), disabled: !idle, danger: true },
        { label: t('discard.useTheirs'), action: () => changes.takeSide([file.path], 'theirs'), disabled: !idle, danger: true },
      )
    } else {
      items.push({ label: t('discard.one'), action: () => changes.discard([file.path]), disabled: !idle, danger: true })
      if (checked.length > 1) {
        items.push({ label: t('discard.checked', { count: checked.length }), action: () => changes.discard(checked), disabled: !idle, danger: true })
      }
    }
    if (file.kind === 'untracked') {
      items.push('separator', { label: t('discard.ignore'), action: () => changes.ignore(file.path, false), disabled: !idle })
      if (ext) items.push({ label: t('discard.ignoreExtension', { ext }), action: () => changes.ignore(file.path, true), disabled: !idle })
    }
    items.push(
      'separator',
      { label: t('common.copyPath'), action: () => app.copy(file.path) },
      { label: t('common.showInExplorer'), action: () => app.showInExplorer(file.kind === 'deleted' ? undefined : file.path) },
    )
    menu.open(e, items)
  }

  function toggleUntracked() {
    prefs.setShowUntracked(!prefs.showUntracked)
    void app.loadStatus()
  }

  function listMenu(e: MouseEvent) {
    const idle = app.busy === null
    const discardable = app.files.filter((f) => f.kind !== 'conflicted').map((f) => f.path)
    menu.open(
      e,
      [
        { label: t('discard.all'), action: () => changes.discard(discardable), disabled: !idle || discardable.length === 0, danger: true },
        { label: t('stash.save'), action: () => changes.stashSave(), disabled: !idle || app.files.length === 0 },
        'separator',
        { label: t('changes.showUntracked'), checked: prefs.showUntracked, action: () => toggleUntracked() },
      ],
      e.currentTarget as HTMLElement,
    )
  }
</script>

<div class="pane">
  <div class="list-header">
    <input
      type="checkbox"
      checked={all}
      indeterminate={some}
      disabled={app.files.length === 0}
      onchange={() => changes.toggleAll()}
      title={t('changes.selectAll')}
      aria-label={t('changes.selectAll')}
    />
    <span class="count">
      {t('changes.count', { count: app.files.length })}
      {#if !prefs.showUntracked}<span class="sub">{t('changes.untrackedHidden')}</span>
      {:else if untrackedCount}<span class="sub">{t('changes.untrackedCount', { count: untrackedCount })}</span>{/if}
    </span>
    <button class="icon-btn" title={t('common.more')} aria-label={t('common.more')} onclick={listMenu}>
      <Icon name="more" size={16} />
    </button>
  </div>

  <div class="list">
    {#if app.status && app.files.length === 0}
      <div class="empty">
        <strong>{t('changes.none')}</strong>
        <span>{t('changes.noneBody')}</span>
      </div>
    {:else}
      <VirtualList
        items={app.files}
        rowHeight={30}
        {selectedIndex}
        label={t('tabs.changes')}
        onselect={(i) => app.files[i] && changes.selectFile(app.files[i].path)}
        oncontext={(i, e) => app.files[i] && fileMenu(app.files[i], e)}
        onactivate={(i) => app.files[i] && diffTool.openWorking(app.files[i])}
        ontoggle={(i) => app.files[i] && changes.toggleFile(app.files[i].path)}
      >
        {#snippet row(file)}
          <FileRow
            path={file.path}
            origPath={file.origPath}
            kind={file.kind}
            checked={!changes.excluded.has(file.path)}
            ontoggle={() => changes.toggleFile(file.path)}
          />
        {/snippet}
      </VirtualList>
    {/if}
  </div>

  {#if changes.stashes.length}
    <div class="stashes" class:open={stashOpen}>
      <button class="stash-header" onclick={() => (stashOpen = !stashOpen)} aria-expanded={stashOpen}>
        <Icon name="archive" size={14} />
        <span>{t('stash.count', { count: changes.stashes.length })}</span>
        <Icon name="chevronDown" size={14} />
      </button>
      {#if stashOpen}
        <ul>
          {#each changes.stashes as s (s.index)}
            <li>
              <div class="stash-text">
                <span class="stash-message" title={s.message}>{s.message}</span>
                <span class="stash-time">{formatRelative(s.time)}</span>
              </div>
              <button class="btn small" disabled={app.busy !== null} onclick={() => changes.stashPop(s.index)}>{t('stash.pop')}</button>
              <button
                class="icon-btn"
                disabled={app.busy !== null}
                title={t('stash.drop')}
                aria-label={t('stash.drop')}
                onclick={() => changes.stashDrop(s)}
              >
                <Icon name="x" size={14} />
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </div>
  {/if}

  <CommitForm />
</div>

<style>
  .pane {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
  }

  .list-header {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 34px;
    padding: 0 6px 0 12px;
    border-bottom: 1px solid var(--border-muted);
    color: var(--fg-muted);
    font-size: 12px;
  }

  .count {
    flex: 1;
  }

  .count .sub {
    color: var(--fg-subtle);
    font-size: 12px;
  }

  .list {
    flex: 1;
    /* Keep a few files visible in short windows (the stash list and form shrink first). */
    min-height: 96px;
  }

  .stashes {
    flex: none;
    border-top: 1px solid var(--border);
  }

  .stash-header {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    height: 30px;
    padding: 0 12px;
    border: none;
    background: var(--bg-subtle);
    color: var(--fg-muted);
    font-size: 12px;
    text-align: left;
  }

  .stash-header span {
    flex: 1;
  }

  .stashes.open .stash-header :global(svg:last-child) {
    transform: rotate(180deg);
  }

  ul {
    max-height: min(160px, 18vh);
    margin: 0;
    padding: 4px 0;
    overflow-y: auto;
    list-style: none;
  }

  li {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 2px 6px 2px 12px;
  }

  .stash-text {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    line-height: 1.3;
  }

  .stash-message {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .stash-time {
    color: var(--fg-subtle);
    font-size: 11px;
  }

  .btn.small {
    height: 24px;
    padding: 0 10px;
    font-size: 12px;
  }
</style>
