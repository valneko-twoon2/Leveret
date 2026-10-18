<!-- Main area of the History tab while two commits are compared (Ctrl+click): the files that differ and their diffs. -->
<script lang="ts">
  import { diffTool } from '../difftool.svelte'
  import { formatRelative, shortHash } from '../format'
  import { history } from '../history.svelte'
  import { t } from '../i18n/index.svelte'
  import { layout } from '../layout.svelte'
  import type { LogEntry } from '../api'
  import DiffHeader from './DiffHeader.svelte'
  import DiffView from './DiffView.svelte'
  import FileRow from './FileRow.svelte'
  import Icon from './Icon.svelte'
  import Splitter from './Splitter.svelte'
  import VirtualList from './VirtualList.svelte'

  const c = $derived(history.compare)
  const files = $derived(c?.files ?? [])
  const selectedIndex = $derived(files.findIndex((f) => f.path === c?.file))
  const selectedFile = $derived(files[selectedIndex] ?? null)
</script>

{#snippet side(entry: LogEntry)}
  <span class="side" title={`${entry.subject}\n${entry.authorName} · ${formatRelative(entry.authorTime)}`}>
    <span class="mono">{shortHash(entry.hash)}</span>
    <span class="subject">{entry.subject}</span>
  </span>
{/snippet}

{#if c}
  <div class="compare-view">
    <header>
      <div class="title">
        <h2>{t('history.compareTitle')}</h2>
        <button class="icon-btn" title={t('history.compareClose')} aria-label={t('history.compareClose')} onclick={() => history.closeComparison()}>
          <Icon name="x" size={15} />
        </button>
      </div>
      <div class="range">
        {@render side(c.base)}
        <button class="icon-btn swap" title={t('history.compareSwap')} aria-label={t('history.compareSwap')} onclick={() => history.swapComparison()}>
          <Icon name="sync" size={14} />
        </button>
        {@render side(c.target)}
      </div>
    </header>

    <div class="split">
      <div class="files" style:width="{layout.sizes.commitFiles}px">
        <div class="files-header">{t('history.compareFiles', { count: files.length })}</div>
        <div class="files-list">
          {#if !c.files}
            <div class="empty">{t('common.loading')}</div>
          {:else if files.length === 0}
            <div class="empty">{t('history.compareNone')}</div>
          {:else}
            <VirtualList
              items={files}
              rowHeight={30}
              {selectedIndex}
              label={t('history.compareFiles', { count: files.length })}
              onselect={(i) => files[i] && history.selectComparedFile(files[i].path)}
              onactivate={(i) => files[i] && diffTool.openRange(c.base.hash, c.target.hash, files[i])}
            >
              {#snippet row(f)}
                <FileRow path={f.path} origPath={f.origPath} kind={f.kind} />
              {/snippet}
            </VirtualList>
          {/if}
        </div>
      </div>
      <Splitter pane="commitFiles" />
      <div class="diff">
        {#if selectedFile}
          <DiffHeader
            path={selectedFile.path}
            origPath={selectedFile.origPath}
            kind={selectedFile.kind}
            height={34}
            onopen={() => diffTool.openRange(c.base.hash, c.target.hash, selectedFile)}
          />
        {/if}
        <div class="diff-body"><DiffView diff={c.diff} /></div>
      </div>
    </div>
  </div>
{/if}

<style>
  .compare-view {
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
  }

  h2 {
    flex: 1;
    margin: 0;
    font-size: 16px;
    font-weight: 600;
  }

  .range {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 10px;
    font-size: 12px;
  }

  .side {
    flex: 1;
    display: flex;
    gap: 6px;
    min-width: 0;
    padding: 4px 8px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-subtle);
  }

  .side .mono {
    flex: none;
    color: var(--accent);
  }

  .subject {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .swap {
    flex: none;
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

  .diff-body {
    flex: 1;
    min-height: 0;
  }
</style>
