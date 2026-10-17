<!-- Draws one row of the commit graph from the lane data computed by the backend. -->
<script lang="ts">
  import type { GraphRow } from '../api'

  interface Props {
    graph: GraphRow
    lanes: number
    height: number
    merge?: boolean
  }
  let { graph, lanes, height, merge = false }: Props = $props()

  const LANE = 14
  const PAD = 10
  const x = (i: number) => PAD + i * LANE
  const width = $derived(PAD * 2 + (lanes - 1) * LANE)
  const mid = $derived(height / 2)
  const color = (c: number) => `var(--graph-${c % 8})`

  function path(from: number, to: number, y1: number, y2: number): string {
    const x1 = x(from)
    const x2 = x(to)
    if (x1 === x2) return `M${x1} ${y1}V${y2}`
    const ym = (y1 + y2) / 2
    return `M${x1} ${y1}C${x1} ${ym} ${x2} ${ym} ${x2} ${y2}`
  }
</script>

<svg {width} {height} aria-hidden="true">
  {#each graph.edges as e, i (i)}
    <path
      d={e.kind === 'up' ? path(e.from, e.to, 0, mid) : path(e.from, e.to, mid, height)}
      stroke={color(e.color)}
      stroke-width="2"
      fill="none"
    />
  {/each}
  <circle
    cx={x(graph.col)}
    cy={mid}
    r={merge ? 3.5 : 4.5}
    fill={merge ? 'var(--bg)' : color(graph.color)}
    stroke={merge ? color(graph.color) : 'var(--bg)'}
    stroke-width={merge ? 2 : 1.5}
  />
</svg>

<style>
  svg {
    flex: none;
    display: block;
    overflow: hidden;
  }
</style>
