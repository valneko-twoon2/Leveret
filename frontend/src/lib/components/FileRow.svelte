<!-- One file in a change list: [checkbox] path/to/ name [badge] -->
<script lang="ts">
  import type { ChangeKind } from '../api'
  import { splitPath } from '../format'
  import StatusBadge from './StatusBadge.svelte'

  interface Props {
    path: string
    origPath?: string | null
    kind: ChangeKind
    checked?: boolean
    ontoggle?: () => void
  }
  let { path, origPath = null, kind, checked, ontoggle }: Props = $props()

  const [dir, name] = $derived(splitPath(path))
</script>

<div class="file" title={origPath ? `${origPath} → ${path}` : path}>
  {#if ontoggle}
    <input
      type="checkbox"
      {checked}
      onchange={ontoggle}
      onmousedown={(e) => e.stopPropagation()}
      aria-label={path}
    />
  {/if}
  <span class="path">
    {#if origPath}<span class="dir">{origPath} → </span>{/if}<span class="dir">{dir}</span><span class="name">{name}</span>
  </span>
  <StatusBadge {kind} />
</div>

<style>
  .file {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    height: 100%;
    padding: 0 12px;
  }

  .path {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    direction: ltr;
  }

  .dir {
    color: var(--fg-muted);
  }
</style>
