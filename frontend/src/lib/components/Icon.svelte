<script lang="ts" module>
  const PATHS = {
    plus: 'M12 5v14M5 12h14',
    refresh: 'M20 12a8 8 0 1 1-2.34-5.66M20 4v5h-5',
    branch: 'M6 3v12M18 9a9 9 0 0 1-9 9M6 21a3 3 0 1 0 0-6 3 3 0 0 0 0 6zM18 9a3 3 0 1 0 0-6 3 3 0 0 0 0 6z',
    x: 'M6 6l12 12M18 6L6 18',
    repo: 'M5 19.5V5a2 2 0 0 1 2-2h12v14H7a2 2 0 0 0-2 2.5zM5 19.5A2 2 0 0 0 7 21h12v-4M9 7h6',
    arrowUp: 'M12 19V5M6 11l6-6 6 6',
    arrowDown: 'M12 5v14M6 13l6 6 6-6',
    copy: 'M9 9h10a1 1 0 0 1 1 1v10a1 1 0 0 1-1 1H9a1 1 0 0 1-1-1V10a1 1 0 0 1 1-1zM5 15H4a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1h10a1 1 0 0 1 1 1v1',
    check: 'M5 12.5l4.5 4.5L19 7.5',
    file: 'M14 3H6.5a1 1 0 0 0-1 1v16a1 1 0 0 0 1 1h11a1 1 0 0 0 1-1V7.5zM14 3v4.5h4.5',
    commit: 'M12 15.5a3.5 3.5 0 1 0 0-7 3.5 3.5 0 0 0 0 7zM3 12h5.5M15.5 12H21',
    clock: 'M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18zM12 7v5l3 2',
    alert: 'M12 3.5l9.5 16.5h-19zM12 10v4.5M12 17.5v.01',
    tag: 'M3 12V4a1 1 0 0 1 1-1h8l9 9-9 9zM7.5 7.5v.01',
    cloud: 'M7 18a4.5 4.5 0 0 1-.6-8.96A6 6 0 0 1 18 8.5a4.5 4.5 0 0 1-.5 9.5z',
    user: 'M12 12a4 4 0 1 0 0-8 4 4 0 0 0 0 8zM4.5 20.5a7.5 7.5 0 0 1 15 0',
    download: 'M12 4v11M7 10.5l5 5 5-5M5 20h14',
    key: 'M14.5 3.5a6 6 0 1 1-4.9 9.46L3.5 19v2h3v-2h2v-2h2l1.04-1.04A6 6 0 0 1 14.5 3.5zM16.5 8v.01',
    external: 'M14 4h6v6M20 4l-9 9M18 14v5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V7a1 1 0 0 1 1-1h5',
    lock: 'M6 11h12a1 1 0 0 1 1 1v8a1 1 0 0 1-1 1H6a1 1 0 0 1-1-1v-8a1 1 0 0 1 1-1zM8 11V8a4 4 0 0 1 8 0v3',
    search: 'M11 18a7 7 0 1 0 0-14 7 7 0 0 0 0 14zM21 21l-5-5',
    folder: 'M3.5 6.5a1 1 0 0 1 1-1H9l2 2.5h8.5a1 1 0 0 1 1 1V18a1 1 0 0 1-1 1h-15a1 1 0 0 1-1-1z',
    logout: 'M15 4h3a1 1 0 0 1 1 1v14a1 1 0 0 1-1 1h-3M10 17l5-5-5-5M15 12H4',
    more: 'M5 12h.01M12 12h.01M19 12h.01',
    agent: 'M6 8.5h12a1.5 1.5 0 0 1 1.5 1.5v7.5a1.5 1.5 0 0 1-1.5 1.5H6A1.5 1.5 0 0 1 4.5 17.5V10A1.5 1.5 0 0 1 6 8.5zM12 8.5V5M12 4.5v.01M9.5 13v1M14.5 13v1M2.5 12.5v3M21.5 12.5v3',
    chevronDown: 'M6 9l6 6 6-6',
    archive: 'M3.5 4.5h17v4h-17zM5 8.5V19a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1V8.5M10 12.5h4',
    undo: 'M9 14L4 9l5-5M4 9h10.5a5.5 5.5 0 0 1 0 11H11',
    sync: 'M4 12a8 8 0 0 1 13.66-5.66L20 8.5M20 4v4.5h-4.5M20 12a8 8 0 0 1-13.66 5.66L4 15.5M4 20v-4.5h4.5',
  } as const
  export type IconName = keyof typeof PATHS
</script>

<script lang="ts">
  interface Props {
    name: IconName
    size?: number
    label?: string
  }
  let { name, size = 16, label }: Props = $props()
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width="1.9"
  stroke-linecap="round"
  stroke-linejoin="round"
  role={label ? 'img' : undefined}
  aria-label={label}
  aria-hidden={label ? undefined : 'true'}
>
  <path d={PATHS[name]} />
</svg>

<style>
  svg {
    flex: none;
  }
</style>
