<!-- Vertical drag handle that resizes the pane on its left. Arrow keys resize, double-click resets. -->
<script lang="ts">
  import { t } from '../i18n/index.svelte'
  import { layout, PANES, type PaneName } from '../layout.svelte'

  interface Props {
    pane: PaneName
  }
  let { pane }: Props = $props()

  let dragging = $state(false)
  let startX = 0
  let startSize = 0

  function onpointerdown(e: PointerEvent) {
    if (e.button !== 0) return
    e.preventDefault()
    dragging = true
    startX = e.clientX
    startSize = layout.sizes[pane]
    ;(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId)
  }

  function onpointermove(e: PointerEvent) {
    if (dragging) layout.set(pane, startSize + e.clientX - startX)
  }

  function stop() {
    if (!dragging) return
    dragging = false
    layout.save()
  }

  function onkeydown(e: KeyboardEvent) {
    const step = e.shiftKey ? 48 : 16
    if (e.key === 'ArrowLeft' || e.key === 'ArrowRight') {
      e.preventDefault()
      layout.set(pane, layout.sizes[pane] + (e.key === 'ArrowLeft' ? -step : step))
      layout.save()
    } else if (e.key === 'Home' || e.key === 'End') {
      e.preventDefault()
      layout.set(pane, e.key === 'Home' ? PANES[pane].min : PANES[pane].max)
      layout.save()
    }
  }
</script>

<!-- A focusable separator with a value is an interactive ARIA widget (window splitter pattern). -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
<div
  class="splitter"
  class:dragging
  role="separator"
  aria-orientation="vertical"
  aria-valuenow={layout.sizes[pane]}
  aria-valuemin={PANES[pane].min}
  aria-valuemax={PANES[pane].max}
  aria-label={t('common.resize')}
  title={t('common.resize')}
  tabindex="0"
  {onpointerdown}
  {onpointermove}
  onpointerup={stop}
  onpointercancel={stop}
  onlostpointercapture={stop}
  ondblclick={() => layout.reset(pane)}
  {onkeydown}
></div>

{#if dragging}
  <!-- Keeps the resize cursor and stops text selection while dragging over other elements. -->
  <div class="drag-cover"></div>
{/if}

<style>
  .splitter {
    position: relative;
    z-index: 5;
    flex: none;
    width: 7px;
    margin: 0 -3px;
    cursor: col-resize;
    touch-action: none;
    outline: none;
  }

  .splitter::after {
    content: '';
    position: absolute;
    top: 0;
    bottom: 0;
    left: 3px;
    width: 1px;
    background: var(--border);
    transition: background 0.1s;
  }

  .splitter:hover::after,
  .splitter:focus-visible::after,
  .splitter.dragging::after {
    left: 2px;
    width: 3px;
    background: var(--accent);
  }

  .drag-cover {
    position: fixed;
    inset: 0;
    z-index: 4;
    cursor: col-resize;
  }
</style>
