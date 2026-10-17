<script lang="ts">
  import { github } from '../github.svelte'
  import { t } from '../i18n/index.svelte'
  import Icon from './Icon.svelte'
  import Modal from './Modal.svelte'

  const TOKEN_PAGE = 'https://github.com/settings/tokens/new?scopes=repo,read:user&description=Leveret'

  function switchTab(tab: 'browser' | 'token') {
    github.signInTab = tab
    if (tab === 'browser' && github.signInStatus === 'idle') void github.startDeviceFlow()
  }
</script>

<Modal title={t('github.dialogTitle')} width={480} onclose={() => github.closeSignIn()}>
  <div class="tabs" role="tablist">
    <button
      role="tab"
      class:active={github.signInTab === 'browser'}
      aria-selected={github.signInTab === 'browser'}
      disabled={!github.deviceFlow}
      onclick={() => switchTab('browser')}
    >
      <Icon name="external" size={14} />
      {t('github.browserTab')}
    </button>
    <button role="tab" class:active={github.signInTab === 'token'} aria-selected={github.signInTab === 'token'} onclick={() => switchTab('token')}>
      <Icon name="key" size={14} />
      {t('github.tokenTab')}
    </button>
  </div>

  {#if github.signInTab === 'browser'}
    <div class="browser">
      {#if github.signInStatus === 'expired' || github.signInStatus === 'denied'}
        <p class="status">{t(github.signInStatus === 'expired' ? 'github.expired' : 'github.denied')}</p>
        <button class="btn" onclick={() => github.startDeviceFlow()}>{t('github.retry')}</button>
      {:else}
        <p>{t('github.browserIntro')}</p>
        <div class="code" aria-live="polite">{github.deviceCode?.userCode ?? '····-····'}</div>
        <button class="btn primary" disabled={!github.deviceCode} onclick={() => github.copyCodeAndOpen()}>
          <Icon name="copy" size={14} />
          {t('github.copyAndOpen')}
        </button>
        {#if github.signInStatus === 'waiting'}
          <p class="status"><span class="spinner"></span>{t('github.waiting')}</p>
        {/if}
      {/if}
    </div>
  {:else}
    <form
      class="token"
      onsubmit={(e) => {
        e.preventDefault()
        void github.signInWithToken()
      }}
    >
      {#if !github.deviceFlow}
        <p class="note">{t('github.deviceFlowUnavailable')}</p>
      {/if}
      <p>{t('github.tokenIntro')}</p>
      <input
        type="password"
        autocomplete="off"
        spellcheck="false"
        placeholder={t('github.tokenPlaceholder')}
        aria-label={t('github.tokenTab')}
        bind:value={github.token}
      />
      <div class="row">
        <button type="button" class="link" onclick={() => github.openUrl(TOKEN_PAGE)}>
          <Icon name="external" size={13} />
          {t('github.createToken')}
        </button>
        <button type="submit" class="btn primary" disabled={!github.token.trim() || github.busy}>{t('github.tokenSubmit')}</button>
      </div>
      <p class="hint"><Icon name="lock" size={12} /> {t('github.tokenStored')}</p>
    </form>
  {/if}
</Modal>

<style>
  .tabs {
    display: flex;
    gap: 4px;
    margin-bottom: 16px;
    padding: 3px;
    border-radius: 8px;
    background: var(--bg-muted);
  }

  .tabs button {
    flex: 1;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    height: 28px;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--fg-muted);
  }

  .tabs button.active {
    background: var(--bg);
    color: var(--fg);
    font-weight: 600;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.12);
  }

  .tabs button:disabled {
    opacity: 0.45;
  }

  p {
    margin: 0 0 12px;
    color: var(--fg-muted);
    white-space: pre-line;
  }

  .browser {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
  }

  .code {
    margin: 4px 0 16px;
    padding: 10px 20px;
    border: 1px dashed var(--border);
    border-radius: 8px;
    background: var(--bg-subtle);
    font-family: var(--font-mono);
    font-size: 28px;
    font-weight: 700;
    letter-spacing: 0.12em;
    user-select: text;
  }

  .status {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 14px 0 0;
  }

  .spinner {
    width: 14px;
    height: 14px;
    border: 2px solid var(--border);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.9s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .token input {
    width: 100%;
    font-family: var(--font-mono);
    user-select: text;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 5px 8px;
    outline: none;
  }

  .token input:focus {
    border-color: var(--accent);
  }

  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-top: 12px;
  }

  .link {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 0;
    border: none;
    background: none;
    color: var(--accent);
  }

  .link:hover {
    text-decoration: underline;
  }

  .note {
    padding: 8px 10px;
    border-radius: var(--radius);
    background: color-mix(in srgb, var(--warning) 12%, transparent);
    color: var(--warning);
  }

  .hint {
    display: flex;
    align-items: center;
    gap: 4px;
    margin: 12px 0 0;
    font-size: 11px;
    color: var(--fg-subtle);
  }
</style>
