<script lang="ts" module>
  /** Picking lines for a partial commit or discard. Only the Changes tab passes one. */
  export interface LinePicker {
    included(h: number, l: number): boolean
    setLines(lines: [number, number][], include: boolean): void
    /** Right click on a hunk header (`l` null) or a line. */
    onmenu?(e: MouseEvent, h: number, l: number | null): void
  }
</script>

<script lang="ts">
  import { isImagePath, type DiffLine, type FileDiff, type ImageSides } from '../api'
  import { t } from '../i18n/index.svelte'
  import { prefs } from '../prefs.svelte'
  import ImageDiff from './ImageDiff.svelte'
  import VirtualList from './VirtualList.svelte'

  interface Props {
    diff: FileDiff | null
    picker?: LinePicker
    /** Where the pictures of an image file are (shown instead of "binary file"). */
    images?: ImageSides
  }
  let { diff, picker, images }: Props = $props()

  type Row = { type: 'hunk'; text: string; h: number } | { type: 'line'; line: DiffLine; h: number; l: number }

  const rows = $derived.by<Row[]>(() => {
    if (!diff) return []
    const out: Row[] = []
    diff.hunks.forEach((hunk, h) => {
      out.push({ type: 'hunk', text: hunk.header, h })
      hunk.lines.forEach((line, l) => out.push({ type: 'line', line, h, l }))
    })
    return out
  })

  /** Approximate display width in `ch`: tabs are 4, East Asian wide characters 2. */
  function columns(text: string): number {
    let n = 0
    for (const ch of text) {
      const c = ch.codePointAt(0)!
      n += ch === '\t' ? prefs.tabSize : c >= 0x1100 && (c <= 0x115f || (c >= 0x2e80 && c <= 0xffdc)) ? 2 : 1
    }
    return n
  }

  const pick = $derived(diff?.selectable ? picker : undefined)
  const gutter = $derived(pick ? 134 : 112)

  const contentWidth = $derived.by(() => {
    let max = 0
    for (const r of rows) max = Math.max(max, columns(r.type === 'hunk' ? r.text : r.line.text))
    return `calc(${max + 2}ch + ${gutter}px)`
  })

  const SIGN = { context: ' ', add: '+', delete: '-', noNewline: '' } as const

  const isChange = (line: DiffLine) => line.kind === 'add' || line.kind === 'delete'

  function hunkLines(h: number): [number, number][] {
    return (diff?.hunks[h]?.lines ?? []).flatMap((line, l) => (isChange(line) ? [[h, l] as [number, number]] : []))
  }

  function hunkState(p: LinePicker, h: number): 'all' | 'some' | 'none' {
    const lines = hunkLines(h)
    const n = lines.filter(([hh, l]) => p.included(hh, l)).length
    return n === lines.length ? 'all' : n === 0 ? 'none' : 'some'
  }

  // Dragging over the gutter gives every line the state the first one got;
  // Shift+click picks the range from the previous click.
  let dragInclude: boolean | null = null
  let anchor: number | null = null

  function pickDown(e: PointerEvent, p: LinePicker, index: number, h: number, l: number) {
    if (e.button !== 0) return
    e.preventDefault()
    const include = !p.included(h, l)
    if (e.shiftKey && anchor !== null) {
      const [from, to] = anchor < index ? [anchor, index] : [index, anchor]
      const range = rows.slice(from, to + 1)
      p.setLines(range.flatMap((r) => (r.type === 'line' && isChange(r.line) ? [[r.h, r.l] as [number, number]] : [])), include)
    } else {
      p.setLines([[h, l]], include)
    }
    anchor = index
    dragInclude = include
    window.addEventListener('pointerup', () => (dragInclude = null), { once: true })
  }

  function pickEnter(p: LinePicker, h: number, l: number) {
    if (dragInclude !== null) p.setLines([[h, l]], dragInclude)
  }

  function oncontext(index: number, e: MouseEvent | KeyboardEvent) {
    const r = rows[index]
    if (!r || !pick?.onmenu || !(e instanceof MouseEvent)) return
    pick.onmenu(e, r.h, r.type === 'line' ? r.l : null)
  }
</script>

