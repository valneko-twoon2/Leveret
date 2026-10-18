<!-- Dialogs, menus and toasts shown above the main layout, each while its store asks for it. -->
<script lang="ts">
  import { agents } from '../agents.svelte'
  import { blame } from '../blame.svelte'
  import { branches } from '../branches.svelte'
  import { github } from '../github.svelte'
  import { issues } from '../issues.svelte'
  import { rewrite } from '../rewrite.svelte'
  import { settings } from '../settings.svelte'
  import { app } from '../state.svelte'
  import { tags } from '../tags.svelte'
  import AgentDialog from './AgentDialog.svelte'
  import BlameDialog from './BlameDialog.svelte'
  import BranchesDialog from './BranchesDialog.svelte'
  import CloneDialog from './CloneDialog.svelte'
  import ContextMenu from './ContextMenu.svelte'
  import ErrorDialog from './ErrorDialog.svelte'
  import IssueEditor from './IssueEditor.svelte'
  import NewBranchDialog from './NewBranchDialog.svelte'
  import NewTagDialog from './NewTagDialog.svelte'
  import PerfPanel from './PerfPanel.svelte'
  import ResetDialog from './ResetDialog.svelte'
  import SettingsDialog from './settings/SettingsDialog.svelte'
  import SignInDialog from './SignInDialog.svelte'
  import Toasts from './Toasts.svelte'
</script>

{#if branches.branchesOpen}
  <BranchesDialog />
{/if}
{#if branches.newBranch}
  <NewBranchDialog />
{/if}

{#if rewrite.reset}
  <ResetDialog />
{/if}
{#if blame.path}
  <BlameDialog />
{/if}
{#if tags.newTag}
  <NewTagDialog />
{/if}
{#if issues.draft}
  <IssueEditor />
{/if}
{#if settings.open}
  <SettingsDialog />
{/if}
{#if agents.agentOpen}
  <AgentDialog />
{/if}
{#if github.cloneOpen}
  <CloneDialog />
{/if}
{#if github.signInOpen}
  <SignInDialog />
{/if}
{#if app.perfOpen}
  <PerfPanel />
{/if}
<ErrorDialog />
<ContextMenu />
<Toasts />
