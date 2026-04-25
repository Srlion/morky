<script>
    import { page } from "$app/state";
    import { api } from "$lib/api";
    import { toaster } from "$lib/components/Toasts.svelte";
    import { AppStatus } from "$lib/status";
    import { globals } from "$lib/globals.svelte";

    const g = globals();
    let { data } = $props();
    let app = $derived(data.app);
    let status = $derived(g[`app_status_${app?.id}`] ?? app?.status);
    const appId = $derived(page.params.appId);

    let path = $state("/");
    let entries = $state([]);
    let loading = $state(false);
    let uploading = $state(false);
    let fileInput = $state();

    const stopped = $derived(
        status === AppStatus.IDLE || status === AppStatus.FAILED,
    );

    const hasVolume = $derived(app?.volume_path && app.volume_path !== "");

    const volumeLabel = $derived(app?.volume_path || "/data");

    const breadcrumbs = $derived(() => {
        const parts = path.split("/").filter(Boolean);
        const crumbs = [{ label: volumeLabel, path: "/" }];
        let built = "";
        for (const p of parts) {
            built += "/" + p;
            crumbs.push({ label: p, path: built });
        }
        return crumbs;
    });

    $effect(() => {
        if (hasVolume) load();
    });

    async function load() {
        loading = true;
        try {
            const res = await api.get(
                `/apps/${appId}/volume/files?path=${encodeURIComponent(path)}`,
            );
            entries = res.entries;
        } catch (e) {
            // 403 just means app is running — the UI already shows the message
            if (!e.message?.includes("403") && !e.status === 403) {
                toaster.error({ title: e.message });
            }
            entries = [];
        } finally {
            loading = false;
        }
    }

    function navigate(p) {
        path = p;
        load();
    }

    function enter(entry) {
        const newPath = path.endsWith("/")
            ? path + entry.name
            : path + "/" + entry.name;
        navigate(newPath);
    }

    function goUp() {
        const parts = path.split("/").filter(Boolean);
        parts.pop();
        navigate(parts.length ? "/" + parts.join("/") : "/");
    }

    function fmtSize(bytes) {
        if (bytes < 1024) return `${bytes} B`;
        if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
        return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    }

    async function download(entry) {
        const filePath = path.endsWith("/")
            ? path + entry.name
            : path + "/" + entry.name;
        const url = `/api/apps/${appId}/volume/file?path=${encodeURIComponent(filePath)}`;
        const a = document.createElement("a");
        a.href = url;
        a.download = entry.name;
        a.click();
    }

    async function deleteEntry(entry) {
        const filePath = path.endsWith("/")
            ? path + entry.name
            : path + "/" + entry.name;
        if (!confirm(`Delete ${entry.name}?`)) return;
        try {
            await api.delete(
                `/apps/${appId}/volume/file?path=${encodeURIComponent(filePath)}`,
            );
            await load();
        } catch (e) {
            toaster.error({ title: e.message });
        }
    }

    async function upload(files) {
        if (!files?.length) return;
        uploading = true;
        try {
            const fd = new FormData();
            for (const f of files) fd.append("file", f, f.name);
            const res = await fetch(
                `/api/apps/${appId}/volume/file?path=${encodeURIComponent(path)}`,
                { method: "POST", body: fd },
            );
            if (!res.ok) {
                const err = await res.json().catch(() => ({}));
                throw new Error(err.error || "Upload failed");
            }
            toaster.success({ title: `Uploaded ${files.length} file(s)` });
            if (fileInput) fileInput.value = "";
            await load();
        } catch (e) {
            toaster.error({ title: e.message });
        } finally {
            uploading = false;
        }
    }
</script>

