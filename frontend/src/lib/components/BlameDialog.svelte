<!-- Annotate (blame): each line of a file with the commit that last changed it. -->
<script lang="ts">
  import { blame, isUncommitted } from '../blame.svelte'
  import { formatDateTime, formatRelative, shortHash } from '../format'
  import { t } from '../i18n/index.svelte'
  import { prefs } from '../prefs.svelte'
  import Modal from './Modal.svelte'
  import VirtualList from './VirtualList.svelte'

  const data = $derived(blame.data)
  const title = $derived(
    blame.rev ? t('blame.titleAt', { path: blame.path ?? '', hash: shortHash(blame.rev) }) : t('blame.title', { path: blame.path ?? '' }),
  )

  /** A row per line; `first` marks where a run of lines from one commit starts. */
  const rows = $derived.by(() => {
    if (!data) return []
    let block = 0
    return data.lines.map((line, i) => {
      const first = i === 0 || data.lines[i - 1].commit !== line.commit
      if (first && i > 0) block++
      return { no: i + 1, text: line.text, commit: data.commits[line.commit], first, odd: block % 2 === 1 }
    })
  })

  const contentWidth = $derived(`calc(${Math.max(0, ...rows.map((r) => r.text.length)) + 2}ch + 300px)`)
</script>

<Modal {title} width={1100} onclose={() => blame.close()}>
  <div class="blame" style:tab-size={prefs.tabSize}>
    {#if !data}
      <div class="empty">{t('common.loading')}</div>
    {:else if data.binary}
      <div class="empty">{t('diff.binary')}</div>
    {:else}
      {#if data.truncated}<div class="notice">{t('diff.truncated')}</div>{/if}
      <div class="lines">
        <VirtualList items={rows} rowHeight={20} selectable={false} {contentWidth}>
          {#snippet row(r)}
            {@const uncommitted = isUncommitted(r.commit.hash)}
            <div class="line" class:first={r.first} class:odd={r.odd}>
              {#if r.first}
                <button
                  class="who"
                  disabled={uncommitted}
                  title={uncommitted ? t('blame.uncommitted') : `${r.commit.summary}\n${r.commit.authorName} · ${formatDateTime(r.commit.authorTime)}\n${t('blame.showCommit')}`}
                  onclick={() => blame.showCommit(r.commit.hash)}
                >
                  {#if uncommitted}
                    <span class="summary">{t('blame.uncommitted')}</span>
                  {:else}
                    <span class="hash">{shortHash(r.commit.hash)}</span>
                    <span class="summary">{r.commit.authorName} · {formatRelative(r.commit.authorTime)} · {r.commit.summary}</span>
                  {/if}
                </button>
              {:else}
                <span class="who"></span>
              {/if}
              <span class="no">{r.no}</span>
              <span class="text">{r.text}</span>
            </div>
          {/snippet}
        </VirtualList>
      </div>
    {/if}
  </div>
</Modal>

<style>
  .blame {
    display: flex;
    flex-direction: column;
    height: min(72vh, 720px);
    margin: -16px -20px;
  }

  .notice {
    padding: 6px 12px;
    background: color-mix(in srgb, var(--warning) 14%, transparent);
    color: var(--warning);
  }

  .lines {
    flex: 1;
    min-height: 0;
    font-family: var(--font-mono);
    font-size: 12px;
  }

  .line {
    display: flex;
    width: 100%;
    height: 20px;
    line-height: 20px;
    white-space: pre;
  }

  .line.odd {
    background: var(--bg-subtle);
  }

  .line.first {
    box-shadow: inset 0 1px 0 var(--border-muted);
  }

  .who {
    flex: none;
    display: flex;
    gap: 8px;
    width: 300px;
    padding: 0 8px;
    overflow: hidden;
    border: none;
    border-right: 1px solid var(--border-muted);
    background: transparent;
    color: var(--fg-muted);
    font: inherit;
    font-family: var(--font-ui);
    line-height: inherit;
    text-align: left;
  }

  button.who:not(:disabled) {
    cursor: pointer;
  }

  button.who:not(:disabled):hover {
    background: var(--bg-hover);
    color: var(--fg);
  }

  .hash {
    flex: none;
    font-family: var(--font-mono);
    color: var(--accent);
  }

  .summary {
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .no {
    flex: none;
    width: 52px;
    padding-right: 8px;
    color: var(--fg-subtle);
    text-align: right;
    user-select: none;
  }

  .text {
    flex: 1;
    padding-left: 6px;
    user-select: text;
  }
</style>
