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
  const origName = $derived(origPath ? splitPath(origPath)[1] : '')
</script>

<div class="file" title={origPath ? `${origPath} → ${path}` : path}>
  {#if ontoggle}
    <input
      type="checkbox"
      {checked}
      onchange={ontoggle}
      onmousedown={(e) => e.stopPropagation()}
      tabindex={-1}
      aria-label={path}
    />
  {/if}
  <span class="path">
    {#if origPath}<span class="orig"><bdi>{origName}</bdi> → </span>{/if}{#if dir}<span class="dir"><bdi>{dir}</bdi></span>{/if}<span class="name"><bdi>{name}</bdi></span>
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

  /* The file name never shrinks; the directory (and a rename's old name) give way first. */
  .path {
    flex: 1;
    display: flex;
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
    direction: ltr;
  }

  .orig,
  .dir,
  .name {
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .orig,
  .dir {
    flex: 0 1 auto;
    min-width: 0;
    color: var(--fg-muted);
  }

  /* Ellipsis at the start, so the end of the directory next to the name stays visible. */
  .dir {
    direction: rtl;
    text-align: left;
  }

  .dir bdi {
    direction: ltr;
    unicode-bidi: isolate;
  }

  .orig {
    flex-shrink: 2;
    padding-right: 0.3em;
  }

  .name {
    flex: 0 0 auto;
    max-width: 100%;
  }
</style>
