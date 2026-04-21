<script>
  import { invalidateAll } from "$app/navigation";
  import { api } from "$lib/api";
  import { toaster } from "$lib/components/Toasts.svelte";
  import { fmtDate } from "$lib/util";
  import EnvVarsDialog, {
    envDialog,
  } from "$lib/components/EnvVarsDialog.svelte";

  let { data } = $props();
  let projects = $derived(data.projects);

  let search = $state("");
  let newEl = $state();
  let newName = $state("");
  let newDesc = $state("");

  let filtered = $derived(
    (search
      ? projects.filter((p) =>
          p.name.toLowerCase().includes(search.toLowerCase()),
        )
      : projects
    ).toSorted((a, b) => a.id - b.id),
  );

  async function create() {
    try {
      await api.post("/projects", {
        name: newName,
        description: newDesc || undefined,
      });
      toaster.success({ title: "Project created" });
      newEl.close();
      newName = "";
      newDesc = "";
      await invalidateAll();
    } catch (e) {
      toaster.error({ title: e.message });
    }
  }
</script>

<div class="flex items-center justify-between mb-6">
  <div>
    <h1 class="text-xl font-bold">Projects</h1>
    <p class="text-sm text-base-content/60 mt-0.5">Manage your deployments</p>
  </div>
  <button class="btn btn-sm btn-primary" onclick={() => newEl.showModal()}>
    <span class="icon-[lucide--plus] size-4"></span> New Project
  </button>
</div>

{#if projects.length}
  <div class="relative max-w-xs mb-5">
    <span
      class="icon-[lucide--search] size-4 absolute left-3 top-1/2 -translate-y-1/2 text-base-content/40 pointer-events-none z-10"
    ></span>
    <input
      class="input input-bordered text-sm pl-9 w-full"
      placeholder="Search projects..."
      bind:value={search}
    />
  </div>
  <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
    {#each filtered as p (p.id)}
      <div
        class="group card bg-base-100 border border-base-300 hover:shadow-md p-4 transition-all flex flex-col justify-between"
      >
        <a href="/projects/{p.id}" class="no-underline">
          <div class="flex items-center justify-between">
            <div class="font-semibold text-sm truncate">{p.name}</div>
            {#if p.app_count}
              <span class="badge badge-ghost text-[10px]"
                >{p.app_count} app{p.app_count === 1 ? "" : "s"}</span
              >
            {/if}
          </div>
          {#if p.description}
            <div class="text-xs text-base-content/60 mt-1.5 line-clamp-2">
              {p.description}
            </div>
          {/if}
        </a>
        <div
          class="mt-4 pt-3 border-t border-base-300 flex items-center justify-between"
        >
          <span class="text-[11px] text-base-content/40"
            >{fmtDate(p.created_at)}</span
          >
          <button
            class="btn btn-xs btn-ghost"
            onclick={() => envDialog.show(`/projects/${p.id}/env`, p.env_vars)}
          >
            Env
          </button>
        </div>
      </div>
    {/each}
  </div>
{:else}
  <div class="flex flex-col items-center justify-center py-24 text-center">
    <div class="bg-base-200 rounded-full p-4 mb-4">
      <span class="icon-[lucide--folder-open] size-8 text-base-content/40"
      ></span>
    </div>
    <p class="text-sm text-base-content/50 mb-1">No projects yet</p>
    <p class="text-xs text-base-content/40">
      Create your first project to get started.
    </p>
  </div>
{/if}

<!-- New Project Dialog -->
<dialog bind:this={newEl} class="modal">
  <div class="modal-box">
    <h3 class="text-lg font-semibold mb-4">New Project</h3>
    <div class="flex flex-col gap-4">
      <label class="flex flex-col gap-1">
        <span class="label">Name</span>
        <input
          class="input input-bordered w-full text-sm"
          bind:value={newName}
          placeholder="my-project"
        />
      </label>
      <label class="flex flex-col gap-1">
        <span class="label"
          >Description <span class="text-base-content/40">(optional)</span
          ></span
        >
        <textarea
          class="textarea textarea-bordered w-full text-sm"
          rows="2"
          bind:value={newDesc}
          placeholder="What is this project for?"
        ></textarea>
      </label>
      <div class="modal-action mt-0">
        <button class="btn btn-sm" onclick={() => newEl.close()}>Cancel</button>
        <button class="btn btn-sm btn-primary" onclick={create}>Create</button>
      </div>
    </div>
  </div>
  <form method="dialog" class="modal-backdrop"><button>close</button></form>
</dialog>

<EnvVarsDialog />
