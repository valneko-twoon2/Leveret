<!-- Main area of the Issues tab: the selected issue, its comments and actions
     (edit, close / reopen, publish a local issue to GitHub, delete, open on github.com). -->
<script lang="ts">
  import { formatDateTime, formatRelative } from '../format'
  import { t } from '../i18n/index.svelte'
  import { issues } from '../issues.svelte'
  import Icon from './Icon.svelte'
  import IssueLabels from './IssueLabels.svelte'

  const detail = $derived(issues.detail)
  const issue = $derived(detail?.issue ?? null)
  const canPublish = $derived(issue?.source === 'local' && issues.canUseGithub)
  // Local issues can always be changed; GitHub ones need a sign-in.
  const canEdit = $derived(issue?.source === 'local' || !!issues.sources?.signedIn)

  function onCommentKey(e: KeyboardEvent) {
    if (e.key === 'Enter' && e.ctrlKey) {
      e.preventDefault()
      void issues.addComment()
    }
  }
</script>

{#if !issue}
  <div class="empty">
    <Icon name="issue" size={32} />
    {#if issues.detailFailed}
      {t('issues.loadFailed')}
      <button class="btn" onclick={() => issues.retrySelected()}>{t('issues.retry')}</button>
    {:else}
      {issues.selected ? t('common.loading') : t('issues.select')}
    {/if}
  </div>
{:else if detail}
  <div class="issue-view">
    <header>
      <div class="heading">
        <h2>{issue.title} <span class="number">{issue.source === 'local' ? t('issues.localId', { id: issue.id }) : `#${issue.id}`}</span></h2>
        <div class="actions">
          {#if canPublish}
            <button class="btn primary" disabled={issues.saving} onclick={() => issues.publish()} title={t('issues.publishHint')}>
              <Icon name="upload" size={14} />
              {issue.publishedAs ? t('issues.publishResume') : t('issues.publish')}
            </button>
          {/if}
          <button class="btn" disabled={issues.saving || !canEdit} onclick={() => issues.openEdit()}><Icon name="edit" size={14} /> {t('issues.edit')}</button>
          {#if issue.state === 'open'}
            <button class="btn" disabled={issues.saving || !canEdit} onclick={() => issues.close()}><Icon name="issueClosed" size={14} /> {t('issues.close')}</button>
          {:else}
            <button class="btn" disabled={issues.saving || !canEdit} onclick={() => issues.reopen()}><Icon name="issue" size={14} /> {t('issues.reopen')}</button>
          {/if}
          {#if issue.source === 'local'}
            <button class="btn danger" disabled={issues.saving} onclick={() => issues.remove()} title={t('issues.delete')} aria-label={t('issues.delete')}>
              <Icon name="trash" size={14} />
            </button>
          {:else if issue.url}
            <button class="btn" onclick={() => issues.openInBrowser()} title={t('issues.openInBrowser')} aria-label={t('issues.openInBrowser')}>
              <Icon name="external" size={14} />
            </button>
          {/if}
        </div>
      </div>
      <div class="meta">
        <span class="state" class:closed={issue.state === 'closed'}>
          <Icon name={issue.state === 'closed' ? 'issueClosed' : 'issue'} size={14} />
          {issue.state === 'closed' ? t('issues.closed') : t('issues.open')}
        </span>
        {#if issue.source === 'local'}
          <span class="source"><Icon name="laptop" size={13} /> {t('issues.localOnly')}</span>
        {:else}
          <span class="source"><Icon name="cloud" size={13} /> {issues.sources?.githubRepo ?? t('issues.sourceGithub')}</span>
        {/if}
        {#if issue.author}<span><Icon name="user" size={13} /> {issue.author}</span>{/if}
        <span title={formatDateTime(issue.createdAt)}><Icon name="clock" size={13} /> {t('issues.createdAt', { time: formatRelative(issue.createdAt) })}</span>
        <IssueLabels labels={issue.labels} />
      </div>
    </header>

    <div class="thread">
      {#if issue.publishedAs}
        <p class="partial"><Icon name="alert" size={14} /> {t('issues.publishPartial', { number: issue.publishedAs })}</p>
      {/if}
      <article class="post">
        {#if issue.body.trim()}
          <p class="body">{issue.body}</p>
        {:else}
          <p class="body muted">{t('issues.noBody')}</p>
        {/if}
      </article>
      {#each detail.comments as c (c.id)}
        <article class="post">
          <div class="post-meta" title={formatDateTime(c.createdAt)}>
            <strong>{c.author ?? t('issues.you')}</strong> · {formatRelative(c.createdAt)}
          </div>
          <p class="body">{c.body}</p>
        </article>
      {/each}

      <div class="reply">
        <textarea
          rows="3"
          placeholder={t('issues.commentPlaceholder')}
          bind:value={issues.comment}
          onkeydown={onCommentKey}
          disabled={!canEdit}
          aria-label={t('issues.comment')}
        ></textarea>
        <button class="btn primary" disabled={issues.saving || !canEdit || !issues.comment.trim()} onclick={() => issues.addComment()} title={t('shortcuts.submit')}>
          <Icon name="comment" size={14} />
          {t('issues.comment')}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    height: 100%;
    color: var(--fg-muted);
  }

  .issue-view {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
  }

  header {
    flex: none;
    padding: 14px 18px 12px;
    border-bottom: 1px solid var(--border);
  }

  .heading {
    display: flex;
    align-items: flex-start;
    gap: 12px;
  }

  h2 {
    flex: 1;
    min-width: 0;
    margin: 0;
    font-size: 17px;
    line-height: 1.35;
    overflow-wrap: anywhere;
  }

  .number {
    color: var(--fg-subtle);
    font-weight: 400;
  }

  .actions {
    flex: none;
    display: flex;
    gap: 6px;
  }

  .actions .btn {
    display: inline-flex;
    align-items: center;
    gap: 5px;
  }

  .actions .danger {
    color: var(--danger);
  }

  .meta {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px 14px;
    margin-top: 8px;
    color: var(--fg-muted);
    font-size: 12px;
  }

  .meta span {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }

  .state {
    padding: 1px 8px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--success) 16%, transparent);
    color: var(--success);
    font-weight: 600;
  }

  .state.closed {
    background: var(--bg-muted);
    color: var(--fg-muted);
  }

  .thread {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 14px 18px 18px;
  }

  .post {
    margin-bottom: 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg);
  }

  .post-meta {
    padding: 6px 12px;
    border-bottom: 1px solid var(--border-muted);
    background: var(--bg-subtle);
    color: var(--fg-muted);
    font-size: 12px;
  }

  .body {
    margin: 0;
    padding: 10px 12px;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    line-height: 1.6;
  }

  .muted {
    color: var(--fg-subtle);
  }

  .partial {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0 0 12px;
    padding: 8px 12px;
    border: 1px solid color-mix(in srgb, var(--warning) 45%, transparent);
    border-radius: var(--radius);
    background: color-mix(in srgb, var(--warning) 12%, transparent);
  }

  .reply {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 8px;
  }

  .reply textarea {
    width: 100%;
    resize: vertical;
  }

  .reply .btn {
    display: inline-flex;
    align-items: center;
    gap: 5px;
  }
</style>
