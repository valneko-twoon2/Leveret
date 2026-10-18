<!-- Image diff: the picture before and after a change, side by side (with size and dimensions). -->
<script lang="ts">
  import { api, type ImageData, type ImageSides } from '../api'
  import { formatBytes } from '../format'
  import { t } from '../i18n/index.svelte'
  import { app } from '../state.svelte'

  let { sides }: { sides: ImageSides } = $props()

  interface Loaded {
    image: ImageData | null
    /** natural size, known once the picture has loaded */
    width?: number
    height?: number
    failed?: boolean
  }

  let before = $state<Loaded | null>(null)
  let after = $state<Loaded | null>(null)
  let loading = $state(true)

  async function fetchSide(repo: string, side: ImageSides['old']): Promise<Loaded> {
    if (!side) return { image: null }
    return { image: await api.getImage(repo, side.rev, side.path) }
  }

  $effect(() => {
    const repo = app.selectedPath
    const current = sides
    if (!repo) return
    loading = true
    let stale = false
    void Promise.all([fetchSide(repo, current.old), fetchSide(repo, current.new)])
      .then(([b, a]) => {
        if (stale) return
        before = b
        after = a
        loading = false
      })
      .catch(() => {
        if (stale) return
        before = after = { image: null, failed: true }
        loading = false
      })
    return () => {
      stale = true
    }
  })

  function measured(side: Loaded | null, e: Event) {
    const img = e.currentTarget as HTMLImageElement
    if (side) {
      side.width = img.naturalWidth
      side.height = img.naturalHeight
    }
  }
</script>

{#snippet panel(label: string, side: Loaded | null)}
  <section>
    <h3>{label}</h3>
    <div class="frame">
      {#if loading}
        <span class="note">{t('common.loading')}</span>
      {:else if side?.failed}
        <span class="note">{t('diff.imageFailed')}</span>
      {:else if !side?.image}
        <span class="note">{t('diff.imageNone')}</span>
      {:else if side.image.dataUrl}
        <img src={side.image.dataUrl} alt={label} onload={(e) => measured(side, e)} />
      {:else}
        <span class="note">{t('diff.imageTooLarge')}</span>
      {/if}
    </div>
    {#if side?.image}
      <p class="info">
        {#if side.width}{side.width} × {side.height} px · {/if}{formatBytes(side.image.size)}
      </p>
    {/if}
  </section>
{/snippet}

<div class="images">
  {@render panel(t('diff.imageBefore'), sides.old ? before : null)}
  {@render panel(t('diff.imageAfter'), sides.new ? after : null)}
</div>

<style>
  .images {
    display: flex;
    align-items: flex-start;
    gap: 16px;
    height: 100%;
    padding: 14px;
    overflow: auto;
  }

  section {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-width: 0;
  }

  h3 {
    margin: 0;
    color: var(--fg-muted);
    font-size: 12px;
    font-weight: 600;
  }

  .frame {
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 160px;
    padding: 10px;
    overflow: auto;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    /* A checkerboard shows transparent areas. */
    background-color: var(--bg);
    background-image:
      linear-gradient(45deg, var(--bg-subtle) 25%, transparent 25%, transparent 75%, var(--bg-subtle) 75%),
      linear-gradient(45deg, var(--bg-subtle) 25%, transparent 25%, transparent 75%, var(--bg-subtle) 75%);
    background-position:
      0 0,
      8px 8px;
    background-size: 16px 16px;
  }

  img {
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
    image-rendering: auto;
  }

  .note {
    color: var(--fg-subtle);
  }

  .info {
    margin: 0;
    color: var(--fg-muted);
    font-size: 12px;
  }
</style>
