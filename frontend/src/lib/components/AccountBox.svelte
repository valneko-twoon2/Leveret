<!-- Sidebar footer: the GitHub account and the add / clone buttons. -->
<script lang="ts">
  import { confirmDialog, pickFolder } from '../dialogs'
  import { github } from '../github.svelte'
  import { t } from '../i18n/index.svelte'
  import { app } from '../state.svelte'
  import Icon from './Icon.svelte'

  async function add() {
    const path = await pickFolder(t('repos.addDialogTitle'))
    if (path) await app.addRepo(path)
  }

  async function signOut() {
    if (await confirmDialog(t('github.signOutConfirm'), t('github.signOut'))) await github.signOut()
  }
</script>

<div class="account-box">
  <div class="buttons">
    <button class="btn" onclick={add} title={t('repos.add')}>
      <Icon name="plus" size={14} />
      {t('repos.add')}
    </button>
    <button class="btn" onclick={() => github.openClone()} title={t('clone.open')} aria-label={t('clone.open')}>
      <Icon name="download" size={14} />
    </button>
  </div>

  {#if github.account}
    <div class="account" title={t('github.signedInAs', { login: github.account.login })}>
      {#if github.account.avatar}
        <img src={github.account.avatar} alt="" width="22" height="22" />
      {:else}
        <span class="avatar"><Icon name="user" size={14} /></span>
      {/if}
      <span class="login">{github.account.login}</span>
      <button class="icon-btn" title={t('github.signOut')} aria-label={t('github.signOut')} onclick={signOut}>
        <Icon name="logout" size={14} />
      </button>
    </div>
  {:else if github.loaded}
    <button class="btn signin" onclick={() => github.openSignIn()}>
      <Icon name="user" size={14} />
      {t('github.signIn')}
    </button>
  {/if}
</div>

<style>
  .account-box {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 10px;
    border-top: 1px solid var(--border-muted);
  }

  .buttons {
    display: flex;
    gap: 6px;
  }

  .buttons .btn:first-child {
    flex: 1;
  }

  .buttons .btn:last-child {
    padding: 0 9px;
  }

  .signin {
    width: 100%;
  }

  .account {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 28px;
    padding-left: 4px;
  }

  img,
  .avatar {
    flex: none;
    width: 22px;
    height: 22px;
    border-radius: 50%;
  }

  .avatar {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-muted);
    color: var(--fg-muted);
  }

  .login {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 600;
  }
</style>
