<script lang="ts">
  import { changes } from '../changes.svelte'
  import { t } from '../i18n/index.svelte'
  import { app } from '../state.svelte'

  const label = $derived.by(() => {
    if (app.busy === 'commit') return t('commit.committing')
    if (changes.amend) return t('commit.amendButton')
    if (app.operation?.kind === 'merge') return t('commit.mergeButton')
    const branch = app.status?.branch
    return branch ? t('commit.button', { branch }) : t('commit.buttonDetached')
  })

  const amendTitle = $derived(
    app.status?.upstream && app.status.ahead === 0 && app.status.headOid ? t('commit.amendPushed') : t('commit.amendHint'),
  )

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
  <input
    type="text"
    placeholder={t('commit.summary')}
    bind:value={changes.summary}
    {onkeydown}
    aria-label={t('commit.summary')}
    disabled={app.commitBlocked}
  />
  <textarea
    rows="4"
    placeholder={t('commit.description')}
    bind:value={changes.description}
    {onkeydown}
    aria-label={t('commit.description')}
    disabled={app.commitBlocked}
  ></textarea>
  <label class="amend" title={amendTitle}>
    <input
      type="checkbox"
      checked={changes.amend}
      disabled={!app.canRewriteHead || app.busy !== null}
      onchange={(e) => changes.setAmend(e.currentTarget.checked)}
    />
    {t('commit.amend')}
  </label>
  <button class="btn primary" type="submit" disabled={!changes.canCommit} title={t('commit.shortcut')}>
    <span class="label">{label}</span>
  </button>
  <div class="hint">{t('commit.selected', { count: changes.includedFiles.length })} · {t('commit.shortcut')}</div>
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

  .amend {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--fg-muted);
    font-size: 12px;
  }

  .btn {
    height: 32px;
  }

  .label {
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .hint {
    font-size: 11px;
    color: var(--fg-subtle);
    text-align: center;
  }
</style>
