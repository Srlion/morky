<script>
    import { goto, invalidateAll } from "$app/navigation";
    import { api } from "$lib/api";
    import { toaster } from "$lib/components/Toasts.svelte";
    import SearchSelect from "$lib/components/SearchSelect.svelte";
    import StatusBadge from "$lib/components/StatusBadge.svelte";
    import EnvVarsDialog, {
        envDialog,
    } from "$lib/components/EnvVarsDialog.svelte";
    import { globals } from "$lib/globals.svelte";

    const g = globals();

    let { data } = $props();
    let project = $derived(data.project);
    let apps = $derived(data.apps);

    let search = $state("");
    let showNew = $state(false);
    let appName = $state("");

    let sources = $state([]);
    let sourcesLoaded = $state(false);
    let sourcesLoading = $state(false);
    let selectedSource = $state(null);

    let repos = $state([]);
    let reposLoaded = $state(false);
    let reposLoading = $state(false);
    let selectedRepo = $state(null);

    let branches = $state([]);
    let branchesLoaded = $state(false);
    let branchesLoading = $state(false);
    let branch = $state("");

    let appNameError = $derived(
        appName && !validAppName(appName)
            ? "Only lowercase letters, digits, and hyphens. Must not start or end with a hyphen."
            : "",
    );

    let filtered = $derived(
        (search
            ? apps.filter((a) =>
                  a.name.toLowerCase().includes(search.toLowerCase()),
              )
            : apps
        ).toSorted((a, b) => a.id - b.id),
    );

    $effect(() => {
        selectedSource;
        repos = [];
        reposLoaded = false;
        selectedRepo = null;
        branches = [];
        branchesLoaded = false;
        branch = "";
    });

    $effect(() => {
        selectedRepo;
        branches = [];
        branchesLoaded = false;
        branch = "";
    });

    async function loadSources() {
        if (sourcesLoaded || sourcesLoading) return;
        sourcesLoading = true;
        try {
            sources = await api.get("/git-sources");
            sourcesLoaded = true;
        } catch {
            /* ignore */
        } finally {
            sourcesLoading = false;
        }
    }

    async function loadRepos() {
        if (reposLoaded || reposLoading || !selectedSource) return;
        reposLoading = true;
        try {
            repos = await api.get(`/git-sources/${selectedSource.id}/repos`);
            reposLoaded = true;
        } catch {
            /* ignore */
        } finally {
            reposLoading = false;
        }
    }

    async function loadBranches() {
        if (
            branchesLoaded ||
            branchesLoading ||
            !selectedSource ||
            !selectedRepo
        ) {
            return;
        }
        branchesLoading = true;
        try {
            const b = await api.get(
                `/git-sources/${selectedSource.id}/branches?repo=${encodeURIComponent(
                    selectedRepo.full_name,
                )}`,
            );
            branches = b;
            branchesLoaded = true;
            if (!branch) {
                const def = selectedRepo.default_branch || "main";
                branch = b.includes(def) ? def : b[0] || "";
            }
        } catch {
            /* ignore */
        } finally {
            branchesLoading = false;
        }
    }

    function openNew() {
        appName = "";
        selectedSource = null;
        sources = [];
        sourcesLoaded = false;
        repos = [];
        reposLoaded = false;
        selectedRepo = null;
        branches = [];
        branchesLoaded = false;
        branch = "";
        showNew = true;
    }

    function closeNew() {
        showNew = false;
    }

    async function createApp() {
        try {
            await api.post("/apps", {
                project_id: project.id,
                name: appName,
                git_source_id: selectedSource.id,
                repo: selectedRepo?.full_name,
                branch,
            });
            toaster.success({ title: "App created" });
            showNew = false;
            await invalidateAll();
        } catch (e) {
            toaster.error({ title: e.message });
        }
    }

    async function deleteApp(appId) {
        if (!confirm("Delete this app?")) return;
        try {
            await api.del(`/apps/${appId}`);
            toaster.success({ title: "App deleted" });
            await invalidateAll();
        } catch (e) {
            toaster.error({ title: e.message });
        }
    }

    async function deleteProject() {
        if (!confirm("Delete this project and all its apps?")) return;
        try {
            await api.del(`/projects/${project.id}`);
            toaster.success({ title: "Project deleted" });
            goto("/projects");
        } catch (e) {
            toaster.error({ title: e.message });
        }
    }

    function validAppName(name) {
        if (!name || name.length > 63) return false;
        if (
            !/^[a-z0-9][a-z0-9-]*[a-z0-9]$/.test(name) &&
            !/^[a-z0-9]$/.test(name)
        )
            return false;
        return true;
    }
</script>

<nav class="text-xs breadcrumbs py-0 mb-4">
    <ul>
        <li><a href="/projects" class="link link-primary">Projects</a></li>
        <li>{project.name}</li>
    </ul>
</nav>

