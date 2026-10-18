<script lang="ts">
  import { changes } from '../changes.svelte'
  import { t } from '../i18n/index.svelte'
  import { menu, type MenuItem } from '../menu.svelte'
  import { forgetMessages, recentMessages } from '../messageHistory'
  import { prefs } from '../prefs.svelte'
  import { app } from '../state.svelte'
  import Icon from './Icon.svelte'

  const label = $derived.by(() => {
    if (app.busy === 'commit') return t('commit.committing')
    if (changes.amend) return t('commit.amendButton')
    if (app.operation?.kind === 'merge') return t('commit.mergeButton')
    const branch = app.status?.branch
    return branch ? t('commit.button', { branch }) : t('commit.buttonDetached')
  })

  /** Why committing isn't possible right now (shown instead of the file count). */
  const reason = $derived.by(() => {
    if (app.busy !== null || app.commitBlocked) return null
    if (!changes.amend && changes.includedFiles.length === 0) return t('commit.needFiles')
    if (changes.summary.trim() === '') return t('commit.needSummary')
    return null
  })

  const amendTitle = $derived(
    app.status?.upstream && app.status.ahead === 0 && app.status.headOid ? t('commit.amendPushed') : t('commit.amendHint'),
  )

  /** Where "push after commit" pushes: the upstream, or the branch it would publish. */
  const pushTarget = $derived(app.status?.upstream ?? t('commit.pushAfterPublish', { branch: app.status?.branch ?? '' }))

  function historyMenu(e: MouseEvent) {
    const repo = app.selectedPath
    if (!repo) return
    const messages = recentMessages(repo)
    const items: MenuItem[] = messages.length
      ? messages.map((m) => ({ label: m.summary, action: () => changes.useMessage(m.summary, m.description) }))
      : [{ label: t('commit.recentEmpty'), action: () => {}, disabled: true }]
    if (messages.length) items.push('separator', { label: t('commit.recentClear'), action: () => forgetMessages(repo) })
    menu.open(e, items, e.currentTarget as HTMLElement)
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && e.ctrlKey) {
      e.preventDefault()
      void changes.commit()
    }
  }
</script>

<form
  class="commit-form"
  onsubmit={(e) => {
    e.preventDefault()
    void changes.commit()
  }}
>
  {#if app.commitBlocked}
    <p class="blocked">{t('commit.blockedByOperation', { operation: app.operationName() })}</p>
  {/if}
  <div class="summary">
    <input
      type="text"
      placeholder={t('commit.summary')}
      bind:value={changes.summary}
      {onkeydown}
      aria-label={t('commit.summary')}
      disabled={app.commitBlocked}
    />
    <button
      type="button"
      class="icon-btn"
      title={t('commit.recent')}
      aria-label={t('commit.recent')}
      disabled={app.commitBlocked}
      onclick={historyMenu}
    >
      <Icon name="clock" size={15} />
    </button>
  </div>
  <textarea
    rows="3"
    placeholder={t('commit.description')}
    bind:value={changes.description}
    {onkeydown}
    aria-label={t('commit.description')}
    disabled={app.commitBlocked}
  ></textarea>
  <label class="option" title={amendTitle}>
    <input
      type="checkbox"
      checked={changes.amend}
      disabled={!app.canRewriteHead || app.busy !== null}
      onchange={(e) => changes.setAmend(e.currentTarget.checked)}
    />
    {t('commit.amend')}
  </label>
  {#if changes.canPushAfterCommit}
    <label class="option" title={t('commit.pushAfterHint', { target: pushTarget })}>
      <input type="checkbox" checked={prefs.pushAfterCommit} onchange={(e) => prefs.set('pushAfterCommit', e.currentTarget.checked)} />
      <span class="option-text">{t('commit.pushAfterTarget', { target: pushTarget })}</span>
    </label>
  {/if}
  <button class="btn primary" type="submit" disabled={!changes.canCommit} title={t('commit.shortcut')}>
    <span class="label">{label}</span>
  </button>
  <div class="hint" class:reason={reason !== null}>
    {reason ?? `${t('commit.selected', { count: changes.includedFiles.length })} · ${t('commit.shortcut')}`}
  </div>
</form>

<style>
  .commit-form {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 12px;
    border-top: 1px solid var(--border);
    background: var(--bg-subtle);
  }

  textarea {
    resize: none;
  }

  input:disabled,
  textarea:disabled {
    opacity: 0.6;
  }

  .blocked {
    margin: 0;
    color: var(--attention);
    font-size: 12px;
  }

  .summary {
    position: relative;
    display: flex;
  }

  .summary input {
    flex: 1;
    min-width: 0;
    padding-right: 32px;
  }

  .summary .icon-btn {
    position: absolute;
    top: 50%;
    right: 3px;
    transform: translateY(-50%);
    width: 26px;
    height: 26px;
  }

  /* Only the box and its text toggle the option, not the whole row. */
  .option {
    display: flex;
    align-items: center;
    align-self: flex-start;
    gap: 6px;
    max-width: 100%;
    color: var(--fg-muted);
    font-size: 12px;
  }

  .option-text {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .btn {
    height: auto;
    min-height: 32px;
    padding-top: 4px;
    padding-bottom: 4px;
  }

  /* Long branch names wrap instead of being cut off. */
  .label {
    white-space: normal;
    overflow-wrap: anywhere;
  }

  .hint {
    font-size: 11px;
    color: var(--fg-subtle);
    text-align: center;
  }

  .hint.reason {
    color: var(--fg-muted);
  }
</style>
