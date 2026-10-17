// The app's own context menu (the browser's is disabled, see lockdown.ts).

export interface MenuAction {
  label: string
  action: () => unknown
  disabled?: boolean
  danger?: boolean
  /** Shown with a check mark (an on / off setting). */
  checked?: boolean
  /** Shortcut shown on the right, e.g. "Ctrl+B". */
  hint?: string
}

export type MenuItem = MenuAction | 'separator'

export interface OpenMenu {
  x: number
  y: number
  items: MenuItem[]
}

class MenuState {
  current = $state.raw<OpenMenu | null>(null)

  /** Opens a menu at the pointer (or below `anchor` when opened from a button / the keyboard). */
  open(e: MouseEvent | KeyboardEvent, items: MenuItem[], anchor?: HTMLElement) {
    e.preventDefault()
    e.stopPropagation()
    // Drop leading, trailing and doubled separators left by conditional items.
    const cleaned = items.filter((item, i, all) => item !== 'separator' || (i > 0 && i < all.length - 1 && all[i - 1] !== 'separator'))
    if (cleaned.length === 0) return
    let x = 0
    let y = 0
    const target = anchor ?? (e instanceof KeyboardEvent ? (e.currentTarget as HTMLElement | null) : null)
    if (target) {
      const r = target.getBoundingClientRect()
      x = r.left
      y = r.bottom + 2
    } else if (e instanceof MouseEvent) {
      x = e.clientX
      y = e.clientY
    }
    this.current = { x, y, items: cleaned }
  }

  close() {
    this.current = null
  }
}

export const menu = new MenuState()
