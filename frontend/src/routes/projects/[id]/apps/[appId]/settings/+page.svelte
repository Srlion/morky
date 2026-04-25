<script>
    import { page } from "$app/state";
    import { beforeNavigate, invalidateAll } from "$app/navigation";
    import { api } from "$lib/api";
    import { toaster } from "$lib/components/Toasts.svelte";
    import SearchSelect from "$lib/components/SearchSelect.svelte";

    let { data } = $props();
    let app = $derived(data.app);

    let branch = $state("");
    let buildMethod = $state("");
    let dockerfilePath = $state("");
    let port = $state(3000);
    let healthCheckPath = $state("");
    let volumePath = $state("");
    let containerLogsEnabled = $state(false);
    let domain = $state("");

    let branches = $state([]);
    let branchesLoaded = $state(false);
    let branchesLoading = $state(false);

    const appId = $derived(page.params.appId);

    let buildDirty = $derived(
        app &&
            (branch !== app.branch ||
                buildMethod !== app.build_method ||
                dockerfilePath !== app.dockerfile_path ||
                port !== app.port ||
                healthCheckPath !== app.health_check_path ||
                volumePath !== app.volume_path),
    );

    let generalDirty = $derived(
        app &&
            (containerLogsEnabled !== app.container_logs_enabled ||
                (domain || "") !== (app.domain || "")),
    );

    let dirty = $derived(buildDirty || generalDirty);

    // Browser navigation (refresh, close tab, external link)
    $effect(() => {
        function handleBeforeUnload(e) {
            if (dirty) {
                e.preventDefault();
            }
        }
        window.addEventListener("beforeunload", handleBeforeUnload);
        return () =>
            window.removeEventListener("beforeunload", handleBeforeUnload);
    });

    // SvelteKit client-side navigation
    beforeNavigate(({ cancel }) => {
        if (dirty && !confirm("You have unsaved changes. Leave anyway?")) {
            cancel();
        }
    });

    $effect(() => {
        if (app) {
            branch = app.branch;
            buildMethod = app.build_method;
            dockerfilePath = app.dockerfile_path;
            port = app.port;
            healthCheckPath = app.health_check_path;
            volumePath = app.volume_path ?? "";
            containerLogsEnabled = app.container_logs_enabled;
            domain = app.domain ?? "";
        }
    });

    async function loadBranches() {
        if (
            branchesLoaded ||
            branchesLoading ||
            !app?.git_source_id ||
            !app?.repo
        ) {
            return;
        }
        branchesLoading = true;
        try {
            branches = await api.get(
                `/git-sources/${app.git_source_id}/branches?repo=${encodeURIComponent(
                    app.repo,
                )}`,
            );
            branchesLoaded = true;
        } catch {
            /* ignore */
        } finally {
            branchesLoading = false;
        }
    }

    async function save() {
        try {
            await api.put(`/apps/${appId}/settings`, {
                branch,
                build_method: buildMethod,
                dockerfile_path: dockerfilePath,
                port,
                health_check_path: healthCheckPath,
                volume_path: volumePath,
            });
            toaster.success({ title: "Settings saved" });
            await invalidateAll();
        } catch (e) {
            toaster.error({ title: e.message });
        }
    }

    async function saveGeneral() {
        // <-- renamed from saveLogging
        try {
            await api.put(`/apps/${appId}/general-settings`, {
                container_logs_enabled: containerLogsEnabled,
                domain: domain.trim() || null,
            });
            toaster.success({ title: "General settings saved" });
            await invalidateAll();
        } catch (e) {
            toaster.error({ title: e.message });
        }
    }
</script>

