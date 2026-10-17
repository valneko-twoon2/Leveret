<script lang="ts">
  import type { ChangeKind } from '../api'
  import { t } from '../i18n/index.svelte'

  let { kind }: { kind: ChangeKind } = $props()

  const LETTER: Record<ChangeKind, string> = {
    modified: 'M',
    added: 'A',
    deleted: 'D',
    renamed: 'R',
    copied: 'C',
    typeChanged: 'T',
    untracked: 'U',
    conflicted: '!',
  }
</script>

<span class="badge {kind}" title={t(`kind.${kind}`)}>{LETTER[kind]}</span>

<style>
  .badge {
    flex: none;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 16px;
    height: 16px;
    border: 1.5px solid currentColor;
    border-radius: 4px;
    font-size: 10px;
    font-weight: 700;
    line-height: 1;
    font-family: var(--font-mono);
  }
  .modified,
  .typeChanged {
    color: var(--warning);
  }
  .added,
  .untracked,
  .copied {
    color: var(--success);
  }
  .deleted {
    color: var(--danger);
  }
  .renamed {
    color: var(--accent);
  }
  .conflicted {
    color: var(--attention);
  }
</style>