<div class="diff" style:tab-size={prefs.tabSize}>
  {#if !diff}
    <div class="empty"></div>
  {:else if diff.binary && images && isImagePath(images.new?.path ?? images.old?.path ?? '')}
    <ImageDiff sides={images} />
  {:else if diff.binary}
    <div class="empty">{t('diff.binary')}</div>
  {:else if diff.tooLarge}
    <div class="empty">{t('diff.tooLarge')}</div>
  {:else if rows.length === 0}
    <div class="empty">{t('diff.noChanges')}</div>
  {:else}
    {#if diff.truncated}
      <div class="notice">{t('diff.truncated')}</div>
    {:else if picker && !diff.selectable && prefs.ignoreWhitespace}
      <div class="notice muted">{t('diff.pickNeedsWhitespace')}</div>
    {/if}
    <div class="lines" class:picking={!!pick}>
      <VirtualList items={rows} rowHeight={20} selectable={false} {contentWidth} oncontext={pick?.onmenu ? oncontext : undefined}>
        {#snippet row(r: Row, index: number)}
          {#if r.type === 'hunk'}
            <div class="line hunk">
              {#if pick}
                {@const state = hunkState(pick, r.h)}
                <span class="pick">
                  <input
                    type="checkbox"
                    checked={state === 'all'}
                    indeterminate={state === 'some'}
                    title={t('diff.pickHunk')}
                    aria-label={t('diff.pickHunk')}
                    onchange={() => pick.setLines(hunkLines(r.h), state !== 'all')}
                  />
                </span>
              {/if}
              <span class="no"></span><span class="no"></span>
              <span class="text">{r.text}</span>
            </div>
          {:else}
            {@const change = isChange(r.line)}
            {@const included = !pick || !change || pick.included(r.h, r.l)}
            <div class="line {r.line.kind}" class:excluded={!included}>
              {#if pick && change}
                <button
                  class="gutter"
                  tabindex="-1"
                  title={t('diff.pickLine')}
                  aria-label={t('diff.pickLine')}
                  aria-pressed={included}
                  onpointerdown={(e) => pickDown(e, pick, index, r.h, r.l)}
                  onpointerenter={() => pickEnter(pick, r.h, r.l)}
                >
                  <span class="pick"><span class="box" class:on={included}></span></span>
                  <span class="no">{r.line.oldNo ?? ''}</span><span class="no">{r.line.newNo ?? ''}</span>
                </button>
              {:else}
                {#if pick}<span class="pick"></span>{/if}
                <span class="no">{r.line.oldNo ?? ''}</span><span class="no">{r.line.newNo ?? ''}</span>
              {/if}
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

  .notice.muted {
    background: var(--bg-subtle);
    color: var(--fg-muted);
    font-size: 12px;
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
    tab-size: inherit;
  }

  .gutter {
    display: flex;
    flex: none;
    height: 100%;
    padding: 0;
    border: none;
    border-radius: 0;
    background: transparent;
    color: inherit;
    font: inherit;
    line-height: inherit;
    cursor: pointer;
  }

  .pick {
    flex: none;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 22px;
    background: var(--bg-subtle);
    user-select: none;
  }

  .pick input {
    margin: 0;
  }

  .box {
    width: 11px;
    height: 11px;
    border: 1px solid var(--fg-subtle);
    border-radius: 3px;
    background: var(--bg);
  }

  .box.on {
    border-color: var(--accent);
    background: var(--accent);
    box-shadow: inset 0 0 0 2px var(--bg);
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

  .hunk .no,
  .hunk .pick {
    background: transparent;
  }

  .add {
    background: var(--diff-add-bg);
  }

  .add .no,
  .add .pick {
    background: var(--diff-add-gutter);
  }

  .delete {
    background: var(--diff-del-bg);
  }

  .delete .no,
  .delete .pick {
    background: var(--diff-del-gutter);
  }

  /* Lines left out of the commit: shown, but without their change colors. */
  .line.excluded {
    background: transparent;
  }

  .line.excluded .no,
  .line.excluded .pick {
    background: var(--bg-subtle);
  }

  .line.excluded .text {
    color: var(--fg-muted);
  }

  .gutter:hover .box {
    border-color: var(--accent);
  }

  .noNewline .text {
    color: var(--fg-subtle);
    font-style: italic;
  }
</style>
