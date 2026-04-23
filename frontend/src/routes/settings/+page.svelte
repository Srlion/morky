<script>
    import { invalidateAll } from "$app/navigation";
    import { api } from "$lib/api";
    import { toaster } from "$lib/components/Toasts.svelte";

    let { data } = $props();
    let tab = $state("panel");

    // Panel
    let panelDomain = $state("");
    let saving = $state(false);

    $effect(() => {
        panelDomain = data.settings?.panel_domain ?? "";
    });

    let original = $derived(data.settings?.panel_domain ?? "");
    let dirty = $derived(panelDomain.trim().toLowerCase() !== original);

    async function saveDomain() {
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

    // Security
    let currentPassword = $state("");
    let newPassword = $state("");
    let confirmPassword = $state("");
    let changingPassword = $state(false);

    async function changePassword() {
        if (newPassword !== confirmPassword) {
            toaster.error({ title: "Passwords do not match" });
            return;
        }
        if (newPassword.length < 8) {
            toaster.error({ title: "Password must be at least 8 characters" });
            return;
        }
        changingPassword = true;
        try {
            await api.post("/settings/change-password", {
                current_password: currentPassword,
                new_password: newPassword,
            });
            toaster.success({ title: "Password changed" });
            currentPassword = "";
            newPassword = "";
            confirmPassword = "";
        } catch (e) {
            toaster.error({ title: e.message });
        } finally {
            changingPassword = false;
        }
    }
</script>

<div class="mb-6">
    <h1 class="text-xl font-bold">Settings</h1>
    <p class="text-sm text-base-content/60 mt-0.5">
        Configure panel-wide options
    </p>
</div>

<div role="tablist" class="tabs tabs-bordered mb-6">
    <button
        role="tab"
        class="tab {tab === 'panel' ? 'tab-active' : ''}"
        onclick={() => (tab = "panel")}>Panel</button
    >
    <button
        role="tab"
        class="tab {tab === 'security' ? 'tab-active' : ''}"
        onclick={() => (tab = "security")}>Security</button
    >
</div>

{#if tab === "panel"}
    <div
        class="card bg-base-100 border border-base-300 p-5 rounded-2xl max-w-2xl"
    >
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
                <span class="text-xs text-base-content/40">
                    Must be a valid FQDN (e.g. panel.yourdomain.com). Leave
                    empty to disable.
                </span>
            </label>
        </div>
        <div class="flex justify-end mt-5">
            <button
                class="btn btn-sm btn-primary"
                class:btn-warning={dirty}
                disabled={!dirty || saving}
                onclick={saveDomain}
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
{:else if tab === "security"}
    <div
        class="card bg-base-100 border border-base-300 p-5 rounded-2xl max-w-2xl"
    >
        <h2 class="text-lg font-semibold mb-5">Change Password</h2>
        <div class="flex flex-col gap-4">
            <label class="flex flex-col gap-1">
                <span class="label">Current Password</span>
                <input
                    type="password"
                    class="input input-bordered w-full text-sm"
                    bind:value={currentPassword}
                    autocomplete="current-password"
                />
            </label>
            <label class="flex flex-col gap-1">
                <span class="label">New Password</span>
                <input
                    type="password"
                    class="input input-bordered w-full text-sm"
                    bind:value={newPassword}
                    autocomplete="new-password"
                />
            </label>
            <label class="flex flex-col gap-1">
                <span class="label">Confirm New Password</span>
                <input
                    type="password"
                    class="input input-bordered w-full text-sm"
                    bind:value={confirmPassword}
                    autocomplete="new-password"
                />
            </label>
        </div>
        <div class="flex justify-end mt-5">
            <button
                class="btn btn-sm btn-primary"
                disabled={!currentPassword ||
                    !newPassword ||
                    !confirmPassword ||
                    changingPassword}
                onclick={changePassword}
            >
                {#if changingPassword}
                    <span class="loading loading-spinner loading-xs"></span> Saving...
                {:else}
                    <span class="icon-[lucide--lock] size-3.5"></span> Change Password
                {/if}
            </button>
        </div>
    </div>
{/if}
