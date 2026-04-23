<script>
  import { page } from "$app/state";
  import { onDestroy } from "svelte";
  import { api } from "$lib/api";
  import { toaster } from "$lib/components/Toasts.svelte";
  import StatusBadge from "$lib/components/StatusBadge.svelte";
  import { DeployStatus } from "$lib/status";
  import { globals } from "$lib/globals.svelte";

  const g = globals();

  let { data } = $props();
  let deployment = $state();
  let log = $state("Waiting for logs...");
  let logEl = $state(undefined);
  let ws = null;
  let reconnectTimer = null;
  let wsKey = $state("");
  let copied = $state(false);

  // Container logs
  let containerLines = $state([]);
  let containerTotal = $state(0);
  let containerLoaded = $state(false);
  let containerLoading = $state(false);

  const projectId = $derived(page.params.id);
  const appId = $derived(page.params.appId);
  const deployId = $derived(page.params.deployId);
  const isDone = $derived(
    deployment &&
      [DeployStatus.FAILED, DeployStatus.DONE].includes(deployment.status),
  );
  const hasMoreLogs = $derived(containerLines.length < containerTotal);
  const meta = $derived(
    deployment
      ? [
          {
            label: "Commit",
            value: deployment.commit_sha?.slice(0, 7) || "-",
          },
          { label: "Branch", value: deployment.branch || "-" },
          {
            label: "Build Method",
            value: deployment.build_method || "-",
          },
          { label: "Image", value: deployment.image_tag || "-" },
        ]
      : [],
  );

  $effect(() => {
    deployment = data.deployment;
    log = data.deployment?.build_log || "Waiting for logs...";
    // Reset container logs when deployment changes
    containerLines = [];
    containerTotal = 0;
    containerLoaded = false;
  });

  $effect(() => {
    const aid = appId;
    const did = deployId;
    const key = `${aid}:${did}`;
    const valid = isValidId(aid) && isValidId(did);

    if (!valid || isDone) {
      cleanupWs();
      wsKey = "";
      return;
    }

    if (wsKey !== key) {
      cleanupWs();
      wsKey = key;
      connectWs(aid, did, key);
    }
  });

  onDestroy(cleanupWs);

  function status() {
    return g[`deploy_status_${deployment?.id}`] ?? deployment?.status;
  }

  function scrollLog() {
    if (logEl) logEl.scrollTop = logEl.scrollHeight;
  }

  function copyLog() {
    navigator.clipboard.writeText(log).then(() => {
      copied = true;
      setTimeout(() => (copied = false), 2000);
    });
  }

  function isValidId(value) {
    const n = Number(value);
    return Number.isInteger(n) && n > 0;
  }

  function cleanupWs() {
    if (reconnectTimer) {
      clearTimeout(reconnectTimer);
      reconnectTimer = null;
    }
    if (ws) {
      ws.onclose = null;
      ws.close();
      ws = null;
    }
  }

  function connectWs(aid, did, key) {
    if (!isValidId(aid) || !isValidId(did)) return;

    const proto = location.protocol === "https:" ? "wss:" : "ws:";
    ws = new WebSocket(
      `${proto}//${location.host}/api/apps/${aid}/deployments/${did}/ws`,
    );

    ws.onmessage = (e) => {
      if (wsKey !== key) return;

      const msg = JSON.parse(e.data);
      if (msg.t === "bulk") {
        log = msg.d;
      } else if (msg.t === "line") {
        if (log === "Waiting for logs...") log = "";
        log += msg.d + "\n";
      } else if (msg.t === "status") {
        api
          .get(`/apps/${aid}/deployments/${did}`)
          .then((d) => (deployment = d));
      }

      scrollLog();
    };

    ws.onclose = () => {
      const stillSameKey = wsKey === key && `${appId}:${deployId}` === key;
      const idsStillValid = isValidId(appId) && isValidId(deployId);

      if (stillSameKey && idsStillValid && !isDone) {
        reconnectTimer = setTimeout(() => connectWs(aid, did, key), 1000);
      }
    };
  }

  async function loadContainerLogs() {
    containerLoading = true;
    try {
      const res = await api.get(
        `/apps/${appId}/deployments/${deployId}/container-log?offset=0&limit=500`,
      );
      containerLines = res.lines;
      containerTotal = res.total;
      containerLoaded = true;
    } catch (e) {
      toaster.error({ title: e.message });
    } finally {
      containerLoading = false;
    }
  }

  async function loadMoreLogs() {
    containerLoading = true;
    try {
      const res = await api.get(
        `/apps/${appId}/deployments/${deployId}/container-log?offset=${containerLines.length}&limit=500`,
      );
      containerLines = [...containerLines, ...res.lines];
      containerTotal = res.total;
    } catch (e) {
      toaster.error({ title: e.message });
    } finally {
      containerLoading = false;
    }
  }

  let containerCopied = $state(false);

  function copyContainerLog() {
    const text = containerLines
      .map(([line, ts]) => {
        const d = new Date(ts * 1000);
        return `${d.toLocaleDateString("en-US", {
          month: "2-digit",
          day: "2-digit",
          year: "2-digit",
        })} ${d.toLocaleTimeString("en-US", { hour12: true })}  ${line}`;
      })
      .join("\n");
    navigator.clipboard.writeText(text).then(() => {
      containerCopied = true;
      setTimeout(() => (containerCopied = false), 2000);
    });
  }

  async function cancel() {
    if (!confirm("Cancel this deployment?")) return;
    try {
      await api.post(`/apps/${appId}/deployments/${deployId}/cancel`);
    } catch (e) {
      toaster.error({ title: e.message });
    }
  }
