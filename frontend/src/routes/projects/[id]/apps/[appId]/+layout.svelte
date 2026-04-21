<script>
  import { page } from "$app/state";
  import { goto, invalidateAll } from "$app/navigation";
  import { api } from "$lib/api";
  import { toaster } from "$lib/components/Toasts.svelte";
  import EnvVarsDialog, {
    envDialog,
  } from "$lib/components/EnvVarsDialog.svelte";
  import StatusBadge from "$lib/components/StatusBadge.svelte";
  import { AppStatus } from "$lib/status";
  import { globals } from "$lib/globals.svelte";

  const g = globals();

  let { data, children } = $props();
  let app = $derived(data.app);
  let project = $derived(data.project);

  let status = $derived(g[`app_status_${app?.id}`] ?? app?.status);
  let isBuilding = $derived(g[`app_deploy_status_${app?.id}`] === "building");

  let currentDeploymentId = $derived(
    g[`app_current_deployment_${app?.id}`] ?? app?.current_deployment_id,
  );

  const projectId = $derived(page.params.id);
  const appId = $derived(page.params.appId);
  const base = $derived(`/projects/${projectId}/apps/${appId}`);
  const currentTab = $derived(
    page.url.pathname.includes("/deployments")
      ? "deployments"
      : page.url.pathname.includes("/logs")
        ? "logs"
        : "settings",
  );

  async function deploy() {
    try {
      const res = await api.post(`/apps/${appId}/deploy`);
      toaster.success({ title: "Deployment started" });
      goto(`${base}/deployments/${res.deployment_id}`);
    } catch (e) {
      toaster.error({ title: e.message });
    }
  }

  async function stop() {
    if (!confirm("Stop this app?")) return;
    try {
      await api.post(`/apps/${appId}/stop`);
      toaster.error({ title: "Stopping app..." });
      await invalidateAll();
    } catch (e) {
      toaster.error({ title: e.message });
    }
  }

  async function start() {
    try {
      await api.post(`/apps/${appId}/start`);
      toaster.success({ title: "Starting app..." });
      await invalidateAll();
    } catch (e) {
      toaster.error({ title: e.message });
    }
  }

  async function cancelDeploy() {
    if (!confirm("Cancel the current deployment?")) return;
    try {
      const res = await api.get(`/apps/${appId}/deployments?per_page=1`);
      const latest = res.items[0];
      if (!latest) return toaster.error({ title: "No deployment found" });
      await api.post(`/apps/${appId}/deployments/${latest.id}/cancel`);
      toaster.success({ title: "Canceling deployment..." });
      await invalidateAll();
    } catch (e) {
      toaster.error({ title: e.message });
    }
  }

  const tabs = [
    { id: "settings", path: "settings", label: "Settings" },
    { id: "deployments", path: "deployments", label: "Deployments" },
    { id: "logs", path: "logs", label: "Logs" },
  ];
</script>

{#if app}
  <nav class="text-xs breadcrumbs py-0 mb-4">
    <ul>
      <li><a href="/projects" class="link link-primary">Projects</a></li>
      <li>
        <a href="/projects/{projectId}" class="link link-primary"
          >{project.name}</a
        >
      </li>
      <li>{app.name}</li>
    </ul>
  </nav>

  <div
    class="flex flex-col sm:flex-row sm:items-center justify-between gap-3 mb-5"
  >
    <div>
      <div class="flex items-center gap-2.5">
        <h1 class="text-xl font-bold">{app.name}</h1>
        <StatusBadge {status} />
        {#if isBuilding}
          <StatusBadge status="building" />
        {/if}
      </div>
      {#if app.repo}
        <p class="text-sm text-base-content/60 mt-0.5 font-mono">{app.repo}</p>
      {/if}
    </div>
    <div class="flex gap-2">
      {#if isBuilding}
        <button class="btn btn-sm btn-error btn-outline" onclick={cancelDeploy}>
          <span class="icon-[lucide--x] size-3.5"></span> Cancel
        </button>
      {/if}

      {#if status === AppStatus.RUNNING}
        <button
          class="btn btn-sm btn-error btn-outline"
          onclick={stop}
          disabled={isBuilding}
        >
          <span class="icon-[lucide--square] size-3.5"></span> Stop
        </button>
      {:else if (status === AppStatus.IDLE || status === AppStatus.FAILED) && currentDeploymentId && !isBuilding}
        <button class="btn btn-sm btn-success btn-outline" onclick={start}>
          <span class="icon-[lucide--play] size-3.5"></span> Start
        </button>
      {/if}

      <button
        class="btn btn-sm btn-primary"
        onclick={deploy}
        disabled={isBuilding}
      >
        {#if isBuilding}
          <span class="loading loading-spinner size-3.5"></span> Building...
        {:else}
          <span class="icon-[lucide--rocket] size-3.5"></span> Deploy
        {/if}
      </button>
      <button
        class="btn btn-sm btn-ghost"
        onclick={() =>
          envDialog.show(`/apps/${appId}/env`, app.env_vars, project.env_vars)}
      >
        <span class="icon-[lucide--variable] size-3.5"></span> Env
      </button>
    </div>
  </div>

  <div role="tablist" class="tabs tabs-bordered mb-6">
    {#each tabs as t}
      <a
        role="tab"
        href="{base}/{t.path}"
        class="tab {currentTab === t.id ? 'tab-active' : ''}">{t.label}</a
      >
    {/each}
  </div>

  {@render children()}
{/if}

<EnvVarsDialog />
