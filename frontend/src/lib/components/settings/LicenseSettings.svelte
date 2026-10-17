<!-- Settings: Leveret's license and the licenses of the open source packages it ships. -->
<script lang="ts">
  import { onMount } from 'svelte'
  import { api } from '../../api'
  import { t } from '../../i18n/index.svelte'
  import { loadLicenses, type LicenseData, type ThirdPartyPackage } from '../../licenses'
  import { perf } from '../../perf'
  import { app, describeError } from '../../state.svelte'
  import Icon from '../Icon.svelte'

  let data = $state.raw<LicenseData | null>(null)
  let filter = $state('')
  let openKey = $state<string | null>(null)

  const list = $derived.by(() => {
    const q = filter.trim().toLowerCase()
    const all = data?.packages ?? []
    return q ? all.filter((p) => p.name.toLowerCase().includes(q) || p.license.toLowerCase().includes(q)) : all
  })
  const groups = $derived([
    { label: t('settings.about.rust'), items: list.filter((p) => p.ecosystem === 'rust') },
    { label: t('settings.about.npm'), items: list.filter((p) => p.ecosystem === 'npm') },
  ])

  const key = (p: ThirdPartyPackage) => `${p.ecosystem}:${p.name}@${p.version}`
  const isGithub = (url: string) => /^https:\/\/github\.com\//.test(url)

  onMount(() => {
    void perf.view('settings.licenses', async () => {
      data = await loadLicenses().catch((e: unknown) => {
        app.error = describeError(e)
        return null
      })
    })
  })
</script>

<h4>{t('settings.about.ownLicense')}</h4>
<p class="hint">{t('settings.about.licenseHint')}</p>
{#if data}
  <pre class="text">{data.self}</pre>
{/if}

<h4>{t('settings.about.thirdParty')}</h4>
<p class="hint">{t('settings.about.thirdPartyHint')}</p>
{#if !data}
  <p class="hint">{t('common.loading')}</p>
{:else}
  <div class="top">
    <div class="search">
      <Icon name="search" size={14} />
      <input type="text" placeholder={t('settings.about.filter')} bind:value={filter} aria-label={t('settings.about.filter')} />
    </div>
    <span class="count">{t('settings.about.thirdPartyCount', { count: list.length })}</span>
  </div>
  {#each groups as group (group.label)}
    {#if group.items.length}
      <h5>{group.label}</h5>
      <ul>
        {#each group.items as p (key(p))}
          {@const open = openKey === key(p)}
          <li>
            <button class="row" aria-expanded={open} onclick={() => (openKey = open ? null : key(p))}>
              <span class="chevron" class:open><Icon name="chevronDown" size={12} /></span>
              <span class="name">{p.name}</span>
              <span class="version mono">{p.version}</span>
              <span class="license">{p.license}</span>
            </button>
            {#if open}
              <div class="details">
                {#if p.repository}
                  <p class="repo">
                    {#if isGithub(p.repository)}
                      <button class="link" onclick={() => app.guard(() => api.openGithubUrl(p.repository))}>
                        <Icon name="external" size={12} />
                        {p.repository}
                      </button>
                    {:else}
                      <span class="mono">{p.repository}</span>
                    {/if}
                  </p>
                {/if}
                {#each p.texts as index (index)}
                  <pre class="text">{data.texts[index]}</pre>
                {:else}
                  <p class="hint">{t('settings.about.noText')}</p>
                {/each}
              </div>
            {/if}
          </li>
        {/each}
      </ul>
    {/if}
  {/each}
{/if}

<style>
  h4 {
    margin: 14px 0 4px;
  }

  h5 {
    margin: 12px 0 4px;
    color: var(--fg-muted);
    font-size: 11px;
    font-weight: 600;
  }

  .hint {
    margin: 0 0 8px;
    color: var(--fg-muted);
    font-size: 12px;
  }

  .text {
    max-height: 220px;
    overflow: auto;
    margin: 0 0 8px;
    padding: 10px 12px;
    border: 1px solid var(--border-muted);
    border-radius: var(--radius);
    background: var(--bg-subtle);
    font-family: var(--font-mono);
    font-size: 11px;
    font-weight: 400;
    line-height: 1.5;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    user-select: text;
  }

  .top {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 4px;
  }

  .search {
    flex: 1;
    position: relative;
    display: flex;
    align-items: center;
    color: var(--fg-muted);
  }

  .search :global(svg) {
    position: absolute;
    left: 9px;
  }

  .search input {
    width: 100%;
    padding-left: 28px;
  }

  .count {
    color: var(--fg-muted);
    font-size: 12px;
  }

  ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    height: 28px;
    padding: 0 6px;
    border: none;
    border-radius: var(--radius);
    background: transparent;
    color: var(--fg);
    text-align: left;
  }

  .row:hover {
    background: var(--bg-hover);
  }

  .chevron {
    display: inline-flex;
    color: var(--fg-subtle);
    transform: rotate(-90deg);
    transition: transform 0.1s;
  }

  .chevron.open {
    transform: none;
  }

  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .version {
    color: var(--fg-subtle);
    font-size: 12px;
  }

  .license {
    flex: none;
    width: 150px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--fg-muted);
    font-size: 12px;
    text-align: right;
  }

  .details {
    padding: 4px 6px 8px 26px;
  }

  .repo {
    margin: 0 0 6px;
    font-size: 12px;
  }

  .link {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 0;
    border: none;
    background: transparent;
    color: var(--accent);
  }
</style>
