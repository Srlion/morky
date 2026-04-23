<script>
    import { page } from "$app/state";

    let open = $state(false);

    const links = [
        {
            href: "/projects",
            label: "Projects",
            icon: "icon-[lucide--folder-open]",
        },
        {
            href: "/git-sources",
            label: "Git Sources",
            icon: "icon-[lucide--git-branch]",
        },
        {
            href: "/monitoring",
            label: "Monitoring",
            icon: "icon-[lucide--activity]",
        },
        {
            href: "/backup",
            label: "Backup",
            icon: "icon-[lucide--archive]",
        },
        // Settings removed from here
    ];

    function isActive(href) {
        return (
            page.url.pathname === href ||
            page.url.pathname.startsWith(href + "/")
        );
    }
</script>

{#if open}
    <button
        class="fixed inset-0 z-40 bg-black/50 backdrop-blur-sm md:hidden"
        onclick={() => (open = false)}
        aria-label="Close sidebar"
    ></button>
{/if}

<header
    class="md:hidden flex items-center gap-3 px-4 py-3 bg-base-200 border-b border-base-300 shrink-0"
>
    <button
        class="btn btn-square btn-sm btn-ghost"
        onclick={() => (open = !open)}
        aria-label="Toggle sidebar"
    >
        <span class="icon-[lucide--panel-left] size-4"></span>
    </button>
    <span class="font-bold text-sm">Morky</span>
</header>

<aside
    class="w-56 shrink-0 flex flex-col bg-base-200 border-r border-base-300 max-md:fixed max-md:inset-y-0 max-md:left-0 max-md:z-50 max-md:transition-transform max-md:duration-200 {open
        ? ''
        : 'max-md:-translate-x-full'}"
>
    <a
        href="/"
        class="flex items-center gap-2.5 px-5 py-5 text-lg font-bold no-underline"
    >
        <span
            class="size-8 rounded-lg bg-primary text-primary-content flex items-center justify-center text-sm font-bold"
            >K</span
        >
        Morky
    </a>

    <div class="px-4 mb-3"><div class="divider my-0"></div></div>

    <nav class="flex-1 px-3 flex flex-col gap-1">
        {#each links as { href, label, icon }}
            <a
                {href}
                class="flex items-center gap-2.5 px-3 py-2 rounded-lg text-sm transition-all duration-150 {isActive(
                    href,
                )
                    ? 'bg-primary/15 text-primary font-medium'
                    : 'text-base-content/70 hover:bg-base-300 hover:text-base-content'}"
                onclick={() => (open = false)}
            >
                <span class="{icon} size-4"></span>
                {label}
            </a>
        {/each}
    </nav>

    <!-- Settings with Logout -->
    <div class="border-t border-base-300 px-3 py-3 mt-auto">
        <div class="flex flex-col gap-1">
            <a
                href="/settings"
                class="flex items-center gap-2.5 px-3 py-2 rounded-lg text-sm transition-all duration-150 {isActive(
                    '/settings',
                )
                    ? 'bg-primary/15 text-primary font-medium'
                    : 'text-base-content/70 hover:bg-base-300 hover:text-base-content'}"
                onclick={() => (open = false)}
            >
                <span class="icon-[lucide--settings] size-4"></span>
                Settings
            </a>

            <form action="/auth/logout" method="POST">
                <button
                    type="submit"
                    class="flex items-center gap-2.5 w-full px-3 py-2 rounded-lg text-sm text-base-content/70 hover:bg-error/10 hover:text-error transition-all duration-150"
                >
                    <span class="icon-[lucide--log-out] size-4"></span>
                    Logout
                </button>
            </form>
        </div>
    </div>
</aside>
