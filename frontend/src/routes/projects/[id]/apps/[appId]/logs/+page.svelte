<script>
    import { page } from "$app/state";
    import { onDestroy, onMount, tick } from "svelte";
    import { api } from "$lib/api";
    import Convert from "ansi-to-html";

    const convert = new Convert({ escapeXML: true });

    let log = $state("Loading...");
    let logHtml = $derived(convert.toHtml(log));
    let logEl = $state();
    let timer;

    const appId = $derived(page.params.appId);

    onMount(() => {
        refresh();
        timer = setInterval(refresh, 3000);
    });
    onDestroy(() => clearInterval(timer));

    async function refresh() {
        const res = await api.get(`/apps/${appId}/container-logs`);
        log = res.log || "(no logs)";
        await tick();
        if (logEl) logEl.scrollTop = logEl.scrollHeight;
    }
</script>

<div class="flex items-center justify-between mb-3">
    <span class="text-xs text-base-content/50">
        Container logs for <kbd class="kbd kbd-xs">app-{appId}</kbd>
    </span>
    <button class="btn btn-xs btn-ghost" onclick={refresh}>
        <span class="icon-[lucide--refresh-cw] size-3"></span> Refresh
    </button>
</div>

<pre
    bind:this={logEl}
    class="bg-base-200 border border-base-300 rounded-2xl p-4 text-xs font-mono max-h-[70vh] overflow-auto whitespace-pre-wrap text-base-content/60">{@html logHtml}</pre>
