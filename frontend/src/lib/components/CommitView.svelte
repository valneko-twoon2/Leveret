<!-- Main area of the History tab: commit message, metadata, files and the diff. -->
<script lang="ts">
  import { formatDateTime, shortHash } from '../format'
  import { t } from '../i18n/index.svelte'
  import { layout } from '../layout.svelte'
  import { menu } from '../menu.svelte'
  import { app } from '../state.svelte'
  import DiffView from './DiffView.svelte'
  import FileRow from './FileRow.svelte'
  import Icon from './Icon.svelte'
  import Splitter from './Splitter.svelte'
  import VirtualList from './VirtualList.svelte'

  const detail = $derived(app.detail)
  const subject = $derived(detail?.message.split('\n')[0] ?? '')
  const body = $derived(detail?.message.split('\n').slice(1).join('\n').trim() ?? '')
  const selectedIndex = $derived(detail?.files.findIndex((f) => f.path === app.detailFile) ?? -1)

  let copied = $state(false)
  async function copyHash() {
    if (!detail) return
    await navigator.clipboard.writeText(detail.hash)
    copied = true
    setTimeout(() => (copied = false), 1500)
  }
</script>

{#if !detail}
  <div class="empty">{t('history.selectCommit')}</div>
{:else}
  <div class="commit-view">
    <header>
      <h2>{subject}</h2>
      {#if body}<p class="body">{body}</p>{/if}
      <div class="meta">
        <span title={detail.authorEmail}><Icon name="commit" size={14} /> {detail.authorName}</span>
        <span title={t('history.date')}><Icon name="clock" size={14} /> {formatDateTime(detail.authorTime)}</span>
        <button class="hash mono" title={detail.hash} onclick={copyHash}>
          <Icon name={copied ? 'check' : 'copy'} size={13} />
          {copied ? t('common.copied') : shortHash(detail.hash)}
        </button>
        {#if detail.parents.length}
          <span class="parents">
            {t('history.parents')}:
            {#each detail.parents as p (p)}<span class="mono">{shortHash(p)}</span>{/each}
          </span>
        {/if}
        {#if detail.committerName !== detail.authorName}
          <span title={detail.committerEmail}>{t('history.committer')}: {detail.committerName}</span>
        {/if}
      </div>
    </header>

    <div class="split">
      <div class="files" style:width="{layout.sizes.commitFiles}px">
        <div class="files-header">{t('history.filesChanged', { count: detail.files.length })}</div>
        <div class="files-list">
          <VirtualList
            items={detail.files}
            rowHeight={30}
            {selectedIndex}
            label={t('history.filesChanged', { count: detail.files.length })}
            onselect={(i) => detail.files[i] && app.selectCommitFile(detail.files[i].path)}
            oncontext={(i, e) => menu.open(e, [{ label: t('common.copyPath'), action: () => app.copy(detail.files[i]?.path ?? '') }])}
          >
            {#snippet row(f)}
              <FileRow path={f.path} origPath={f.origPath} kind={f.kind} />
            {/snippet}
          </VirtualList>
        </div>
      </div>
      <Splitter pane="commitFiles" />
      <div class="diff">
        <DiffView diff={app.commitDiff} />
      </div>
    </div>
  </div>
{/if}

<style>
  .commit-view {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
  }

  header {
    padding: 14px 16px 12px;
    border-bottom: 1px solid var(--border);
    user-select: text;
  }

  h2 {
    margin: 0;
    font-size: 16px;
    font-weight: 600;
  }

  .body {
    margin: 6px 0 0;
    max-height: 120px;
    overflow: auto;
    white-space: pre-wrap;
    color: var(--fg-muted);
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

  .hash {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: 22px;
    padding: 0 6px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-subtle);
    color: var(--fg);
    font-size: 12px;
  }

  .parents .mono {
    margin-left: 4px;
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
  }
</style>
