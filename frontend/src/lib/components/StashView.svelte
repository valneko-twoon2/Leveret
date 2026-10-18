<!-- Main area of the Changes tab while a stash is open: its message, files and diffs, and what to do with it. -->
<script lang="ts">
  import type { CommitFile, StashDetail } from '../api'
  import { formatDateTime, formatRelative, shortHash } from '../format'
  import { t } from '../i18n/index.svelte'
  import { commitImages } from '../imageSides'
  import { layout } from '../layout.svelte'
  import { stash } from '../stash.svelte'
  import { app } from '../state.svelte'
  import DiffView from './DiffView.svelte'
  import FileRow from './FileRow.svelte'
  import Icon from './Icon.svelte'
  import Splitter from './Splitter.svelte'
  import VirtualList from './VirtualList.svelte'

  const entry = $derived(stash.selected)
  const detail = $derived(stash.detail)
  const files = $derived(detail?.files ?? [])
  const selectedIndex = $derived(files.findIndex((f) => f.path === stash.file))
  const idle = $derived(app.busy === null)
  const selectedFile = $derived(files.find((f) => f.path === stash.file) ?? null)

  /** Untracked files are in the stash's third parent and have no earlier version. */
  const stashImages = (f: CommitFile, d: StashDetail) =>
    f.kind === 'untracked' ? commitImages(f, null, d.untracked ?? d.oid) : commitImages(f, d.base, d.oid)
</script>

{#if entry}
  <div class="stash-view">
    <header>
      <div class="title">
        <Icon name="archive" size={16} />
        <h2 title={entry.message}>{entry.message}</h2>
        <button class="icon-btn" title={t('stash.close')} aria-label={t('stash.close')} onclick={() => stash.close()}>
          <Icon name="x" size={15} />
        </button>
      </div>
      <div class="meta">
        <span title={formatDateTime(entry.time)}><Icon name="clock" size={14} /> {formatRelative(entry.time)}</span>
        {#if detail}
          <span>{t('stash.base')}: <span class="mono">{shortHash(detail.base)}</span></span>
          {#if detail.untracked}<span>{t('stash.untracked')}</span>{/if}
        {/if}
        <span class="actions">
          <button class="btn" disabled={!idle} title={t('stash.applyHint')} onclick={() => stash.restore(entry, true)}>{t('stash.apply')}</button>
          <button class="btn primary" disabled={!idle} title={t('stash.popHint')} onclick={() => stash.restore(entry, false)}>{t('stash.pop')}</button>
          <button class="btn danger" disabled={!idle} onclick={() => stash.drop(entry)}>{t('stash.drop')}</button>
        </span>
      </div>
    </header>

    <div class="split">
      <div class="files" style:width="{layout.sizes.commitFiles}px">
        <div class="files-header">{t('stash.files', { count: files.length })}</div>
        <div class="files-list">
          {#if !detail}
            <div class="empty">{t('common.loading')}</div>
          {:else}
            <VirtualList
              items={files}
              rowHeight={30}
              {selectedIndex}
              label={t('stash.files', { count: files.length })}
              onselect={(i) => files[i] && stash.selectFile(files[i].path)}
            >
              {#snippet row(f)}
                <FileRow path={f.path} origPath={f.origPath} kind={f.kind} />
              {/snippet}
            </VirtualList>
          {/if}
        </div>
      </div>
      <Splitter pane="commitFiles" />
      <div class="diff"><DiffView diff={stash.diff} images={selectedFile && detail ? stashImages(selectedFile, detail) : undefined} /></div>
    </div>
  </div>
{/if}

<style>
  .stash-view {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
  }

  header {
    padding: 12px 16px;
    border-bottom: 1px solid var(--border);
  }

  .title {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--fg-muted);
  }

  h2 {
    flex: 1;
    min-width: 0;
    margin: 0;
    overflow: hidden;
    color: var(--fg);
    font-size: 16px;
    font-weight: 600;
    text-overflow: ellipsis;
    white-space: nowrap;
    user-select: text;
  }

  .meta {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px 16px;
    margin-top: 10px;
    color: var(--fg-muted);
    font-size: 12px;
  }

  .meta > span {
    display: inline-flex;
    align-items: center;
    gap: 5px;
  }

  .actions {
    margin-left: auto;
  }

  .split {
    flex: 1;
    display: flex;
    min-height: 0;
  }

  .files {
    display: flex;
    flex-direction: column;
    flex: none;
    min-width: 0;
  }

  .files-header {
    display: flex;
    align-items: center;
    height: 34px;
    padding: 0 12px;
    border-bottom: 1px solid var(--border-muted);
    color: var(--fg-muted);
    font-size: 12px;
  }

  .files-list {
    flex: 1;
    min-height: 0;
  }

  .diff {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
</style>
