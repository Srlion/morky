<script>
  import { page } from "$app/state";
  import { globals } from "$lib/globals.svelte";

  const g = globals();

  let jobs = $derived.by(() => {
    const list = [];
    for (const [k, v] of Object.entries(g)) {
      if (k.startsWith("job_") && v && typeof v === "object" && v.status) {
        list.push(v);
      }
    }
    return list;
  });

  let active = $derived(
    jobs.filter((j) => j.status === "pending" || j.status === "running"),
  );

  let failed = $derived(jobs.filter((j) => j.status === "failed"));

  let expanded = $state(false);

  let currentAppId = $derived.by(() => {
    const m = page.url.pathname.match(/\/apps\/(\d+)/);
    return m ? Number(m[1]) : null;
  });

  function jobAppId(j) {
    return j.payload?.app_id ?? null;
  }

  function jobLabel(j) {
    return j.display || j.name.replace(/_/g, " ");
  }
</script>

{#if active.length > 0 || failed.length > 0}
  <div class="fixed bottom-4 left-1/2 -translate-x-1/2 z-40">
    <button
      class="
        btn btn-sm gap-2 shadow-lg {failed.length ? 'btn-error' : 'btn-primary'}
      "
      onclick={() => (expanded = !expanded)}
    >
      {#if active.length > 0}
        <span class="loading loading-spinner loading-xs"></span>
        {active.length} job{active.length === 1 ? "" : "s"}
      {/if}
      {#if failed.length > 0}
        <span class="icon-[lucide--circle-x] size-3.5"></span>
        {failed.length} failed
      {/if}
    </button>

    {#if expanded}
      <div
        class="absolute bottom-full left-1/2 -translate-x-1/2 mb-2 w-72 bg-base-100 border border-base-300 rounded-xl shadow-xl overflow-hidden"
      >
        <div class="px-3 py-2 border-b border-base-300 text-xs font-semibold">
          Background Jobs
        </div>
        <ul class="max-h-48 overflow-y-auto">
          {#each [...active, ...failed] as j (j.id)}
            {@const isCurrentApp =
              currentAppId != null && jobAppId(j) === currentAppId}
            <li
              class="
                px-3 py-2 flex items-center gap-2 text-xs border-b border-base-300/50 last:border-0
                {isCurrentApp ? 'bg-primary/10' : ''}
              "
            >
              {#if j.status === "running"}
                <span class="loading loading-spinner loading-xs text-primary"
                ></span>
              {:else if j.status === "pending"}
                <span class="icon-[lucide--clock] size-3.5 text-base-content/40"
                ></span>
              {:else if j.status === "failed"}
                <span class="icon-[lucide--circle-x] size-3.5 text-error"
                ></span>
              {/if}
              <span class="flex-1 truncate">{jobLabel(j)}</span>
              {#if isCurrentApp}
                <span class="badge badge-primary text-[10px]">this app</span>
              {/if}
              <span class="badge badge-ghost text-[10px]">{j.status}</span>
            </li>
          {/each}
        </ul>
      </div>
    {/if}
  </div>
{/if}
