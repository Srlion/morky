<script>
  import { invalidateAll } from "$app/navigation";
  import { onMount } from "svelte";
  import { api } from "$lib/api";
  import { toaster } from "$lib/components/Toasts.svelte";
  import { page } from "$app/state";
  import GitSourceCard from "./GitSourceCard.svelte";

  let { data } = $props();
  let sources = $derived(
    (data.sources ?? []).slice().sort((a, b) => Number(a.id) - Number(b.id)),
  );

  let renameId = $state(0);
  let renameName = $state("");
  let renameEl = $state();
  let orgEl = $state();
  let orgName = $state("");

  async function del(id) {
    if (!confirm("Delete this source?")) return;
    const source = sources.find((s) => s.id === id);
    const url =
      source?.provider === "github"
        ? `/git-sources/github/${id}`
        : `/git-sources/${id}`;
    try {
      await api.del(url);
      toaster.success({ title: "Source deleted" });
      await invalidateAll();
    } catch (e) {
      toaster.error({ title: e.message });
    }
  }

  function openRename(s) {
    renameId = s.id;
    renameName = s.name;
    renameEl.showModal();
  }

  async function doRename() {
    try {
      await api.put(`/git-sources/${renameId}/name`, {
        name: renameName,
      });
      toaster.success({ title: "Renamed" });
      renameEl.close();
      await invalidateAll();
    } catch (e) {
      toaster.error({ title: e.message });
    }
  }

  onMount(() => {
    const err = new URL(page.url).searchParams.get("error");
    if (err) {
      toaster.error({ title: err });
      history.replaceState(null, "", "/git-sources");
    }
  });
</script>

<div class="flex items-center justify-between mb-6">
  <div>
    <h1 class="text-xl font-bold">Git Sources</h1>
    <p class="text-sm text-base-content/60 mt-0.5">
      Manage your connected repositories
    </p>
  </div>
  <div class="dropdown dropdown-end">
    <div tabindex="0" role="button" class="btn btn-sm btn-primary">
      <span class="icon-[lucide--plus] size-4"></span> Add Source
    </div>
    <ul
      tabindex="-1"
      class="dropdown-content menu bg-base-100 rounded-box z-10 w-52 p-2 shadow-lg border border-base-300 mt-1"
    >
      <li>
        <a href="/git-sources/github/new" class="no-underline text-sm">
          GitHub (Personal)
        </a>
      </li>
      <li>
        <button class="text-sm" onclick={() => orgEl.showModal()}>
          GitHub (Organization)
        </button>
      </li>
    </ul>
  </div>
</div>

{#if sources.length}
  <div role="alert" class="alert alert-warning mb-6 text-xs">
    <span class="icon-[lucide--triangle-alert] size-4 shrink-0"></span>
    <span
      >You can rename this source here, but do not rename the GitHub App on
      GitHub or the connection will break.
    </span>
  </div>

  <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
    {#each sources as s (s.id)}
      <GitSourceCard source={s} onRename={openRename} onDelete={del} />
    {/each}
  </div>
{:else}
  <div class="flex flex-col items-center justify-center py-24 text-center">
    <div class="bg-base-200 rounded-full p-4 mb-4">
      <span class="icon-[lucide--git-branch] size-8 text-base-content/40"
      ></span>
    </div>
    <p class="text-sm text-base-content/50 mb-1">No git sources yet</p>
    <p class="text-xs text-base-content/40">Add one to get started.</p>
  </div>
{/if}

<!-- Rename Dialog -->
<dialog bind:this={renameEl} class="modal">
  <div class="modal-box">
    <h3 class="text-lg font-semibold mb-4">Rename Source</h3>
    <label class="flex flex-col gap-1">
      <span class="label">Name</span>
      <input
        class="input input-bordered w-full text-sm"
        bind:value={renameName}
      />
    </label>
    <div class="modal-action">
      <button class="btn btn-sm" onclick={() => renameEl.close()}>
        Cancel
      </button>
      <button class="btn btn-sm btn-primary" onclick={doRename}>Save</button>
    </div>
  </div>
  <form method="dialog" class="modal-backdrop"><button>close</button></form>
</dialog>

<!-- Org Dialog -->
<dialog bind:this={orgEl} class="modal">
  <div class="modal-box">
    <h3 class="text-lg font-semibold mb-4">GitHub Organization</h3>
    <label class="flex flex-col gap-1">
      <span class="label">Organization name</span>
      <input
        class="input input-bordered w-full text-sm"
        bind:value={orgName}
        placeholder="my-org"
      />
    </label>
    <div class="modal-action">
      <button class="btn btn-sm" onclick={() => orgEl.close()}>Cancel</button>
      <a
        href="/git-sources/github/new?org={orgName}"
        class="btn btn-sm btn-primary no-underline">Continue</a
      >
    </div>
  </div>
  <form method="dialog" class="modal-backdrop"><button>close</button></form>
</dialog>
