<!-- The file above a diff, with a button that opens it in WinMerge (when installed). -->
<script lang="ts">
  import { onMount } from 'svelte'
  import type { ChangeKind } from '../api'
  import { diffTool } from '../difftool.svelte'
  import { t } from '../i18n/index.svelte'
  import FileRow from './FileRow.svelte'
  import Icon from './Icon.svelte'

  interface Props {
    path: string
    origPath?: string | null
    kind: ChangeKind
    onopen: () => void
    /** Matches the header beside it so the borders line up (the tabs: 37, the commit's file list: 34). */
    height: number
  }
  let { path, origPath = null, kind, onopen, height }: Props = $props()

  const merge = $derived(kind === 'conflicted')

  onMount(() => void diffTool.check())
</script>

<div class="diff-header" style:height="{height}px">
  <div class="file"><FileRow {path} {origPath} {kind} /></div>
  {#if diffTool.available}
    <button class="btn" onclick={onopen} title={t(merge ? 'diff.mergeInToolHint' : 'diff.openInToolHint')}>
      <Icon name="external" size={13} />
      {t(merge ? 'diff.mergeInTool' : 'diff.openInTool')}
    </button>
  {/if}
</div>

<style>
  .diff-header {
    flex: none;
    display: flex;
    align-items: center;
    gap: 8px;
    padding-right: 8px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-subtle);
  }

  .file {
    flex: 1;
    min-width: 0;
    height: 100%;
  }

  .btn {
    flex: none;
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 24px;
    padding: 0 8px;
    font-size: 12px;
  }
</style>