{#if !hasVolume}
    <div class="text-sm text-base-content/50 py-10 text-center">
        This app has no volume configured. Enable it in Build Settings.
    </div>
{:else if !stopped}
    <div class="text-sm text-base-content/50 py-10 text-center">
        <p class="font-medium mb-1">
            App must be stopped to access the volume.
        </p>
        <p class="text-xs">Stop the app first, then come back here.</p>
    </div>
{:else}
    <!-- Toolbar -->
    <div class="flex items-center gap-2 mb-3 flex-wrap">
        <!-- Breadcrumb -->
        <nav class="text-xs breadcrumbs py-0 flex-1 min-w-0">
            <ul>
                {#each breadcrumbs() as crumb}
                    <li>
                        <button
                            class="link link-primary"
                            onclick={() => navigate(crumb.path)}
                            >{crumb.label}</button
                        >
                    </li>
                {/each}
            </ul>
        </nav>

        <input
            type="file"
            multiple
            class="hidden"
            bind:this={fileInput}
            onchange={(e) => upload(e.target.files)}
        />
        <button
            class="btn btn-xs btn-outline"
            onclick={() => fileInput.click()}
            disabled={uploading}
        >
            {#if uploading}
                <span class="loading loading-spinner loading-xs"></span>
            {:else}
                <span class="icon-[lucide--upload] size-3"></span>
            {/if}
            Upload
        </button>
        <button
            class="btn btn-xs btn-ghost"
            onclick={load}
            disabled={loading}
            aria-label="Refresh"
        >
            <span class="icon-[lucide--refresh-cw] size-3"></span>
        </button>
    </div>

    <!-- File table -->
    <div class="border border-base-300 rounded-2xl overflow-hidden">
        {#if loading}
            <div class="p-8 text-center text-sm text-base-content/40">
                <span class="loading loading-spinner loading-sm"></span>
            </div>
        {:else if entries.length === 0 && path === "/"}
            <div class="p-8 text-center text-sm text-base-content/40">
                Volume is empty.
            </div>
        {:else}
            <table class="table table-sm w-full">
                <thead>
                    <tr class="text-xs text-base-content/40">
                        <th>Name</th>
                        <th class="text-right">Size</th>
                        <th></th>
                    </tr>
                </thead>
                <tbody>
                    {#if path !== "/"}
                        <tr class="hover cursor-pointer" onclick={goUp}>
                            <td
                                colspan="3"
                                class="font-mono text-xs text-base-content/40"
                            >
                                <span class="icon-[lucide--folder] size-3 mr-1"
                                ></span>..
                            </td>
                        </tr>
                    {/if}
                    {#each entries as entry (entry.name)}
                        <tr class="hover">
                            <td class="font-mono text-xs">
                                {#if entry.is_dir}
                                    <button
                                        class="flex items-center gap-1.5 text-left"
                                        onclick={() => enter(entry)}
                                    >
                                        <span
                                            class="icon-[lucide--folder] size-3 text-warning"
                                        ></span>
                                        {entry.name}/
                                    </button>
                                {:else}
                                    <span class="flex items-center gap-1.5">
                                        <span
                                            class="icon-[lucide--file] size-3 text-base-content/30"
                                        ></span>
                                        {entry.name}
                                    </span>
                                {/if}
                            </td>
                            <td class="text-right text-xs text-base-content/40">
                                {entry.is_dir ? "" : fmtSize(entry.size)}
                            </td>
                            <td class="text-right">
                                <div
                                    class="flex items-center justify-end gap-1"
                                >
                                    {#if !entry.is_dir}
                                        <button
                                            class="btn btn-xs btn-ghost"
                                            onclick={() => download(entry)}
                                            title="Download"
                                        >
                                            <span
                                                class="icon-[lucide--download] size-3"
                                            ></span>
                                        </button>
                                    {/if}
                                    <button
                                        class="btn btn-xs btn-ghost text-error"
                                        onclick={() => deleteEntry(entry)}
                                        title="Delete"
                                    >
                                        <span
                                            class="icon-[lucide--trash-2] size-3"
                                        ></span>
                                    </button>
                                </div>
                            </td>
                        </tr>
                    {/each}
                </tbody>
            </table>
        {/if}
    </div>
{/if}
