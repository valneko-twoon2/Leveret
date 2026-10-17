<!-- Fixed row height list that only renders visible rows. Arrow keys move the selection. -->
<script lang="ts" generics="T">
  import type { Snippet } from 'svelte'

  interface Props {
    items: T[]
    rowHeight: number
    selectedIndex?: number
    /** CSS width of the scrollable content (for horizontally wide rows). */
    contentWidth?: string
    label?: string
    selectable?: boolean
    onselect?: (index: number) => void
    onreachend?: () => void
    /** Right click or the context menu key on a row (the row is selected first). */
    oncontext?: (index: number, e: MouseEvent | KeyboardEvent) => void
    row: Snippet<[T, number]>
    footer?: Snippet
  }

  let {
    items,
    rowHeight,
    selectedIndex = -1,
    contentWidth,
    label,
    selectable = true,
    onselect,
    onreachend,
    oncontext,
    row,
    footer,
  }: Props = $props()

  const OVERSCAN = 8
  let viewport: HTMLDivElement
  let scrollTop = $state(0)
  let height = $state(0)

  const start = $derived(Math.max(0, Math.floor(scrollTop / rowHeight) - OVERSCAN))
  const end = $derived(Math.min(items.length, Math.ceil((scrollTop + height) / rowHeight) + OVERSCAN))
  const visible = $derived(items.slice(start, end))

  function onscroll() {
    scrollTop = viewport.scrollTop
    if (onreachend && scrollTop + height >= items.length * rowHeight - rowHeight * 10) onreachend()
  }

  function ensureVisible(index: number) {
    const top = index * rowHeight
    if (top < viewport.scrollTop) viewport.scrollTop = top
    else if (top + rowHeight > viewport.scrollTop + height) viewport.scrollTop = top + rowHeight - height
  }

  function onkeydown(e: KeyboardEvent) {
    if (oncontext && selectedIndex >= 0 && (e.key === 'ContextMenu' || (e.shiftKey && e.key === 'F10'))) {
      e.preventDefault()
      const rowEl = viewport.querySelector<HTMLElement>('.row.selected')
      oncontext(selectedIndex, rowEl ? new MouseEvent('contextmenu', rowCenter(rowEl)) : e)
      return
    }
    if (!selectable || !onselect || items.length === 0) return
    const page = Math.max(1, Math.floor(height / rowHeight) - 1)
    const moves: Record<string, number> = {
      ArrowDown: selectedIndex + 1,
      ArrowUp: selectedIndex - 1,
      PageDown: selectedIndex + page,
      PageUp: selectedIndex - page,
      Home: 0,
      End: items.length - 1,
    }
    if (!(e.key in moves)) return
    e.preventDefault()
    const next = Math.min(items.length - 1, Math.max(0, moves[e.key]))
    if (next !== selectedIndex) onselect(next)
    ensureVisible(next)
  }

  function rowCenter(el: HTMLElement): MouseEventInit {
    const r = el.getBoundingClientRect()
    return { clientX: r.left + 24, clientY: r.top + r.height / 2 }
  }

  // Reset scroll when the list is replaced by a shorter one (e.g. another repository).
  $effect(() => {
    if (viewport && items.length * rowHeight < viewport.scrollTop) {
      viewport.scrollTop = 0
      scrollTop = 0
    }
  })
</script>

<div
  class="viewport"
  bind:this={viewport}
  bind:clientHeight={height}
  {onscroll}
  {onkeydown}
  role="listbox"
  aria-label={label}
  tabindex={selectable ? 0 : -1}
>
  <div class="content" style:height="{items.length * rowHeight}px" style:min-width={contentWidth}>
    {#each visible as item, k (start + k)}
      {@const index = start + k}
      <div
        class="row"
        class:selected={selectable && index === selectedIndex}
        style:top="{index * rowHeight}px"
        style:height="{rowHeight}px"
        role="option"
        aria-selected={index === selectedIndex}
        tabindex="-1"
        onmousedown={(e) => e.button === 0 && selectable && onselect?.(index)}
        oncontextmenu={(e) => {
          if (!oncontext) return
          if (selectable && index !== selectedIndex) onselect?.(index)
          oncontext(index, e)
        }}
      >
        {@render row(item, index)}
      </div>
    {/each}
  </div>
  {@render footer?.()}
</div>

<style>
  .viewport {
    position: relative;
    height: 100%;
    overflow: auto;
    outline: none;
  }

  .content {
    position: relative;
  }

  .row {
    position: absolute;
    left: 0;
    right: 0;
    display: flex;
    align-items: center;
  }

  .row.selected {
    background: var(--bg-selected);
  }

  .viewport:focus-within .row.selected {
    background: var(--bg-selected-strong);
    color: var(--fg-on-accent);
    --fg-muted: color-mix(in srgb, var(--fg-on-accent) 80%, transparent);
    --fg-subtle: color-mix(in srgb, var(--fg-on-accent) 70%, transparent);
  }
</style>
