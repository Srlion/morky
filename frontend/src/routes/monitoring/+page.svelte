<script>
    import { onDestroy, onMount } from "svelte";
    import { api } from "$lib/api";
    import LineChart from "$lib/components/LineChart.svelte";
    import { toaster } from "$lib/components/Toasts.svelte";
    import { fmtRelativeTime } from "$lib/util";

    let { data } = $props();
    let stats = $state([]);
    let containers = $state([]);
    let podmanDisk = $state(null);
    let buildkitBytes = $state(0);
    let sortCol = $state("cpu");
    let sortAsc = $state(false);
    let page = $state(0);
    let perPage = $state(30);
    const perPageOpts = [10, 20, 30, 50, 100];
    let search = $state("");
    let statsTimer, procTimer;

    // cleanup state
    let cleanupSettings = $state(null);
    let cleanupSaving = $state(false);
    let cleaning = $state("");
    let cleanupDirty = $derived(
        cleanupSettings && data.cleanupSettings
            ? JSON.stringify(cleanupSettings) !==
                  JSON.stringify(data.cleanupSettings)
            : false,
    );

    const intervalOpts = [
        { value: 1, label: "Every hour" },
        { value: 6, label: "Every 6 hours" },
        { value: 12, label: "Every 12 hours" },
        { value: 24, label: "Every 24 hours" },
        { value: 48, label: "Every 2 days" },
        { value: 168, label: "Every week" },
    ];
    const cleanupTargets = [
        {
            key: "containers",
            label: "Containers",
            icon: "icon-[lucide--container]",
        },
        { key: "images", label: "Images", icon: "icon-[lucide--image]" },
        { key: "volumes", label: "Volumes", icon: "icon-[lucide--database]" },
        { key: "buildkit", label: "BuildKit", icon: "icon-[lucide--hammer]" },
    ];

    $effect(() => {
        stats = data.stats ?? [];
        podmanDisk = data.podmanDisk ?? null;
        cleanupSettings = data.cleanupSettings
            ? { ...data.cleanupSettings }
            : null;
    });
    $effect(() => {
        sortCol;
        sortAsc;
        perPage;
        search;
        page = 0;
    });

    function savePerPage(v) {
        try {
            localStorage.setItem("mon_perPage", v);
        } catch {}
    }

    let latest = $derived(stats.length ? stats[stats.length - 1] : null);

    let filtered = $derived(() => {
        let rows = containers
            .map((c) => ({
                name: c.name,
                displayName: c.app_name ?? c.name,
                app_id: c.app_id,
                app_name: c.app_name,
                project_id: c.project_id,
                project_name: c.project_name,
                cpu: parseFloat(c.cpu) || 0,
                cpuDisplay: c.cpu,
                mem: parseFloat(c.mem_percent) || 0,
                memDisplay: `${c.mem_used} (${c.mem_percent})`,
            }))
            .filter(
                (r) =>
                    !search ||
                    r.displayName
                        .toLowerCase()
                        .includes(search.toLowerCase()) ||
                    r.project_name
                        ?.toLowerCase()
                        .includes(search.toLowerCase()),
            );
        const dir = sortAsc ? 1 : -1;
        rows.sort((a, b) => {
            if (sortCol === "name")
                return dir * a.displayName.localeCompare(b.displayName);
            if (sortCol === "cpu") return dir * (a.cpu - b.cpu);
            if (sortCol === "mem") return dir * (a.mem - b.mem);
            return 0;
        });
        return rows;
    });

    let totalPages = $derived(
        Math.max(1, Math.ceil(filtered().length / perPage)),
    );
    let paged = $derived(
        filtered().slice(page * perPage, (page + 1) * perPage),
    );

    let timeLabels = $derived(
        stats.map((_, i) => {
            const s = (stats.length - 1 - i) * 2;
            return s === 0 ? "now" : `-${s}s`;
        }),
    );
    let cpuDatasets = $derived([
        {
            label: "CPU",
            data: stats.map((s) => s.cpu_percent),
            color: "#3b82f6",
        },
    ]);
    let memDatasets = $derived([
        {
            label: "Memory",
            data: stats.map((s) => s.mem_percent),
            color: "#10b981",
        },
    ]);
    let systemCpu = $state(0);

    onMount(() => {
        try {
            const saved = parseInt(localStorage.getItem("mon_perPage"));
            if (perPageOpts.includes(saved)) perPage = saved;
        } catch {}
        refreshProcs();
        statsTimer = setInterval(refreshStats, 2000);
        procTimer = setInterval(refreshProcs, 5000);
    });
    onDestroy(() => {
        clearInterval(statsTimer);
        clearInterval(procTimer);
    });

    async function refreshStats() {
        try {
            stats = await api.get("/monitoring/stats");
        } catch {}
    }
    async function refreshProcs() {
        try {
            const res = await api.get("/monitoring/processes");
            containers = res.containers ?? [];
            systemCpu = res.system_cpu ?? 0;
            const diskRes = await api.get("/monitoring/podman-disk");
            podmanDisk = diskRes.podman ?? null;
            buildkitBytes = diskRes.buildkit_bytes ?? 0;
        } catch {}
    }

    function toggleSort(col) {
        if (sortCol === col) sortAsc = !sortAsc;
        else {
            sortCol = col;
            sortAsc = col === "name";
        }
    }
    function fmtMem(mib) {
        return !mib
            ? "0M"
            : mib >= 1024
              ? `${(mib / 1024).toFixed(2)}G`
              : `${mib}M`;
    }
    function fmtDisk(gib) {
        return gib ? `${gib.toFixed(1)}G` : "0G";
    }
    function pctColor(p) {
        return p >= 90 ? "text-error" : p >= 70 ? "text-warning" : "";
    }
    function barColor(p, base) {
        return p >= 90 ? "bg-error" : p >= 70 ? "bg-warning" : base;
    }

    // cleanup actions
    async function runCleanup(targets) {
        const key =
            Object.keys(targets)
                .filter((k) => targets[k])
                .join("+") || "run";
        cleaning = key;
        try {
            const res = await api.post("/monitoring/cleanup/run", {
                ...targets,
                buildkit_keep_storage_gb: 0,
            });
            if (res.success) {
                toaster.success({
                    title: `Cleaned: ${res.targets.join(", ")}`,
                });
            } else {
                toaster.error({ title: res.error || "Cleanup failed" });
            }
            await Promise.all([refreshCleanupSettings(), refreshProcs()]);
        } catch (e) {
            toaster.error({ title: e.message });
        } finally {
            cleaning = "";
        }
    }

    async function saveCleanupSettings() {
        cleanupSaving = true;
        try {
            await api.put("/monitoring/cleanup/settings", {
                ...cleanupSettings,
                buildkit_keep_storage_gb: 0,
            });
            toaster.success({ title: "Cleanup settings saved" });
            await refreshCleanupSettings();
        } catch (e) {
            toaster.error({ title: e.message });
        } finally {
            cleanupSaving = false;
        }
    }

    async function refreshCleanupSettings() {
        try {
            const s = await api.get("/monitoring/cleanup/settings");
            data.cleanupSettings = s;
            cleanupSettings = { ...s };
        } catch {}
    }
