<!-- Interactive rebase: reorder the commits after a base (newest on top), and pick, reword, squash or drop each. -->
<script lang="ts">
  import type { RebaseAction } from '../api'
  import { formatRelative, shortHash } from '../format'
  import { t } from '../i18n/index.svelte'
  import { rewrite } from '../rewrite.svelte'
  import { app } from '../state.svelte'
  import Icon from './Icon.svelte'
  import Modal from './Modal.svelte'

  const ACTIONS: RebaseAction[] = ['pick', 'reword', 'squash', 'fixup', 'drop']
  const plan = $derived(rewrite.rebase)

  /** The oldest kept commit (squash / fixup need a kept commit below them). */
  const oldestKept = $derived(plan ? plan.rows.findLastIndex((r) => r.action !== 'drop') : -1)

  const problem = $derived.by(() => {
    if (!plan) return null
    if (plan.rows.some((r) => r.commit.pushed)) return t('rebase.pushed')
    if (oldestKept < 0) return t('rebase.allDropped')
    const kept = plan.rows[oldestKept]
    if (kept.action === 'squash' || kept.action === 'fixup') return t('rebase.nothingBelow')
    if (plan.rows.some((r) => r.action === 'reword' && !r.message.trim())) return t('rebase.emptyMessage')
    return null
  })

  const resulting = $derived(plan ? plan.rows.filter((r) => r.action === 'pick' || r.action === 'reword').length : 0)

  let dragged = $state<number | null>(null)

  function drop(to: number) {
    if (dragged !== null && dragged !== to) rewrite.moveRow(dragged, to - dragged)
    dragged = null
  }

  function onRowKey(e: KeyboardEvent, i: number) {
    if (!e.altKey || (e.key !== 'ArrowUp' && e.key !== 'ArrowDown')) return
    e.preventDefault()
    rewrite.moveRow(i, e.key === 'ArrowUp' ? -1 : 1)
  }
</script>

{#if plan}
  <Modal title={t('rebase.title')} width={760} onclose={() => rewrite.closeRebase()} closeOnBackdrop={false}>
    <p class="intro">{t('rebase.intro')}</p>
    <ol class="rows">
      {#each plan.rows as row, i (row.commit.hash)}
        <li
          class="row {row.action}"
          class:dragging={dragged === i}
          draggable="true"
          ondragstart={() => (dragged = i)}
          ondragover={(e) => e.preventDefault()}
          ondrop={() => drop(i)}
          ondragend={() => (dragged = null)}
        >
          <span class="handle" aria-hidden="true">⋮⋮</span>
          <span class="move">
            <button class="icon-btn" title={t('rebase.moveUp')} aria-label={t('rebase.moveUp')} disabled={i === 0} onclick={() => rewrite.moveRow(i, -1)} onkeydown={(e) => onRowKey(e, i)}>
              <Icon name="arrowUp" size={13} />
            </button>
            <button class="icon-btn" title={t('rebase.moveDown')} aria-label={t('rebase.moveDown')} disabled={i === plan.rows.length - 1} onclick={() => rewrite.moveRow(i, 1)} onkeydown={(e) => onRowKey(e, i)}>
              <Icon name="arrowDown" size={13} />
            </button>
          </span>
          <select value={row.action} onchange={(e) => rewrite.setAction(i, e.currentTarget.value as RebaseAction)} aria-label={t('rebase.actionFor', { subject: row.commit.subject })}>
            {#each ACTIONS as a (a)}
              <option value={a} disabled={(a === 'squash' || a === 'fixup') && i >= oldestKept && row.action !== a}>{t(`rebase.${a}`)}</option>
            {/each}
          </select>
          <div class="commit">
            {#if row.action === 'reword'}
              <textarea rows="2" value={row.message} oninput={(e) => rewrite.setMessage(i, e.currentTarget.value)} aria-label={t('rebase.reword')}></textarea>
            {:else}
              <span class="subject" title={row.commit.message}>
                {#if row.action === 'squash' || row.action === 'fixup'}<Icon name="arrowDown" size={12} />{/if}
                {row.commit.subject}
              </span>
            {/if}
            <span class="meta">
              <span class="mono">{shortHash(row.commit.hash)}</span> · {row.commit.authorName} · {formatRelative(row.commit.authorTime)}
              {#if row.commit.pushed}<span class="pushed">{t('rebase.pushedBadge')}</span>{/if}
            </span>
          </div>
        </li>
      {/each}
    </ol>
    <p class="note">{t('rebase.stashNote')}</p>
    {#if problem}<p class="problem"><Icon name="alert" size={14} /> {problem}</p>{/if}

    {#snippet footer()}
      <span class="summary">{t('rebase.summary', { from: plan.rows.length, to: resulting })}</span>
      <button class="btn" onclick={() => rewrite.closeRebase()}>{t('common.cancel')}</button>
      <button class="btn primary" disabled={problem !== null || app.busy !== null} onclick={() => rewrite.runRebase()}>{t('rebase.run')}</button>
    {/snippet}
  </Modal>
{/if}

<style>
  .intro,
  .note {
    margin: 0 0 10px;
    color: var(--fg-muted);
    font-size: 12px;
  }

  .note {
    margin: 10px 0 0;
  }

  .rows {
    max-height: min(52vh, 520px);
    margin: 0;
    padding: 0;
    overflow-y: auto;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    list-style: none;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 8px;
    border-bottom: 1px solid var(--border-muted);
    background: var(--bg);
  }

  .row:last-child {
    border-bottom: none;
  }

  .row.dragging {
    opacity: 0.5;
  }

  .handle {
    flex: none;
    color: var(--fg-subtle);
    cursor: grab;
    letter-spacing: -3px;
    user-select: none;
  }

  .move {
    flex: none;
    display: flex;
    flex-direction: column;
  }

  .move .icon-btn {
    width: 20px;
    height: 16px;
  }

  select {
    flex: none;
    width: 230px;
    height: 26px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg);
  }

  .commit {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .subject {
    display: flex;
    align-items: center;
    gap: 4px;
    overflow: hidden;
    font-weight: 500;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  textarea {
    width: 100%;
    resize: vertical;
  }

  .meta {
    color: var(--fg-muted);
    font-size: 11px;
  }

  .pushed {
    margin-left: 6px;
    color: var(--attention);
  }

  .row.squash,
  .row.fixup {
    padding-left: 24px;
    background: var(--bg-subtle);
  }

  .row.fixup .subject {
    color: var(--fg-muted);
  }

  .row.drop .subject {
    color: var(--fg-subtle);
    text-decoration: line-through;
  }

  .problem {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 8px 0 0;
    color: var(--danger);
  }

  .summary {
    margin-right: auto;
    color: var(--fg-muted);
    font-size: 12px;
  }
</style>