</script>

{#if deployment}
  <div class="flex items-center gap-3 mb-5">
    <h2 class="text-lg font-semibold">Deployment #{deployment.id}</h2>
    <StatusBadge status={status()} />
    {#if status() === DeployStatus.BUILDING}
      <button class="btn btn-xs btn-error btn-outline" onclick={cancel}>
        <span class="icon-[lucide--x] size-3"></span> Cancel
      </button>
    {/if}
  </div>

  {#if deployment.error}
    <div role="alert" class="alert alert-error text-sm mb-4">
      {deployment.error}
    </div>
  {/if}

  <div class="flex flex-col lg:flex-row gap-4">
    <!-- Metadata sidebar -->
    <div
      class="flex flex-col sm:flex-row lg:flex-col gap-3 lg:w-48 lg:shrink-0"
    >
      {#each meta as { label, value }}
        <div
          class="card bg-base-100 border border-base-300 rounded-2xl p-3 sm:flex-1 lg:flex-none"
        >
          <div class="text-[11px] text-base-content/40 mb-1">
            {label}
          </div>
          <div class="text-xs font-mono truncate">{value}</div>
        </div>
      {/each}
      {#if deployment.commit_message}
        <p class="text-[11px] text-base-content/40 leading-relaxed">
          {deployment.commit_message}
        </p>
      {/if}
    </div>

    <div class="flex-1 min-w-0 flex flex-col gap-4">
      <!-- Build log -->
      <div>
        <div class="flex items-center justify-between mb-2">
          <h3 class="text-sm font-semibold">Build Log</h3>
          <button class="btn btn-xs btn-ghost gap-1" onclick={copyLog}>
            <span
              class="{copied
                ? 'icon-[lucide--check]'
                : 'icon-[lucide--copy]'} size-3"
            ></span>
            {copied ? "Copied" : "Copy"}
          </button>
        </div>
        <pre
          bind:this={logEl}
          class="bg-base-200 border border-base-300 rounded-2xl p-4 text-xs font-mono max-h-[50vh] lg:max-h-[65vh] overflow-auto whitespace-pre-wrap break-all text-base-content/60">{log}</pre>
      </div>

      <!-- Container logs -->
      <div>
        <div class="flex items-center justify-between mb-2">
          <h3 class="text-sm font-semibold">Container Logs</h3>
          {#if !containerLoaded}
            <button
              class="btn btn-xs btn-outline gap-1"
              onclick={loadContainerLogs}
              disabled={containerLoading}
            >
              {#if containerLoading}
                <span class="loading loading-spinner loading-xs"></span>
              {:else}
                <span class="icon-[lucide--terminal] size-3"></span>
              {/if}
              Load Logs
            </button>
          {:else}
            <div class="flex items-center gap-2">
              <span class="text-[11px] text-base-content/40">
                {containerLines.length} / {containerTotal} lines
              </span>
              <button
                class="btn btn-xs btn-ghost gap-1"
                onclick={copyContainerLog}
              >
                <span
                  class="{containerCopied
                    ? 'icon-[lucide--check]'
                    : 'icon-[lucide--copy]'} size-3"
                ></span>
                {containerCopied ? "Copied" : "Copy"}
              </button>
            </div>
          {/if}
        </div>

        {#if containerLoaded}
          <pre
            class="bg-base-200 border border-base-300 rounded-2xl p-4 text-xs font-mono max-h-[50vh] lg:max-h-[65vh] overflow-auto whitespace-pre-wrap break-all text-base-content/60">{#each containerLines as entry}{@const d =
                new Date(entry[1] * 1000)}<span
                class="text-base-content/30 select-none"
                >{d.toLocaleDateString("en-US", {
                  month: "2-digit",
                  day: "2-digit",
                  year: "2-digit",
                })} {d.toLocaleTimeString("en-US", {
                  hour12: true,
                })}  </span>{entry[0]}
            {:else}No logs yet.{/each}</pre>

          {#if hasMoreLogs}
            <div class="mt-2 text-center">
              <button
                class="btn btn-xs btn-ghost"
                onclick={loadMoreLogs}
                disabled={containerLoading}
              >
                {#if containerLoading}
                  <span class="loading loading-spinner loading-xs"></span>
                {:else}
                  Load more
                {/if}
              </button>
            </div>
          {/if}
        {/if}
      </div>
    </div>
  </div>
{/if}
