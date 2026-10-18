<!-- Tag list in the branches dialog: check out with a click; push, delete or copy from the "..." menu. -->
<script lang="ts">
  import type { Tag } from '../api'
  import { formatDateTime, formatRelative, shortHash } from '../format'
  import { t } from '../i18n/index.svelte'
  import { menu } from '../menu.svelte'
  import { app } from '../state.svelte'
  import { tags } from '../tags.svelte'
  import Icon from './Icon.svelte'

  let { filter }: { filter: string } = $props()

  const list = $derived.by(() => {
    const q = filter.trim().toLowerCase()
    return q ? tags.tags.filter((tag) => tag.name.toLowerCase().includes(q) || tag.subject.toLowerCase().includes(q)) : tags.tags
  })

  function tagMenu(tag: Tag, e: MouseEvent | KeyboardEvent, anchor?: HTMLElement) {
    menu.open(e, tags.menuItems(tag), anchor)
  }
</script>

<ul>
  {#each list as tag (tag.name)}
    <li>
      <button class="row" disabled={app.busy !== null} onclick={(e) => tagMenu(tag, e, e.currentTarget as HTMLElement)} oncontextmenu={(e) => tagMenu(tag, e)}>
        <Icon name="tag" size={14} />
        <span class="name" title={tag.name}>{tag.name}</span>
        {#if tag.annotated}<span class="kind" title={t('tags.annotatedHint')}>{t('tags.annotated')}</span>{/if}
        <span class="subject" title={tag.subject}>{tag.subject}</span>
        <span class="hash mono">{shortHash(tag.target)}</span>
        <span class="time" title={formatDateTime(tag.time)}>{formatRelative(tag.time)}</span>
      </button>
      <button class="icon-btn more" title={t('common.more')} aria-label={t('common.more')} onclick={(e) => tagMenu(tag, e, e.currentTarget as HTMLElement)}>
        <Icon name="more" size={16} />
      </button>
    </li>
  {:else}
    <li class="none">{tags.loaded ? (filter.trim() ? t('tags.noMatches') : t('tags.none')) : t('common.loading')}</li>
  {/each}
</ul>

<style>
  ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  li {
    position: relative;
    display: flex;
    align-items: center;
  }

  .row {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    height: 32px;
    padding: 0 40px 0 8px;
    border: none;
    border-radius: var(--radius);
    background: transparent;
    color: var(--fg);
    text-align: left;
  }

  .row :global(svg) {
    flex: none;
    color: var(--attention);
  }

  .row:hover:not(:disabled),
  .row:focus-visible {
    background: var(--bg-hover);
  }

  .name {
    flex: none;
    max-width: 40%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 600;
  }

  .kind {
    flex: none;
    padding: 0 6px;
    border-radius: 999px;
    background: var(--bg-muted);
    color: var(--fg-muted);
    font-size: 11px;
  }

  .subject {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--fg-muted);
  }

  .hash {
    flex: none;
    color: var(--fg-subtle);
    font-size: 12px;
  }

  .time {
    flex: none;
    width: 80px;
    text-align: right;
    color: var(--fg-subtle);
    font-size: 12px;
  }

  .more {
    position: absolute;
    right: 6px;
  }

  .none {
    justify-content: center;
    padding: 24px;
    color: var(--fg-muted);
  }
</style>
