<!-- Left column of the Changes tab: changed files with checkboxes, stashes, and the commit form. -->
<script lang="ts">
  import type { FileChange } from '../api'
  import { formatRelative } from '../format'
  import { t } from '../i18n/index.svelte'
  import { menu, type MenuItem } from '../menu.svelte'
  import { app } from '../state.svelte'
  import CommitForm from './CommitForm.svelte'
  import FileRow from './FileRow.svelte'
  import Icon from './Icon.svelte'
  import VirtualList from './VirtualList.svelte'

  const all = $derived(app.files.length > 0 && app.includedFiles.length === app.files.length)
  const some = $derived(app.includedFiles.length > 0 && !all)
  const selectedIndex = $derived(app.files.findIndex((f) => f.path === app.selectedFile))
  let stashOpen = $state(false)

  function extension(path: string): string | null {
    const name = path.split('/').pop() ?? path
    const dot = name.lastIndexOf('.')
    return dot > 0 && dot < name.length - 1 ? name.slice(dot + 1) : null
  }

  function fileMenu(file: FileChange, e: MouseEvent | KeyboardEvent) {
    const idle = app.busy === null
    const checked = app.includedFiles.filter((f) => f.kind !== 'conflicted').map((f) => f.path)
    const ext = extension(file.path)
    const items: MenuItem[] = []
    if (file.kind === 'conflicted') {
      items.push({ label: t('discard.markResolved'), action: () => app.markResolved([file.path]), disabled: !idle })
    } else {
      items.push({ label: t('discard.one'), action: () => app.discard([file.path]), disabled: !idle, danger: true })
      if (checked.length > 1) {
        items.push({ label: t('discard.checked', { count: checked.length }), action: () => app.discard(checked), disabled: !idle, danger: true })
      }
    }
    if (file.kind === 'untracked') {
      items.push('separator', { label: t('discard.ignore'), action: () => app.ignore(file.path, false), disabled: !idle })
      if (ext) items.push({ label: t('discard.ignoreExtension', { ext }), action: () => app.ignore(file.path, true), disabled: !idle })
    }
    items.push(
      'separator',
      { label: t('common.copyPath'), action: () => app.copy(file.path) },
      { label: t('common.showInExplorer'), action: () => app.showInExplorer(file.kind === 'deleted' ? undefined : file.path) },
    )
    menu.open(e, items)
  }

  function listMenu(e: MouseEvent) {
    const idle = app.busy === null
    const discardable = app.files.filter((f) => f.kind !== 'conflicted').map((f) => f.path)
    menu.open(
      e,
      [
        { label: t('discard.all'), action: () => app.discard(discardable), disabled: !idle || discardable.length === 0, danger: true },
        { label: t('stash.save'), action: () => app.stashSave(), disabled: !idle || app.files.length === 0 },
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
      onchange={() => app.toggleAll()}
      title={t('changes.selectAll')}
      aria-label={t('changes.selectAll')}
    />
    <span class="count">{t('changes.count', { count: app.files.length })}</span>
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
        onselect={(i) => app.files[i] && app.selectFile(app.files[i].path)}
        oncontext={(i, e) => app.files[i] && fileMenu(app.files[i], e)}
      >
        {#snippet row(file)}
          <FileRow
            path={file.path}
            origPath={file.origPath}
            kind={file.kind}
            checked={!app.excluded.has(file.path)}
            ontoggle={() => app.toggleFile(file.path)}
          />
        {/snippet}
      </VirtualList>
    {/if}
  </div>

  {#if app.stashes.length}
    <div class="stashes" class:open={stashOpen}>
      <button class="stash-header" onclick={() => (stashOpen = !stashOpen)} aria-expanded={stashOpen}>
        <Icon name="archive" size={14} />
        <span>{t('stash.count', { count: app.stashes.length })}</span>
        <Icon name="chevronDown" size={14} />
      </button>
      {#if stashOpen}
        <ul>
          {#each app.stashes as s (s.index)}
            <li>
              <div class="stash-text">
                <span class="stash-message" title={s.message}>{s.message}</span>
                <span class="stash-time">{formatRelative(s.time)}</span>
              </div>
              <button class="btn small" disabled={app.busy !== null} onclick={() => app.stashPop(s.index)}>{t('stash.pop')}</button>
              <button
                class="icon-btn"
                disabled={app.busy !== null}
                title={t('stash.drop')}
                aria-label={t('stash.drop')}
                onclick={() => app.stashDrop(s)}
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

  .list {
    flex: 1;
    min-height: 0;
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
    max-height: 160px;
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
