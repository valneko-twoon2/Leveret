<!-- An issue's labels as small colored chips (GitHub colors when known). -->
<script lang="ts">
  import type { IssueLabel } from '../issueApi'

  let { labels }: { labels: IssueLabel[] } = $props()

  /** GitHub sends a 6-digit hex color; anything else falls back to the neutral chip. */
  function color(l: IssueLabel): string | undefined {
    return l.color && /^[0-9a-f]{6}$/i.test(l.color) ? `#${l.color}` : undefined
  }
</script>

{#each labels as l (l.name)}
  <span class="label" style:--label={color(l)}>{l.name}</span>
{/each}

<style>
  .label {
    flex: none;
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
</style>
