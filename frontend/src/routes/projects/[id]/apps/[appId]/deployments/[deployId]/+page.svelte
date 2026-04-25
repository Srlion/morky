<script>
    import { page } from "$app/state";
    import { onDestroy, untrack } from "svelte";
    import { api } from "$lib/api";
    import { toaster } from "$lib/components/Toasts.svelte";
    import StatusBadge from "$lib/components/StatusBadge.svelte";
    import { DeployStatus } from "$lib/status";
    import { globals } from "$lib/globals.svelte";
    import Convert from "ansi-to-html";

    const convert = new Convert({ escapeXML: true });

    const g = globals();
    let { data } = $props();

    let deployOverride = $state(null);
    const deployment = $derived(deployOverride ?? data.deployment);

    let log = $state("Waiting for logs...");
    let logHtml = $derived(convert.toHtml(log));
    let logEl = $state(undefined);
    let ws = null;
    let reconnectTimer = null;
    let wsKey = $state("");
    let copied = $state(false);

    let containerLines = $state([]);
    let containerTotal = $state(0);
    let containerLoaded = $state(false);
    let containerLoading = $state(false);
    let containerCopied = $state(false);

    const appId = $derived(page.params.appId);
    const deployId = $derived(page.params.deployId);
    const isDone = $derived(
        deployment &&
            [DeployStatus.FAILED, DeployStatus.DONE].includes(
                deployment.status,
            ),
    );
    const hasMoreLogs = $derived(containerLines.length < containerTotal);
    const liveStatus = $derived(
        g[`deploy_status_${deployment?.id}`] ?? deployment?.status,
    );

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

    // Reset override when navigating to a different deployment
    $effect(() => {
        data.deployment;
        deployOverride = null;
    });

    $effect(() => {
        if (!deployment?.id || !isValidId(appId) || !isValidId(deployment.id))
            return;
        untrack(() => {
            log = "Waiting for logs...";
            containerLines = [];
            containerTotal = 0;
            containerLoaded = false;
            loadBuildLog();
        });
    });

    $effect(() => {
        const key = `${appId}:${deployId}`;
        if (!isValidId(appId) || !isValidId(deployId) || isDone) {
            cleanupWs();
            wsKey = "";
            return;
        }
        if (wsKey !== key) {
            cleanupWs();
            wsKey = key;
            connectWs(appId, deployId, key);
        }
    });

    onDestroy(cleanupWs);

    function isValidId(v) {
        return Number.isInteger(Number(v)) && Number(v) > 0;
    }

    function scrollLog() {
        setTimeout(() => {
            if (logEl) logEl.scrollTop = logEl.scrollHeight;
        }, 0);
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

    async function loadBuildLog() {
        try {
            const res = await api.get(
                `/apps/${appId}/deployments/${deployId}/log`,
            );
            log = res.lines?.length
                ? Array.isArray(res.lines)
                    ? res.lines.join("\n") + "\n"
                    : res.lines
                : isDone
                  ? "No logs available for this deployment."
                  : log;
            scrollLog();
        } catch {
            log = "Failed to load logs.";
        }
    }

    function connectWs(aid, did, key) {
        const proto = location.protocol === "https:" ? "wss:" : "ws:";
        ws = new WebSocket(
            `${proto}//${location.host}/api/apps/${aid}/deployments/${did}/ws`,
        );
        ws.onmessage = (e) => {
            if (wsKey !== key) return;
            const msg = JSON.parse(e.data);
            if (msg.t === "bulk") {
                log = Array.isArray(msg.d) ? msg.d.join("\n") + "\n" : msg.d;
            } else if (msg.t === "line") {
                if (log === "Waiting for logs...") log = "";
                log += msg.d + "\n";
            } else if (msg.t === "status") {
                api.get(`/apps/${aid}/deployments/${did}`).then((d) => {
                    deployOverride = d;
                });
            }
            scrollLog();
        };
        ws.onclose = () => {
            if (wsKey === key && !isDone)
                reconnectTimer = setTimeout(
                    () => connectWs(aid, did, key),
                    1000,
                );
        };
    }

    function fmtTs(ts) {
        const d = new Date(ts * 1000);
        return (
            d.toLocaleDateString("en-US", {
                month: "2-digit",
                day: "2-digit",
                year: "2-digit",
            }) +
            " " +
            d.toLocaleTimeString("en-US", { hour12: true })
        );
    }

    function clipboardCopy(text, setter) {
        navigator.clipboard.writeText(text).then(() => {
            setter(true);
            setTimeout(() => setter(false), 2000);
        });
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

    async function cancel() {
        if (!confirm("Cancel this deployment?")) return;
        try {
            await api.post(`/apps/${appId}/deployments/${deployId}/cancel`);
        } catch (e) {
            toaster.error({ title: e.message });
        }
    }
</script>

{#snippet copyBtn(isCopied, onclick)}
    <button class="btn btn-xs btn-ghost gap-1" {onclick}>
        <span
            class="{isCopied
                ? 'icon-[lucide--check]'
                : 'icon-[lucide--copy]'} size-3"
        ></span>
        {isCopied ? "Copied" : "Copy"}
    </button>
{/snippet}

{#if deployment}
    <div class="flex items-center gap-3 mb-5">
        <h2 class="text-lg font-semibold">Deployment #{deployment.id}</h2>
        <StatusBadge status={liveStatus} />
        {#if liveStatus === DeployStatus.BUILDING}
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
                    {@render copyBtn(copied, () =>
                        clipboardCopy(log, (v) => (copied = v)),
                    )}
                </div>
                <pre
                    bind:this={logEl}
                    class="bg-base-200 border border-base-300 rounded-2xl p-4 text-xs font-mono max-h-[50vh] lg:max-h-[65vh] overflow-auto whitespace-pre-wrap break-all text-base-content/60">{@html logHtml}</pre>
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
                                <span class="loading loading-spinner loading-xs"
                                ></span>
                            {:else}
                                <span class="icon-[lucide--terminal] size-3"
                                ></span>
                            {/if}
                            Load Logs
                        </button>
                    {:else}
                        <div class="flex items-center gap-2">
                            <span class="text-[11px] text-base-content/40"
                                >{containerLines.length} / {containerTotal} lines</span
                            >
                            {@render copyBtn(containerCopied, () =>
                                clipboardCopy(
                                    containerLines
                                        .map(
                                            ([line, ts]) =>
                                                `${fmtTs(ts)}  ${line}`,
                                        )
                                        .join("\n"),
                                    (v) => (containerCopied = v),
                                ),
                            )}
                        </div>
                    {/if}
                </div>

                {#if containerLoaded}
                    <pre
                        class="bg-base-200 border border-base-300 rounded-2xl p-4 text-xs font-mono max-h-[50vh] lg:max-h-[65vh] overflow-auto whitespace-pre-wrap break-all text-base-content/60">{#each containerLines as [line, ts], i}<span
                                class="text-base-content/30 select-none"
                                >{fmtTs(ts)}  </span>{@html convert.toHtml(
                                line,
                            )}{#if i < containerLines.length - 1}{"\n"}{/if}{:else}No logs yet.{/each}</pre>

                    {#if hasMoreLogs}
                        <div class="mt-2 text-center">
                            <button
                                class="btn btn-xs btn-ghost"
                                onclick={loadMoreLogs}
                                disabled={containerLoading}
                            >
                                {#if containerLoading}
                                    <span
                                        class="loading loading-spinner loading-xs"
                                    ></span>
                                {:else}Load more{/if}
                            </button>
                        </div>
                    {/if}
                {/if}
            </div>
        </div>
    </div>
{/if}
