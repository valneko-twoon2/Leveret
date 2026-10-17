<!-- One setting: a label and an optional hint on the left, the control on the right
     (or below, with `stacked`, for wide controls such as text fields). -->
<script lang="ts">
  import type { Snippet } from 'svelte'

  interface Props {
    label: string
    hint?: string
    stacked?: boolean
    children: Snippet
  }
  let { label, hint, stacked = false, children }: Props = $props()
</script>

<div class="row" class:stacked>
  <div class="text">
    <div class="label">{label}</div>
    {#if hint}<div class="hint">{hint}</div>{/if}
  </div>
  <div class="control">{@render children()}</div>
</div>

<style>
  .row {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 12px 0;
    border-bottom: 1px solid var(--border-muted);
  }

  .row.stacked {
    flex-direction: column;
    align-items: stretch;
    gap: 8px;
  }

  .text {
    flex: 1;
    min-width: 0;
  }

  .label {
    font-weight: 600;
  }

  .hint {
    margin-top: 2px;
    color: var(--fg-muted);
    font-size: 12px;
    line-height: 1.5;
  }

  .control {
    flex: none;
    display: flex;
    align-items: center;
    gap: 8px;
    max-width: 60%;
  }

  .stacked .control {
    flex-direction: column;
    align-items: stretch;
    max-width: none;
  }
</style>