</script>

<div class="flex items-center justify-between mb-6">
    <div>
        <h1 class="text-xl font-bold">Monitoring</h1>
        <p class="text-sm text-base-content/60 mt-0.5">
            System resource usage - updates every 2 s
        </p>
    </div>
</div>

<!-- CPU & Memory charts -->
<div class="grid grid-cols-1 lg:grid-cols-2 gap-4 mb-4">
    <div
        class="card bg-base-100 border border-base-300 rounded-2xl overflow-hidden"
    >
        <div class="flex items-center justify-between px-5 pt-4 pb-2">
            <div class="flex items-center gap-3">
                <div class="bg-blue-500/10 text-blue-500 rounded-xl p-2">
                    <span class="icon-[lucide--cpu] size-4"></span>
                </div>
                <span class="text-sm font-medium text-base-content/60">CPU</span
                >
            </div>
            <span
                class="text-2xl font-bold tabular-nums {pctColor(
                    latest?.cpu_percent ?? 0,
                )}"
            >
                {latest?.cpu_percent ?? 0}<span
                    class="text-sm font-normal text-base-content/40">%</span
                >
            </span>
        </div>
        <div class="px-5 pb-4">
            <LineChart
                labels={timeLabels}
                datasets={cpuDatasets}
                yMax={100}
                yUnit="%"
                height="180px"
            />
        </div>
    </div>

    <div
        class="card bg-base-100 border border-base-300 rounded-2xl overflow-hidden"
    >
        <div class="flex items-center justify-between px-5 pt-4 pb-2">
            <div class="flex items-center gap-3">
                <div class="bg-emerald-500/10 text-emerald-500 rounded-xl p-2">
                    <span class="icon-[lucide--memory-stick] size-4"></span>
                </div>
                <span class="text-sm font-medium text-base-content/60"
                    >Memory</span
                >
            </div>
            <div class="text-right">
                <span
                    class="text-2xl font-bold tabular-nums {pctColor(
                        latest?.mem_percent ?? 0,
                    )}"
                >
                    {latest?.mem_percent ?? 0}<span
                        class="text-sm font-normal text-base-content/40">%</span
                    >
                </span>
                <div class="text-[10px] text-base-content/40">
                    {fmtMem(latest?.mem_used_mb)} / {fmtMem(
                        latest?.mem_total_mb,
                    )}
                </div>
            </div>
        </div>
        <div class="px-5 pb-4">
            <LineChart
                labels={timeLabels}
                datasets={memDatasets}
                yMax={100}
                yUnit="%"
                height="180px"
            />
        </div>
    </div>
