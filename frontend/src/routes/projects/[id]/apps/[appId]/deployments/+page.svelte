<script>
  import { page } from "$app/state";
  import { goto, invalidateAll } from "$app/navigation";
  import { api } from "$lib/api";
  import { toaster } from "$lib/components/Toasts.svelte";
  import StatusBadge from "$lib/components/StatusBadge.svelte";
  import { DeployStatus } from "$lib/status";
  import { globals } from "$lib/globals.svelte";

  const g = globals();

  let { data } = $props();
  let deployments = $derived(data.deployments);
  let app = $derived(data.app);
  let total = $derived(data.total);
  let currentPage = $derived(data.page);
  let perPage = $derived(data.perPage);
  let totalPages = $derived(Math.ceil(total / perPage));

  const statusFor = (d) => g[`deploy_status_${d?.id}`] ?? d?.status;

  const projectId = $derived(page.params.id);
  const appId = $derived(page.params.appId);
  const base = $derived(`/projects/${projectId}/apps/${appId}`);

  let currentDeploymentId = $derived(
    g[`app_current_deployment_${appId}`] ?? app?.current_deployment_id,
  );

  async function rollback(did) {
    if (!confirm("Rollback to this deployment?")) return;
    try {
      await api.post(`/apps/${appId}/rollback/${did}`);
      toaster.success({ title: "Rollback started" });
      await invalidateAll();
    } catch (e) {
      toaster.error({ title: e.message });
    }
  }

  function goPage(p) {
    goto(`${base}/deployments?page=${p}`, { invalidateAll: true });
  }
</script>

{#if deployments.length}
  <div class="flex flex-col gap-2">
    {#each deployments as d (d.id)}
      <div
        class="
          border rounded-2xl px-4 py-3 flex items-center gap-4 transition-all
          {currentDeploymentId === d.id
          ? 'bg-primary/10 border-primary/30'
          : 'bg-base-100 border-base-300 hover:shadow-md'}
        "
      >
        <a href="{base}/deployments/{d.id}" class="flex-1 min-w-0 no-underline">
          <div class="text-sm flex items-center gap-2">
            <span class="font-mono text-xs text-base-content/50"
              >{d.commit_sha?.slice(0, 7) || "-"}</span
            >
            <span class="truncate text-xs text-base-content/60"
              >{d.commit_message || ""}</span
            >
            {#if currentDeploymentId === d.id}
              <span class="badge badge-primary badge-sm text-[10px]"
                >current</span
              >
            {/if}
          </div>
          <div class="flex items-center gap-2 mt-1">
            <StatusBadge status={statusFor(d)} />
            <span class="text-[11px] text-base-content/40">
              {d.branch} · {d.build_method}
            </span>
          </div>
        </a>
        {#if d.image_exists && d.id !== currentDeploymentId && statusFor(d) !== DeployStatus.BUILDING}
          <button
            class="btn btn-xs btn-warning btn-outline"
            onclick={() => rollback(d.id)}
          >
            <span class="icon-[lucide--rotate-ccw] size-3"></span> Rollback
          </button>
        {/if}
      </div>
    {/each}
    {#if totalPages > 1}
      <div class="flex justify-center items-center gap-2 mt-4">
        <button
          class="btn btn-xs btn-ghost"
          disabled={currentPage <= 1}
          onclick={() => goPage(currentPage - 1)}
          aria-label="Previous page"
        >
          <span class="icon-[lucide--chevron-left] size-4"></span>
        </button>
        <span class="text-xs text-base-content/60"
          >{currentPage} / {totalPages}</span
        >
        <button
          class="btn btn-xs btn-ghost"
          disabled={currentPage >= totalPages}
          onclick={() => goPage(currentPage + 1)}
          aria-label="Next page"
        >
          <span class="icon-[lucide--chevron-right] size-4"></span>
        </button>
      </div>
    {/if}
  </div>
{:else}
  <div class="flex flex-col items-center justify-center py-16 text-center">
    <div class="bg-base-200 rounded-full p-4 mb-4">
      <span class="icon-[lucide--rocket] size-8 text-base-content/40"></span>
    </div>
    <p class="text-sm text-base-content/50 mb-1">No deployments yet</p>
    <p class="text-xs text-base-content/40">
      Hit Deploy to create your first deployment.
    </p>
  </div>
{/if}
