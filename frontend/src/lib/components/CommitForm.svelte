<script lang="ts">
  import { t } from '../i18n/index.svelte'
  import { app } from '../state.svelte'

  const label = $derived.by(() => {
    if (app.committing) return t('commit.committing')
    const branch = app.status?.branch
    return branch ? t('commit.button', { branch }) : t('commit.buttonDetached')
  })

  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && e.ctrlKey) {
      e.preventDefault()
      void app.commit()
    }
  }
</script>

<form
  class="commit-form"
  onsubmit={(e) => {
    e.preventDefault()
    void app.commit()
  }}
>
  <input type="text" placeholder={t('commit.summary')} bind:value={app.summary} {onkeydown} aria-label={t('commit.summary')} />
  <textarea rows="4" placeholder={t('commit.description')} bind:value={app.description} {onkeydown} aria-label={t('commit.description')}
  ></textarea>
  <button class="btn primary" type="submit" disabled={!app.canCommit} title={t('commit.shortcut')}>
    <span class="label">{label}</span>
  </button>
  <div class="hint">{t('commit.selected', { count: app.includedFiles.length })} · {t('commit.shortcut')}</div>
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
