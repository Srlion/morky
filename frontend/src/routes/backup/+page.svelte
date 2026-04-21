<script>
  import { invalidateAll } from "$app/navigation";
  import { api } from "$lib/api";
  import { toaster } from "$lib/components/Toasts.svelte";
  import { globals } from "$lib/globals.svelte";

  const g = globals();

  let { data } = $props();
  let status = $derived(data.status);
  let creating = $state(false);
  let jobId = $state(null);

  let jobStatus = $derived.by(() => {
    if (!jobId) return null;
    const j = g[`job_${jobId}`];
    return j?.status ?? null;
  });

  // when job finishes, refresh page data
  $effect(() => {
    if (jobStatus === "done" || jobStatus === "failed") {
      invalidateAll();
      creating = false;
    }
  });

  async function create() {
    creating = true;
    try {
      const res = await api.post("/backup/create");
      jobId = res.job_id;
      toaster.success({ title: "Backup started" });
    } catch (e) {
      toaster.error({ title: e.message });
      creating = false;
    }
  }

  function download() {
    window.open("/api/backup/download", "_blank");
  }

  let isRunning = $derived(
    creating || jobStatus === "pending" || jobStatus === "running",
  );
</script>

<div class="mb-6">
  <h1 class="text-xl font-bold">Backup</h1>
  <p class="text-sm text-base-content/60 mt-0.5">
    Download a full server backup (database + container volumes)
  </p>
</div>

<div class="card bg-base-100 border border-base-300 p-5 rounded-2xl max-w-lg">
  <div class="flex flex-col gap-4">
    <div class="text-sm text-base-content/60">
      This will pause all running containers, export the SQLite database and all
      container volumes, then package everything into a downloadable zip.
      Containers are automatically unpaused after the backup completes.
    </div>

    {#if status.job_status === "failed" && status.error}
      <div role="alert" class="alert alert-error text-xs">
        <span class="icon-[lucide--circle-x] size-4 shrink-0"></span>
        <span>Last backup failed: {status.error}</span>
      </div>
    {/if}

    {#if status.job_status === "running" || status.job_status === "pending"}
      <div role="alert" class="alert alert-info text-xs">
        <span class="loading loading-spinner loading-xs"></span>
        <span>A backup is currently in progress...</span>
      </div>
    {/if}

    <div class="flex items-center gap-3">
      <button
        class="btn btn-sm btn-primary"
        onclick={create}
        disabled={isRunning}
      >
        {#if isRunning}
          <span class="loading loading-spinner loading-xs"></span> Creating...
        {:else}
          <span class="icon-[lucide--archive] size-3.5"></span> Create Backup
        {/if}
      </button>

      {#if status.has_backup}
        <button class="btn btn-sm btn-outline" onclick={download}>
          <span class="icon-[lucide--download] size-3.5"></span> Download
        </button>
      {/if}
    </div>

    {#if status.has_backup && status.job_status === "done"}
      <div class="text-xs text-success flex items-center gap-1">
        <span class="icon-[lucide--circle-check] size-3.5"></span>
        Backup ready for download
      </div>
    {/if}
  </div>
</div>
