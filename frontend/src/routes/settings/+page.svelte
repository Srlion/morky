<script>
    import { invalidateAll } from "$app/navigation";
    import { api } from "$lib/api";
    import { toaster } from "$lib/components/Toasts.svelte";

    let { data } = $props();
    let panelDomain = $state("");
    let saving = $state(false);

    $effect(() => {
        panelDomain = data.settings?.panel_domain ?? "";
    });

    let original = $derived(data.settings?.panel_domain ?? "");
    let dirty = $derived(panelDomain.trim().toLowerCase() !== original);

    async function save() {
        saving = true;
        try {
            const pd = panelDomain.trim().toLowerCase() || null;
            await api.put("/settings", { panel_domain: pd });
            toaster.success({ title: "Settings saved" });
            await invalidateAll();
        } catch (e) {
            toaster.error({ title: e.message });
        } finally {
            saving = false;
        }
    }
</script>

<div class="mb-6">
    <h1 class="text-xl font-bold">Settings</h1>
    <p class="text-sm text-base-content/60 mt-0.5">
        Configure panel-wide options
    </p>
</div>

<div class="card bg-base-100 border border-base-300 p-5 rounded-2xl max-w-2xl">
    <h2 class="text-lg font-semibold mb-5">
        Panel
        {#if dirty}
            <span class="badge badge-warning badge-sm ml-2">Unsaved</span>
        {/if}
    </h2>

    <div class="flex flex-col gap-4">
        <label class="flex flex-col gap-1">
            <span class="label">Panel Domain</span>
            <input
                class="input input-bordered w-full text-sm font-mono"
                bind:value={panelDomain}
                placeholder="panel.example.com"
                autocomplete="off"
                spellcheck="false"
            />
            <span class="text- text-base-content/40">
                Must be a valid FQDN (e.g. panel.yourdomain.com). Leave empty to
                disable.
            </span>
        </label>
    </div>

    <div class="flex justify-end mt-5">
        <button
            class="btn btn-sm btn-primary"
            class:btn-warning={dirty}
            disabled={!dirty || saving}
            onclick={save}
        >
            {#if saving}
                <span class="loading loading-spinner loading-xs"></span> Saving...
            {:else}
                <span class="icon-[lucide--save] size-3.5"></span>
                {dirty ? "Save Changes" : "Saved"}
            {/if}
        </button>
    </div>
</div>
