<script>
    import "../app.css";
    import favicon from "$lib/assets/favicon.svg";
    import Sidebar from "$lib/components/Sidebar.svelte";
    import Toasts from "$lib/components/Toasts.svelte";
    import JobsIndicator from "$lib/components/JobsIndicator.svelte";

    let { children } = $props();
</script>

<svelte:head>
    <link rel="icon" href={favicon} />
</svelte:head>

<div class="flex flex-col md:flex-row h-screen overflow-hidden">
    <Sidebar />
    <main class="flex-1 overflow-y-auto">
        <div class="max-w-6xl mx-auto p-4 sm:p-6 md:p-8">
            {@render children()}
        </div>
    </main>
</div>

<Toasts />
<JobsIndicator />
