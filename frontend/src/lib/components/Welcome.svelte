<script lang="ts">
  import { pickFolder } from '../dialogs'
  import { github } from '../github.svelte'
  import { t } from '../i18n/index.svelte'
  import { app } from '../state.svelte'
  import Icon from './Icon.svelte'
  import Logo from './Logo.svelte'

  async function add() {
    const path = await pickFolder(t('repos.addDialogTitle'))
    if (path) await app.addRepo(path)
  }
</script>

<div class="welcome">
  <Logo size={96} />
  <h1>{t('welcome.title')}</h1>
  <p>{t('welcome.body')}</p>
  <div class="actions">
    <button class="btn primary" onclick={() => github.openClone()}>
      <Icon name="download" size={15} />
      {t('clone.open')}
    </button>
    <button class="btn" onclick={add}>
      <Icon name="plus" size={15} />
      {t('repos.add')}
    </button>
  </div>
  {#if github.loaded && !github.account}
    <button class="link" onclick={() => github.openSignIn()}>{t('github.signIn')}</button>
  {/if}
</div>

<style>
  .welcome {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    height: 100%;
    padding: 32px;
    text-align: center;
  }

  h1 {
    margin: 8px 0 0;
    font-size: 24px;
    font-weight: 600;
  }

  p {
    margin: 0 0 8px;
    color: var(--fg-muted);
    white-space: pre-line;
    line-height: 1.7;
  }

  .actions {
    display: flex;
    gap: 10px;
  }

  .btn {
    height: 34px;
    padding: 0 18px;
  }

  .link {
    margin-top: 4px;
    padding: 0;
    border: none;
    background: none;
    color: var(--accent);
  }

  .link:hover {
    text-decoration: underline;
  }
</style>
