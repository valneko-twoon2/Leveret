<script lang="ts">
  import type { DiffLine, FileDiff } from '../api'
  import { t } from '../i18n/index.svelte'
  import VirtualList from './VirtualList.svelte'

  interface Props {
    diff: FileDiff | null
  }
  let { diff }: Props = $props()

  type Row = { type: 'hunk'; text: string } | { type: 'line'; line: DiffLine }

  const rows = $derived.by<Row[]>(() => {
    if (!diff) return []
    const out: Row[] = []
    for (const h of diff.hunks) {
      out.push({ type: 'hunk', text: h.header })
      for (const line of h.lines) out.push({ type: 'line', line })
    }
    return out
  })

  /** Approximate display width in `ch`: tabs are 4, East Asian wide characters 2. */
  function columns(text: string): number {
    let n = 0
    for (const ch of text) {
      const c = ch.codePointAt(0)!
      n += ch === '\t' ? 4 : c >= 0x1100 && (c <= 0x115f || (c >= 0x2e80 && c <= 0xffdc)) ? 2 : 1
    }
    return n
  }

  const contentWidth = $derived.by(() => {
    let max = 0
    for (const r of rows) max = Math.max(max, columns(r.type === 'hunk' ? r.text : r.line.text))
    return `calc(${max + 2}ch + 112px)`
  })

  const SIGN = { context: ' ', add: '+', delete: '-', noNewline: '' } as const
</script>

<div class="diff">
  {#if !diff}
    <div class="empty"></div>
  {:else if diff.binary}
    <div class="empty">{t('diff.binary')}</div>
  {:else if diff.tooLarge}
    <div class="empty">{t('diff.tooLarge')}</div>
  {:else if rows.length === 0}
    <div class="empty">{t('diff.noChanges')}</div>
  {:else}
    {#if diff.truncated}
      <div class="notice">{t('diff.truncated')}</div>
    {/if}
    <div class="lines">
      <VirtualList items={rows} rowHeight={20} selectable={false} {contentWidth}>
        {#snippet row(r: Row)}
          {#if r.type === 'hunk'}
            <div class="line hunk">
              <span class="no"></span><span class="no"></span>
              <span class="text">{r.text}</span>
            </div>
          {:else}
            <div class="line {r.line.kind}">
              <span class="no">{r.line.oldNo ?? ''}</span><span class="no">{r.line.newNo ?? ''}</span>
              <span class="text"><span class="sign">{SIGN[r.line.kind]}</span>{r.line.text}</span>
            </div>
          {/if}
        {/snippet}
      </VirtualList>
    </div>
  {/if}
</div>

<style>
  .diff {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    min-width: 0;
  }

  .notice {
    padding: 6px 12px;
    background: color-mix(in srgb, var(--warning) 14%, transparent);
    color: var(--warning);
    border-bottom: 1px solid var(--border-muted);
  }

  .lines {
    flex: 1;
    min-height: 0;
    font-family: var(--font-mono);
    font-size: 12px;
    user-select: text;
  }

  .line {
    display: flex;
    width: 100%;
    height: 20px;
    line-height: 20px;
    white-space: pre;
    tab-size: 4;
  }

  .no {
    flex: none;
    width: 48px;
    padding-right: 8px;
    text-align: right;
    color: var(--fg-subtle);
    user-select: none;
    background: var(--bg-subtle);
  }

  .no + .no {
    border-right: 1px solid var(--border-muted);
  }

  .text {
    flex: 1;
    padding-left: 6px;
  }

  .sign {
    display: inline-block;
    width: 1.5ch;
    user-select: none;
    color: var(--fg-subtle);
  }

  .hunk {
    background: var(--diff-hunk-bg);
    color: var(--diff-hunk-fg);
  }

  .hunk .no {
    background: transparent;
  }

  .add {
    background: var(--diff-add-bg);
  }

  .add .no {
    background: var(--diff-add-gutter);
  }

  .delete {
    background: var(--diff-del-bg);
  }

  .delete .no {
    background: var(--diff-del-gutter);
  }

  .noNewline .text {
    color: var(--fg-subtle);
    font-style: italic;
  }
</style>
