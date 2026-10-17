<!-- An issue's labels as small colored chips (GitHub colors when known). -->
<script lang="ts">
  import type { IssueLabel } from '../issueApi'

  interface Props {
    labels: IssueLabel[]
    /** Show at most this many chips, then a "+N" chip listing the rest in its tooltip. */
    max?: number
  }
  let { labels, max = Infinity }: Props = $props()

  const shown = $derived(labels.length > max ? labels.slice(0, max) : labels)
  const rest = $derived(labels.slice(shown.length))

  /** GitHub sends a 6-digit hex color; anything else falls back to the neutral chip. */
  function color(l: IssueLabel): string | undefined {
    return l.color && /^[0-9a-f]{6}$/i.test(l.color) ? `#${l.color}` : undefined
  }
</script>

{#each shown as l (l.name)}
  <span class="label" style:--label={color(l)} title={l.name}>{l.name}</span>
{/each}
{#if rest.length}
  <span class="label more" title={rest.map((l) => l.name).join(', ')}>+{rest.length}</span>
{/if}

<style>
  .label {
    flex: 0 1 auto;
    min-width: 0;
    max-width: 140px;
    padding: 0 7px;
    border: 1px solid color-mix(in srgb, var(--label, var(--fg-muted)) 45%, transparent);
    border-radius: 999px;
    background: color-mix(in srgb, var(--label, var(--fg-muted)) 16%, transparent);
    color: var(--fg);
    font-size: 11px;
    line-height: 17px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .more {
    flex: none;
  }
</style>
