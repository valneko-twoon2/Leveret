import { mount } from 'svelte'
import './app.css'
import App from './App.svelte'
import { inTauri } from './lib/api'
import { installLockdown } from './lib/lockdown'

// Block browser behaviour before anything renders. F12 stays usable in the
// browser preview and in `tauri dev` (the backend still decides whether
// devtools exist at all).
installLockdown({ devtools: !inTauri || import.meta.env.DEV })

const app = mount(App, {
  target: document.getElementById('app')!,
})

export default app