<div class="flex items-center justify-between mb-6">
    <div>
        <h1 class="text-xl font-bold">{project.name}</h1>
        {#if project.description}<p class="text-sm text-base-content/60 mt-0.5">
                {project.description}
            </p>{/if}
    </div>
    <div class="flex gap-2">
        {#if !showNew}
            <button class="btn btn-sm btn-primary" onclick={openNew}>
                <span class="icon-[lucide--plus] size-4"></span> New App
            </button>
        {/if}
        <button
            class="btn btn-sm btn-ghost"
            onclick={() =>
                envDialog.show(`/projects/${project.id}/env`, project.env_vars)}
        >
            <span class="icon-[lucide--variable] size-3.5"></span> Env
        </button>
        <button
            class="btn btn-sm btn-error btn-outline"
            aria-label="Delete project"
            onclick={deleteProject}
        >
            <span class="icon-[lucide--trash-2] size-3.5"></span>
        </button>
    </div>
</div>

{#if showNew}
    <div class="card bg-base-100 border border-base-300 p-5 rounded-2xl mb-6">
        <div class="flex items-center justify-between mb-4">
            <h3 class="text-lg font-semibold">New App</h3>
            <button
                class="btn btn-sm btn-ghost btn-square"
                aria-label="Close"
                onclick={closeNew}
            >
                <span class="icon-[lucide--x] size-4"></span>
            </button>
        </div>
        <div
            class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4 items-start"
        >
            <label class="flex flex-col gap-1">
                <span class="label">Name</span>
                <input
                    class="input input-bordered w-full {appNameError
                        ? 'input-error'
                        : ''}"
                    bind:value={appName}
                    placeholder="my-app"
                />
                {#if appNameError}
                    <span class="text-error text-xs">{appNameError}</span>
                {/if}
            </label>

            <div class="flex flex-col gap-1">
                <span class="label">Git Source</span>
                <SearchSelect
                    bind:value={selectedSource}
                    items={sources}
                    loading={sourcesLoading}
                    labelKey="name"
                    placeholder="Select source..."
                    searchPlaceholder="Search sources..."
                    onopen={loadSources}
                />
            </div>

            {#if selectedSource}
                <div class="flex flex-col gap-1">
                    <span class="label">Repository</span>
                    <SearchSelect
                        bind:value={selectedRepo}
                        items={repos}
                        loading={reposLoading}
                        labelKey="full_name"
                        placeholder="Select repository..."
                        searchPlaceholder="Search repos..."
                        onopen={loadRepos}
                    />
                </div>
            {/if}

            {#if selectedRepo}
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
            {/if}
        </div>

        <div class="flex justify-end gap-2 mt-5">
            <button class="btn btn-sm" onclick={closeNew}>Cancel</button>
            <button
                class="btn btn-sm btn-primary"
                disabled={!branch || !validAppName(appName)}
                onclick={createApp}
            >
                Create
            </button>
        </div>
    </div>
{/if}

{#if apps.length}
    <div class="relative max-w-xs mb-5">
        <span
            class="icon-[lucide--search] size-4 absolute left-3 top-1/2 -translate-y-1/2 text-base-content/40 pointer-events-none z-10"
        ></span>
        <input
            class="input input-bordered text-sm pl-9 w-full"
            placeholder="Search apps..."
            bind:value={search}
        />
    </div>
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
        {#each filtered as app (app.id)}
            {@const status = g[`app_status_${app.id}`] ?? app.status}
            {@const isBuilding =
                g[`app_deploy_status_${app.id}`] === "building"}

            <div
                class="group card bg-base-100 border border-base-300 hover:shadow-md p-4 transition-all"
            >
                <a
                    href="/projects/{project.id}/apps/{app.id}/settings"
                    class="block no-underline"
                >
                    <div class="flex items-center justify-between mb-2">
                        <span class="font-semibold text-sm truncate"
                            >{app.name}</span
                        >

                        <div class="flex items-center gap-2">
                            {#if isBuilding}
                                <div
                                    class="badge badge-primary badge-outline gap-1 animate-pulse text-[10px] h-5 font-bold"
                                >
                                    <span
                                        class="loading loading-spinner size-2.5"
                                    ></span>
                                    BUILDING
                                </div>
                            {/if}
                            <StatusBadge {status} />
                        </div>
                    </div>
                </a>
                <div
                    class="mt-3 pt-2 border-t border-base-300 flex justify-end"
                >
                    <button
                        class="btn btn-xs btn-error btn-outline opacity-0 group-hover:opacity-100 transition-opacity"
                        onclick={() => deleteApp(app.id)}
                    >
                        <span class="icon-[lucide--trash-2] size-3"></span> Delete
                    </button>
                </div>
            </div>
        {/each}
    </div>
{:else}
    <div class="flex flex-col items-center justify-center py-24 text-center">
        <div class="bg-base-200 rounded-full p-4 mb-4">
            <span class="icon-[lucide--box] size-8 text-base-content/40"></span>
        </div>
        <p class="text-sm text-base-content/50 mb-1">No apps yet</p>
        <p class="text-xs text-base-content/40">
            Create an app to start deploying.
        </p>
    </div>
{/if}

<EnvVarsDialog />
