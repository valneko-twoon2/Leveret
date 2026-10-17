// Japanese messages. This catalog defines the message keys; other locales must
// provide the same shape (see `Messages` in ../index.svelte.ts).
// Placeholders are written as {name}. Sections live in ./ja/ by area.
import app from './ja/app'
import changes from './ja/changes'
import history from './ja/history'
import branches from './ja/branches'
import tags from './ja/tags'
import remote from './ja/remote'
import issues from './ja/issues'
import agent from './ja/agent'
import errors from './ja/errors'
import settings from './ja/settings'

export default {
  ...app,
  ...changes,
  ...history,
  ...branches,
  ...tags,
  ...remote,
  ...issues,
  ...agent,
  ...errors,
  ...settings,
}
