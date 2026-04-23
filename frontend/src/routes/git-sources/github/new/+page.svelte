<script>
    import { page } from "$app/state";
    import { onMount } from "svelte";
    import { api } from "$lib/api";

    let org = $derived(new URL(page.url).searchParams.get("org") || "");

    let githubUrl = $state("");
    let manifest = $state("");
    let loading = $state(true);
    let error = $state("");
    let formEl = $state();

    onMount(async () => {
        try {
            const res = await api.post("/git-sources/github/create-manifest", {
                org: org || undefined,
            });
            githubUrl = res.github_url;
            manifest = res.manifest;
        } catch (e) {
            error = e.message;
        } finally {
            loading = false;
        }
    });
</script>

<nav class="text-xs breadcrumbs py-0 mb-4">
    <ul>
        <li>
            <a href="/git-sources" class="link link-primary">Git Sources</a>
        </li>
        <li>New GitHub App</li>
    </ul>
</nav>

<div class="max-w-md">
    <h1 class="text-xl font-bold mb-1">Create GitHub App</h1>
    {#if org}
        <p class="text-sm text-base-content/60 mb-6">
            Organization: <strong class="text-base-content/80">{org}</strong>
        </p>
    {:else}
        <p class="text-sm text-base-content/60 mb-6">
            This will create a GitHub App on your personal account.
        </p>
    {/if}

    <div
        class="bg-base-200 border border-base-300 rounded-2xl p-4 mb-6 text-sm text-base-content/60 leading-relaxed"
    >
        You'll be redirected to GitHub to authorize the app creation. Morky will
        store the app credentials to access your repositories.
    </div>

    {#if error}
        <div role="alert" class="alert alert-error text-sm mb-4">{error}</div>
    {/if}

    {#if loading}
        <div class="flex items-center gap-2 text-sm text-base-content/50">
            <span class="icon-[lucide--loader-2] size-4 animate-spin"></span>
            Preparing...
        </div>
    {:else if githubUrl}
        <form bind:this={formEl} action={githubUrl} method="POST">
            <input type="hidden" name="manifest" value={manifest} />
            <div class="flex gap-2">
                <a
                    href="/git-sources"
                    class="btn btn-sm btn-ghost no-underline"
                >
                    <span class="icon-[lucide--arrow-left] size-3.5"></span> Cancel
                </a>
                <button type="submit" class="btn btn-sm btn-primary">
                    Create on GitHub
                </button>
            </div>
        </form>
    {/if}
</div>