<div class="flex flex-col gap-5">
    <div class="card bg-base-100 border border-base-300 p-5 rounded-2xl">
        <h2 class="text-lg font-semibold mb-5">
            Build Settings
            {#if buildDirty}
                <span class="badge badge-warning badge-sm ml-2">Unsaved</span>
            {/if}
        </h2>

        <div
            class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4 items-start"
        >
            <div class="flex flex-col gap-1">
                <span class="label">Branch</span>
                <SearchSelect
                    bind:value={branch}
                    items={branches}
                    loading={branchesLoading}
                    placeholder="Select branch..."
                    searchPlaceholder="Search branches..."
                    onopen={loadBranches}
                />
            </div>

            <label class="flex flex-col gap-1">
                <span class="label">Build Method</span>
                <select
                    class="select select-bordered w-full"
                    bind:value={buildMethod}
                >
                    <option value="railpack">Railpack</option>
                    <option value="dockerfile">Dockerfile</option>
                </select>
            </label>

            {#if buildMethod === "dockerfile"}
                <label class="flex flex-col gap-1">
                    <span class="label">Dockerfile Path</span>
                    <input
                        class="input input-bordered w-full"
                        bind:value={dockerfilePath}
                    />
                </label>
            {/if}

            <label class="flex flex-col gap-1">
                <span class="label">Port</span>
                <input
                    class="input input-bordered w-full"
                    type="number"
                    min="1"
                    max="65535"
                    bind:value={port}
                />
            </label>

            <label class="flex flex-col gap-1">
                <span class="label">Health Check Path</span>
                <input
                    class="input input-bordered w-full"
                    bind:value={healthCheckPath}
                    placeholder="/healthz"
                />
                <span class="text-[11px] text-base-content/40">
                    Leave empty to skip. Must return 200 or 201.
                </span>
            </label>

            <label class="flex flex-col gap-1">
                <span class="label">Volume Path</span>
                <input
                    class="input input-bordered w-full"
                    bind:value={volumePath}
                    placeholder="/data"
                />
                <span class="text-[11px] text-base-content/40">
                    Leave empty to disable persistent storage.
                </span>
            </label>
        </div>

        <div class="flex justify-end mt-5">
            <button
                class="btn btn-sm btn-primary"
                class:btn-warning={buildDirty}
                disabled={!buildDirty}
                onclick={save}
            >
                <span class="icon-[lucide--save] size-3.5"></span>
                {buildDirty ? "Save Changes" : "Save"}
            </button>
        </div>
    </div>

    <div class="card bg-base-100 border border-base-300 p-5 rounded-2xl">
        <h2 class="text-lg font-semibold mb-5">
            General
            {#if generalDirty}
                <span class="badge badge-warning badge-sm ml-2">Unsaved</span>
            {/if}
        </h2>

        <div class="grid grid-cols-1 md:grid-cols-2 gap-5">
            <label class="flex flex-col gap-1">
                <span class="label">Custom Domain</span>
                <input
                    class="input input-bordered w-full font-mono"
                    bind:value={domain}
                    placeholder="app.example.com"
                    autocapitalize="off"
                    spellcheck="false"
                />
                <span class="text- text-base-content/40">
                    Point DNS to this server first. Leave empty to remove.
                </span>
            </label>

            <label
                class="flex items-center justify-between cursor-pointer p-3 rounded-lg border border-base-300 md:mt-6"
            >
                <div>
                    <span class="font-medium">Container Logs</span>
                    <p class="text-[11px] text-base-content/40">
                        Store container stdout/stderr logs.
                        <br />
                        Changes take effect after restarting/deploying the app.
                    </p>
                </div>
                <input
                    type="checkbox"
                    class="toggle toggle-primary"
                    bind:checked={containerLogsEnabled}
                />
            </label>
        </div>

        <div class="flex justify-end mt-5">
            <button
                class="btn btn-sm btn-primary"
                class:btn-warning={generalDirty}
                disabled={!generalDirty}
                onclick={saveGeneral}
            >
                <span class="icon-[lucide--save] size-3.5"></span>
                {generalDirty ? "Save Changes" : "Save"}
            </button>
        </div>
    </div>
</div>
