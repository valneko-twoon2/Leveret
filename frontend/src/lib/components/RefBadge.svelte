<script lang="ts">
  import type { RefLabel } from '../api'
  import Icon from './Icon.svelte'

  let { label }: { label: RefLabel } = $props()
</script>

<span class="ref ref-badge {label.kind}" class:current={label.current} class:short={label.name.length <= 8} title={label.name}>
  {#if label.kind === 'tag'}<Icon name="tag" size={11} />{:else if label.kind === 'remote'}<Icon name="cloud" size={11} />{/if}
  <span class="text">{label.name}</span>
</span>

<style>
  /* Badges give way before the commit subject next to them (higher flex-shrink). */
  .ref {
    flex: 0 3 auto;
    display: inline-flex;
    align-items: center;
    gap: 3px;
    min-width: 0;
    max-width: 140px;
    height: 18px;
    padding: 0 6px;
    border: 1px solid;
    border-radius: 999px;
    font-size: 11px;
    font-weight: 600;
    line-height: 1;
    overflow: hidden;
    white-space: nowrap;
  }

  /* Short names (main, HEAD, v1.2.0) are cheap to show in full. */
  .short {
    flex: none;
  }

  .ref :global(svg) {
    flex: none;
  }

  .text {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .branch,
  .head {
    color: var(--accent);
    border-color: color-mix(in srgb, var(--accent) 50%, transparent);
    background: color-mix(in srgb, var(--accent) 10%, var(--bg));
  }

  .branch.current {
    color: var(--fg-on-accent);
    background: var(--accent);
    border-color: var(--accent);
  }

  .remote {
    color: var(--graph-3);
    border-color: color-mix(in srgb, var(--graph-3) 50%, transparent);
    background: color-mix(in srgb, var(--graph-3) 10%, var(--bg));
  }

  .tag {
    color: var(--warning);
    border-color: color-mix(in srgb, var(--warning) 50%, transparent);
    background: color-mix(in srgb, var(--warning) 10%, var(--bg));
  }
</style>