</div>

<!-- Disk & Podman Storage -->
<div class="card bg-base-100 border border-base-300 rounded-2xl p-4 mb-6">
    <div class="flex items-center gap-4">
        <div class="bg-amber-500/10 text-amber-500 rounded-xl p-2">
            <span class="icon-[lucide--hard-drive] size-4"></span>
        </div>
        <div class="flex-1 min-w-0">
            <div class="flex items-center justify-between mb-1.5">
                <span class="text-sm font-medium text-base-content/60"
                    >Disk</span
                >
                <div class="flex items-baseline gap-2">
                    <span class="text-[11px] text-base-content/40">
                        {fmtDisk(latest?.disk_used_gb)} / {fmtDisk(
                            latest?.disk_total_gb,
                        )}
                    </span>
                    <span
                        class="text-lg font-bold tabular-nums {pctColor(
                            latest?.disk_percent ?? 0,
                        )}"
                    >
                        {latest?.disk_percent ?? 0}<span
                            class="text-xs font-normal text-base-content/40"
                            >%</span
                        >
                    </span>
                </div>
            </div>
            <div class="w-full bg-base-300 rounded-full h-2">
                <div
                    class="{barColor(
                        latest?.disk_percent ?? 0,
                        'bg-amber-500',
                    )} h-2 rounded-full transition-all duration-500"
                    style:width="{latest?.disk_percent ?? 0}%"
                ></div>
            </div>
        </div>
    </div>

    {#if podmanDisk}
        {@const byType = Object.fromEntries(podmanDisk.map((d) => [d.Type, d]))}
        {@const total =
            podmanDisk.reduce((s, d) => s + (d.RawSize || 0), 0) +
            buildkitBytes}
        <div class="border-t border-base-300 mt-4 pt-3">
            <div class="flex items-center justify-between mb-2">
                <span class="text-[11px] text-base-content/40"
                    >Podman Storage</span
                >
                <span class="text-xs tabular-nums">
                    <span class="font-bold">
                        {total >= 1e9
                            ? (total / 1e9).toFixed(1) + " GB"
                            : (total / 1e6).toFixed(0) + " MB"}
                    </span>
                    {#if latest?.disk_total_gb}
                        <span class="text-base-content/40 ml-1">
                            ({(
                                (total / (latest.disk_total_gb * 1073741824)) *
                                100
                            ).toFixed(1)}%)
                        </span>
                    {/if}
                </span>
            </div>
            <div class="grid grid-cols-4 gap-2 text-center">
                {#each [{ label: "Images", data: byType["Images"] }, { label: "Containers", data: byType["Containers"] }, { label: "Volumes", data: byType["Local Volumes"] }] as group}
                    <div class="bg-base-200 rounded-lg px-2 py-1.5">
                        <div class="text-[10px] text-base-content/40">
                            {group.label}
                        </div>
                        <div class="text-xs font-bold tabular-nums">
                            {group.data?.Size ?? "0B"}
                        </div>
                        <div class="text-[10px] text-base-content/40">
                            {group.data?.Total ?? 0}
                        </div>
                    </div>
                {/each}
                <div class="bg-base-200 rounded-lg px-2 py-1.5">
                    <div class="text-[10px] text-base-content/40">BuildKit</div>
                    <div class="text-xs font-bold tabular-nums">
                        {buildkitBytes >= 1e9
                            ? (buildkitBytes / 1e9).toFixed(1) + " GB"
                            : buildkitBytes >= 1e6
                              ? (buildkitBytes / 1e6).toFixed(0) + " MB"
                              : (buildkitBytes / 1e3).toFixed(0) + " KB"}
                    </div>
                    <div class="text-[10px] text-base-content/40">&nbsp;</div>
                </div>
            </div>
        </div>
    {:else}
        <div class="border-t border-base-300 mt-4 pt-3">
            <div class="flex items-center justify-between mb-2">
                <div class="skeleton h-3 w-24 rounded"></div>
                <div class="skeleton h-3 w-16 rounded"></div>
            </div>
            <div class="grid grid-cols-3 gap-2">
                {#each [0, 1, 2] as _}
                    <div
                        class="bg-base-200 rounded-lg px-2 py-1.5 flex flex-col gap-1 items-center"
                    >
                        <div class="skeleton h-2 w-10 rounded"></div>
                        <div class="skeleton h-3 w-8 rounded"></div>
                        <div class="skeleton h-2 w-4 rounded"></div>
                    </div>
                {/each}
            </div>
        </div>
    {/if}

    <!-- Cleanup section - always visible, no border separator -->
    {#if cleanupSettings}
        <div class="mt-4 pt-3">
            <div class="flex items-center gap-2 mb-3">
                <span
                    class="icon-[lucide--trash-2] size-3.5 text-base-content/40"
                ></span>
                <span class="text-xs font-medium text-base-content/60"
                    >Cleanup</span
                >
                {#if cleanupSettings.last_cleanup_at}
                    <span class="text-[10px] text-base-content/30">
                        last {fmtRelativeTime(
                            new Date(cleanupSettings.last_cleanup_at * 1000),
                        )}
                    </span>
                {/if}
            </div>

            <div class="flex flex-col gap-4">
                <!-- Manual cleanup buttons -->
                <div class="flex flex-wrap gap-2">
                    <button
                        class="btn btn-xs btn-error btn-outline"
                        disabled={!!cleaning}
                        onclick={() =>
                            runCleanup({
                                containers: true,
                                images: true,
                                volumes: true,
                                buildkit: true,
                            })}
                    >
                        {#if cleaning === "containers+images+volumes+buildkit"}
                            <span class="loading loading-spinner loading-xs"
                            ></span>
                        {:else}
                            <span class="icon-[lucide--trash-2] size-3"></span>
                        {/if}
                        All
                    </button>
                    {#each cleanupTargets as t}
                        <button
                            class="btn btn-xs btn-outline"
                            disabled={!!cleaning}
                            onclick={() => runCleanup({ [t.key]: true })}
                        >
                            {#if cleaning === t.key}
                                <span class="loading loading-spinner loading-xs"
                                ></span>
                            {:else}
                                <span class="{t.icon} size-3"></span>
                            {/if}
                            {t.label}
                        </button>
                    {/each}
                </div>

                <!-- Auto cleanup -->
                <div class="flex flex-col gap-3 p-3 bg-base-200 rounded-lg">
                    <label
                        class="flex items-center justify-between cursor-pointer"
                    >
                        <span class="text-xs font-medium">Auto Cleanup</span>
                        <input
                            type="checkbox"
                            class="toggle toggle-xs toggle-primary"
                            bind:checked={cleanupSettings.auto_cleanup_enabled}
                        />
                    </label>

                    {#if cleanupSettings.auto_cleanup_enabled}
                        <select
                            class="select select-xs select-bordered w-full"
                            bind:value={cleanupSettings.cleanup_interval_hours}
                        >
                            {#each intervalOpts as opt}
                                <option value={opt.value}>{opt.label}</option>
                            {/each}
                        </select>

                        <div class="flex flex-wrap gap-x-4 gap-y-1">
                            {#each cleanupTargets as t}
                                <label
                                    class="flex items-center gap-1.5 cursor-pointer"
                                >
                                    <input
                                        type="checkbox"
                                        class="checkbox checkbox-xs checkbox-primary"
                                        bind:checked={
                                            cleanupSettings[`clean_${t.key}`]
                                        }
                                    />
                                    <span class="text-xs">{t.label}</span>
                                </label>
                            {/each}
                        </div>
                    {/if}

                    {#if cleanupDirty}
                        <button
                            class="btn btn-xs btn-primary w-fit"
                            disabled={cleanupSaving}
                            onclick={saveCleanupSettings}
                        >
                            {#if cleanupSaving}
                                <span class="loading loading-spinner loading-xs"
                                ></span>
                            {/if}
                            Save
                        </button>
                    {/if}
                </div>
            </div>
        </div>
    {/if}
</div>

<!-- Containers table -->
<div class="card bg-base-100 border border-base-300 p-5 rounded-2xl">
    <div class="flex items-center gap-2 mb-4">
        <span class="icon-[lucide--container] size-4 text-primary"></span>
        <h2 class="text-sm font-semibold">Containers</h2>
        <span class="text-[10px] text-base-content/40"
            >({filtered().length})</span
        >
        <input
            type="text"
            placeholder="Search…"
            bind:value={search}
            class="input input-xs input-bordered w-40 ml-auto"
        />
    </div>

    {#if filtered().length}
        <div class="overflow-x-auto">
            <table class="table table-xs">
                <thead>
                    <tr class="text-[11px] text-base-content/40">
                        {#each [{ key: "name", label: "Name", align: "" }, { key: "cpu", label: "CPU", align: "text-right" }, { key: "mem", label: "Memory", align: "text-right" }] as col}
                            <th
                                class="cursor-pointer select-none hover:text-base-content/70 transition-colors {col.align}"
                                onclick={() => toggleSort(col.key)}
                            >
                                <span class="inline-flex items-center gap-0.5">
                                    {col.label}
                                    {#if sortCol === col.key}
                                        <span
                                            class="{sortAsc
                                                ? 'icon-[lucide--chevron-up]'
                                                : 'icon-[lucide--chevron-down]'} size-3"
                                        ></span>
                                    {/if}
                                </span>
                            </th>
                        {/each}
                    </tr>
                </thead>
                <tbody>
                    {#each paged as row}
                        <tr class="hover">
                            <td class="text-xs max-w-80 truncate">
                                {#if row.app_name}
                                    <a
                                        href="/projects/{row.project_id}"
                                        class="link link-primary"
                                        >{row.project_name}</a
                                    >
                                    <span class="text-base-content/30 mx-0.5"
                                        >›</span
                                    >
                                    <a
                                        href="/projects/{row.project_id}/apps/{row.app_id}/settings"
                                        class="link link-primary font-mono"
                                        >{row.app_name}</a
                                    >
                                {:else}
                                    <span class="font-mono">{row.name}</span>
                                {/if}
                            </td>
                            <td class="text-right font-mono text-xs">
                                <span
                                    class={row.cpu >= 50
                                        ? "text-error font-bold"
                                        : row.cpu >= 10
                                          ? "text-warning"
                                          : ""}
                                >
                                    {row.cpuDisplay}
                                </span>
                            </td>
                            <td class="text-right font-mono text-xs"
                                >{row.memDisplay}</td
                            >
                        </tr>
                    {/each}
                    {#if systemCpu > 0}
                        <tr class="text-base-content/40">
                            <td class="text-xs italic">System / Other</td>
                            <td class="text-right font-mono text-xs"
                                >{systemCpu}%</td
                            >
                            <td class="text-right font-mono text-xs">-</td>
                        </tr>
                    {/if}
                </tbody>
            </table>
        </div>

        {#if totalPages > 1 || perPage !== 30}
            <div
                class="flex items-center justify-between mt-3 pt-3 border-t border-base-300"
            >
                <div class="flex items-center gap-2">
                    <span class="text-[11px] text-base-content/40">
                        {page * perPage + 1}-{Math.min(
                            (page + 1) * perPage,
                            filtered().length,
                        )} of {filtered().length}
                    </span>
                    <select
                        class="select select-xs select-bordered w-auto text-[11px]"
                        value={perPage}
                        onchange={(e) => {
                            perPage = +e.currentTarget.value;
                            savePerPage(perPage);
                        }}
                    >
                        {#each perPageOpts as n}<option value={n}
                                >{n} / page</option
                            >{/each}
                    </select>
                </div>
                <div class="flex items-center gap-1">
                    <button
                        class="btn btn-xs btn-ghost btn-square"
                        aria-label="First page"
                        disabled={page === 0}
                        onclick={() => (page = 0)}
                    >
                        <span class="icon-[lucide--chevron-left] size-3"
                        ></span><span
                            class="icon-[lucide--chevron-left] size-3 -ml-2"
                        ></span>
                    </button>
                    <button
                        class="btn btn-xs btn-ghost btn-square"
                        aria-label="Previous page"
                        disabled={page === 0}
                        onclick={() => page--}
                    >
                        <span class="icon-[lucide--chevron-left] size-3"></span>
                    </button>
                    <span class="text-xs tabular-nums px-2"
                        >{page + 1} / {totalPages}</span
                    >
                    <button
                        class="btn btn-xs btn-ghost btn-square"
                        aria-label="Next page"
                        disabled={page >= totalPages - 1}
                        onclick={() => page++}
                    >
                        <span class="icon-[lucide--chevron-right] size-3"
                        ></span>
                    </button>
                    <button
                        class="btn btn-xs btn-ghost btn-square"
                        aria-label="Last page"
                        disabled={page >= totalPages - 1}
                        onclick={() => (page = totalPages - 1)}
                    >
                        <span class="icon-[lucide--chevron-right] size-3"
                        ></span><span
                            class="icon-[lucide--chevron-right] size-3 -ml-2"
                        ></span>
                    </button>
                </div>
            </div>
        {/if}
    {:else}
        <p class="text-xs text-base-content/40">No containers running</p>
    {/if}
</div>
