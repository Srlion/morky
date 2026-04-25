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

    let dragging = $state(false);

    let showMkdir = $state(false);

    let newFolderName = $state("");

    let mkdirInput = $state();

    // Internal DnD state

    let dragEntry = $state(null);

    let dropTarget = $state(null);

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

    function joinPath(base, name) {
        return base.endsWith("/") ? base + name : base + "/" + name;
    }

    async function load() {
        loading = true;

        try {
            const res = await api.get(
                `/apps/${appId}/volume/files?path=${encodeURIComponent(path)}`,
            );

            entries = res.entries;
        } catch (e) {
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
        navigate(joinPath(path, entry.name));
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
        const filePath = joinPath(path, entry.name);

        const url = `/api/apps/${appId}/volume/file?path=${encodeURIComponent(filePath)}`;

        const a = document.createElement("a");

        a.href = url;

        a.download = entry.name;

        a.click();
    }

    async function deleteEntry(entry) {
        const filePath = joinPath(path, entry.name);

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

    // External drop (files from desktop)

    function isExternalDrag(e) {
        return e.dataTransfer?.types?.includes("Files");
    }

    function handleDragOver(e) {
        e.preventDefault();

        if (isExternalDrag(e)) dragging = true;
    }

    function handleDragLeave(e) {
        if (e.currentTarget.contains(e.relatedTarget)) return;

        dragging = false;
    }

    function handleDrop(e) {
        e.preventDefault();

        dragging = false;

        // External file drop

        if (isExternalDrag(e) && e.dataTransfer?.files?.length) {
            upload(e.dataTransfer.files);

            return;
        }

        // Internal drop on the table background (not on a folder row) — ignore

        dragEntry = null;

        dropTarget = null;
    }

    // Internal DnD (move entries between folders)

    function handleRowDragStart(e, entry) {
        dragEntry = entry;

        e.dataTransfer.effectAllowed = "move";

        e.dataTransfer.setData("text/plain", entry.name);
    }

    function handleRowDragEnd() {
        dragEntry = null;

        dropTarget = null;
    }

    function handleRowDragOver(e, entry) {
        if (!dragEntry || !entry.is_dir || entry.name === dragEntry.name)
            return;

        e.preventDefault();

        e.dataTransfer.dropEffect = "move";

        dropTarget = entry.name;
    }

    function handleRowDragLeave(e, entry) {
        if (dropTarget === entry.name) dropTarget = null;
    }

    async function handleRowDrop(e, entry) {
        e.preventDefault();

        e.stopPropagation();

        if (!dragEntry || !entry.is_dir || entry.name === dragEntry.name)
            return;

        const from = joinPath(path, dragEntry.name);

        const to = joinPath(joinPath(path, entry.name), dragEntry.name);

        dragEntry = null;

        dropTarget = null;

        try {
            await api.post(
                `/apps/${appId}/volume/move?from=${encodeURIComponent(from)}&to=${encodeURIComponent(to)}`,
            );

            await load();
        } catch (e) {
            toaster.error({ title: e.message });
        }
    }

    // Drop on ".." row → move to parent

    async function handleParentDrop(e) {
        e.preventDefault();

        e.stopPropagation();

        if (!dragEntry) return;

        const from = joinPath(path, dragEntry.name);

        const parts = path.split("/").filter(Boolean);

        parts.pop();

        const parentPath = parts.length ? "/" + parts.join("/") : "/";

        const to = joinPath(parentPath, dragEntry.name);

        dragEntry = null;

        dropTarget = null;

        try {
            await api.post(
                `/apps/${appId}/volume/move?from=${encodeURIComponent(from)}&to=${encodeURIComponent(to)}`,
            );

            await load();
        } catch (e) {
            toaster.error({ title: e.message });
        }
    }

    function startMkdir() {
        showMkdir = true;

        newFolderName = "";

        setTimeout(() => mkdirInput?.focus(), 0);
    }

    async function createFolder() {
        const name = newFolderName.trim();

        if (!name) return;

        const folderPath = joinPath(path, name);

        try {
            await api.post(
                `/apps/${appId}/volume/mkdir?path=${encodeURIComponent(folderPath)}`,
            );

            showMkdir = false;

            newFolderName = "";

            await load();
        } catch (e) {
            toaster.error({ title: e.message });
        }
    }

    function handleMkdirKey(e) {
        if (e.key === "Enter") createFolder();

        if (e.key === "Escape") {
            showMkdir = false;

            newFolderName = "";
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

        <button class="btn btn-xs btn-outline" onclick={startMkdir}>
            <span class="icon-[lucide--folder-plus] size-3"></span>

            New Folder
        </button>

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

    <!-- New folder input -->

    {#if showMkdir}
        <div class="flex items-center gap-2 mb-3">
            <span class="icon-[lucide--folder-plus] size-4 text-warning"></span>

            <input
                bind:this={mkdirInput}
                bind:value={newFolderName}
                onkeydown={handleMkdirKey}
                class="input input-bordered input-sm flex-1 font-mono"
                placeholder="folder name"
            />

            <button class="btn btn-xs btn-primary" onclick={createFolder}>
                Create
            </button>

            <button
                class="btn btn-xs btn-ghost"
                onclick={() => {
                    showMkdir = false;

                    newFolderName = "";
                }}
            >
                Cancel
            </button>
        </div>
    {/if}

    <!-- File table with drop zone -->

    <div
        role="region"
        aria-label="File drop zone"
        class="border rounded-2xl overflow-hidden transition-colors {dragging
            ? 'border-primary bg-primary/5 border-dashed border-2'
            : 'border-base-300'}"
        ondragover={handleDragOver}
        ondragleave={handleDragLeave}
        ondrop={handleDrop}
    >
        {#if dragging}
            <div class="p-12 text-center">
                <span class="icon-[lucide--upload] size-8 text-primary mb-2"
                ></span>

                <p class="text-sm text-primary font-medium">
                    Drop files here to upload
                </p>
            </div>
        {:else if loading}
            <div class="p-8 text-center text-sm text-base-content/40">
                <span class="loading loading-spinner loading-sm"></span>
            </div>
        {:else if entries.length === 0 && path === "/"}
            <div class="p-8 text-center text-sm text-base-content/40">
                Volume is empty. Drag files here or click Upload.
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
                        <tr
                            class="hover cursor-pointer {dragEntry &&
                            dropTarget === '..'
                                ? 'bg-primary/10'
                                : ''}"
                            onclick={goUp}
                            ondragover={(e) => {
                                if (dragEntry) {
                                    e.preventDefault();

                                    dropTarget = "..";
                                }
                            }}
                            ondragleave={() => {
                                if (dropTarget === "..") dropTarget = null;
                            }}
                            ondrop={handleParentDrop}
                        >
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
                        <tr
                            class="hover {entry.is_dir
                                ? 'cursor-pointer'
                                : ''} {dragEntry?.name === entry.name
                                ? 'opacity-40'
                                : ''} {dropTarget === entry.name
                                ? 'bg-primary/10'
                                : ''}"
                            onclick={() => entry.is_dir && enter(entry)}
                            draggable="true"
                            ondragstart={(e) => handleRowDragStart(e, entry)}
                            ondragend={handleRowDragEnd}
                            ondragover={(e) => handleRowDragOver(e, entry)}
                            ondragleave={(e) => handleRowDragLeave(e, entry)}
                            ondrop={(e) => handleRowDrop(e, entry)}
                        >
                            <td class="font-mono text-xs">
                                {#if entry.is_dir}
                                    <span
                                        class="flex items-center gap-1.5 text-left"
                                    >
                                        <span
                                            class="icon-[lucide--folder] size-3 text-warning"
                                        ></span>
                                        {entry.name}/
                                    </span>
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
