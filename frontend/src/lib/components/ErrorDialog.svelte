<script lang="ts">
  import { changes } from '../changes.svelte'
  import { github } from '../github.svelte'
  import { t, type MessageKey } from '../i18n/index.svelte'
  import { settings } from '../settings.svelte'
  import { stash } from '../stash.svelte'
  import { app } from '../state.svelte'
  import Icon from './Icon.svelte'

  /** One-click fixes for well-known errors, shown next to OK. */
  const FIXES: Record<string, { label: MessageKey; run: () => unknown }> = {
    identityMissing: { label: 'settings.git.openSettings', run: () => settings.openSettings('git') },
    pushRejected: { label: 'remote.pull', run: () => app.remote('pull') },
    localChangesConflict: { label: 'stash.save', run: () => stash.save() },
    githubNotSignedIn: { label: 'github.signIn', run: () => github.openSignIn() },
    githubUnauthorized: { label: 'github.signIn', run: () => github.openSignIn() },
    noUpstream: { label: 'remote.publish', run: () => app.remote('push') },
  }
  const fix = $derived(app.error?.code ? FIXES[app.error.code] : undefined)

  function runFix() {
    const action = fix
    close()
    if (action) void action.run()
  }

  let button = $state<HTMLButtonElement>()
  $effect(() => {
    if (app.error) button?.focus()
  })

  function close() {
    app.error = null
  }
</script>

<svelte:window onkeydown={(e) => app.error && e.key === 'Escape' && close()} />

{#if app.error}
  <div class="backdrop" role="presentation" onmousedown={close}>
    <div
      class="dialog"
      role="alertdialog"
      aria-modal="true"
      aria-labelledby="error-title"
      tabindex="-1"
      onmousedown={(e) => e.stopPropagation()}
    >
      <h3 id="error-title"><Icon name="alert" size={18} /> {t('errors.title')}</h3>
      <p class="message">{app.error.message}</p>
      {#if app.error.detail}
        <pre class="detail">{app.error.detail}</pre>
      {/if}
      <div class="actions">
        {#if fix}
          <button class="btn" onclick={runFix}>{t(fix.label)}</button>
        {/if}
        <button class="btn primary" bind:this={button} onclick={close}>{t('common.ok')}</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 100;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(0, 0, 0, 0.35);
  }

  .dialog {
    width: min(520px, calc(100vw - 48px));
    padding: 18px 20px 16px;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--bg);
    box-shadow: var(--shadow);
  }

  h3 {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0 0 8px;
    font-size: 15px;
    color: var(--danger);
  }

  .message {
    margin: 0;
    white-space: pre-line;
    user-select: text;
  }

  .detail {
    max-height: 200px;
    margin: 12px 0 0;
    padding: 10px;
    overflow: auto;
    border-radius: var(--radius);
    background: var(--bg-subtle);
    border: 1px solid var(--border-muted);
    font-family: var(--font-mono);
    font-size: 12px;
    white-space: pre-wrap;
    user-select: text;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    margin-top: 16px;
  }

  .actions .btn {
    min-width: 80px;
  }
</style>
