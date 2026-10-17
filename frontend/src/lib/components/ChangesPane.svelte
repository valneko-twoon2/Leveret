<!-- Left column of the Changes tab: changed files with checkboxes, and the commit form. -->
<script lang="ts">
  import { t } from '../i18n/index.svelte'
  import { app } from '../state.svelte'
  import CommitForm from './CommitForm.svelte'
  import FileRow from './FileRow.svelte'
  import VirtualList from './VirtualList.svelte'

  const all = $derived(app.files.length > 0 && app.includedFiles.length === app.files.length)
  const some = $derived(app.includedFiles.length > 0 && !all)
  const selectedIndex = $derived(app.files.findIndex((f) => f.path === app.selectedFile))
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
    <span>{t('changes.count', { count: app.files.length })}</span>
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
        onselect={(i) => app.selectFile(app.files[i].path)}
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
    padding: 0 12px;
    border-bottom: 1px solid var(--border-muted);
    color: var(--fg-muted);
    font-size: 12px;
  }

  .list {
    flex: 1;
    min-height: 0;
  }
</style>
