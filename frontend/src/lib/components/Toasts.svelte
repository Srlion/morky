<script module>
    let toasts = $state([]);
    let queue = [];
    let nextId = 0;

    export const config = $state({
        position: "bottom-right", // top-left | top-center | top-right | bottom-left | bottom-center | bottom-right
        max: 4,
    });

    const DEFAULTS = { success: 3500, error: 5000, info: 3500 };

    const POS = {
        "top-left": "top-6 left-6 items-start",
        "top-center": "top-6 left-1/2 -translate-x-1/2 items-center",
        "top-right": "top-6 right-6 items-end",
        "bottom-left": "bottom-6 left-6 items-start",
        "bottom-center": "bottom-6 left-1/2 -translate-x-1/2 items-center",
        "bottom-right": "bottom-6 right-6 items-end",
    };

    const ICONS = {
        success: "icon-[lucide--circle-check]",
        error: "icon-[lucide--circle-x]",
        info: "icon-[lucide--info]",
    };
    const ALERT = {
        success: "alert-success",
        error: "alert-error",
        info: "alert-info",
    };
    const PROG = {
        success: "bg-success-content/30",
        error: "bg-error-content/30",
        info: "bg-info-content/30",
    };

    function isBottom() {
        return config.position.startsWith("bottom");
    }

    function add(opts) {
        const duration = opts.duration ?? DEFAULTS[opts.type] ?? 3500;
        const t = {
            id: ++nextId,
            createdAt: Date.now(),
            duration,
            remaining: duration,
            paused: false,
            dismissing: false,
            ...opts,
        };
        if (toasts.filter((x) => !x.dismissing).length >= config.max) {
            queue.push(t);
        } else {
            toasts.push(t);
            startTimer(t);
        }
    }

    function flush() {
        while (
            queue.length &&
            toasts.filter((x) => !x.dismissing).length < config.max
        ) {
            const t = queue.shift();
            toasts.push(t);
            startTimer(t);
        }
    }

    function startTimer(t) {
        t._start = Date.now();
        t._timeout = setTimeout(() => dismiss(t.id), t.remaining);
    }

    function clearTimer(t) {
        if (t._timeout) {
            clearTimeout(t._timeout);
            t._timeout = null;
        }
    }

    function pause(id) {
        const t = toasts.find((x) => x.id === id);
        if (!t || t.paused) return;
        clearTimer(t);
        t.remaining -= Date.now() - t._start;
        t.paused = true;
    }

    function resume(id) {
        const t = toasts.find((x) => x.id === id);
        if (!t || !t.paused) return;
        t.paused = false;
        startTimer(t);
    }

    function dismiss(id) {
        const t = toasts.find((x) => x.id === id);
        if (!t || t.dismissing) return;
        t.dismissing = true;
        clearTimer(t);
        setTimeout(() => {
            const i = toasts.findIndex((x) => x.id === id);
            if (i !== -1) toasts.splice(i, 1);
            flush();
        }, 300);
    }

    export const toaster = {
        success: (o) => add({ type: "success", ...o }),
        error: (o) => add({ type: "error", ...o }),
        info: (o) => add({ type: "info", ...o }),
    };
</script>

{#if toasts.length}
    <div
        class="
      fixed z-50 flex {isBottom() ? 'flex-col-reverse' : 'flex-col'} gap-2 {POS[
            config.position
        ]}
    "
    >
        {#each toasts as t (t.id)}
            {@const icon = ICONS[t.type] ?? "icon-[lucide--info]"}
            <div
                class="
          alert {ALERT[t.type] ??
                    'alert-info'} shadow-lg pr-3 min-w-72 max-w-sm relative overflow-hidden
          {t.dismissing ? 'toast-exit' : 'toast-enter'}
        "
                class:exit-up={!isBottom() && t.dismissing}
                onmouseenter={() => pause(t.id)}
                onmouseleave={() => resume(t.id)}
                role="alert"
            >
                <span class="{icon} size-4 shrink-0"></span>
                <span class="text-sm flex-1">
                    {t.title}{t.description ? `: ${t.description}` : ""}
                </span>
                <button
                    class="btn btn-ghost btn-xs btn-square"
                    onclick={() => dismiss(t.id)}
                    aria-label="Dismiss"
                >
                    <span class="icon-[lucide--x] size-3.5"></span>
                </button>
                <div
                    class="
            absolute bottom-0 left-0 h-0.5 {PROG[t.type] ??
                        'bg-info-content/30'} toast-progress
          "
                    class:toast-paused={t.paused}
                    style="animation-duration: {t.duration}ms"
                ></div>
            </div>
        {/each}
    </div>
{/if}

<style>
    @keyframes toast-in-down {
        from {
            opacity: 0;
            transform: translateY(16px) scale(0.95);
        }
        to {
            opacity: 1;
            transform: translateY(0) scale(1);
        }
    }
    @keyframes toast-out-down {
        from {
            opacity: 1;
            transform: translateY(0) scale(1);
        }
        to {
            opacity: 0;
            transform: translateY(16px) scale(0.9);
        }
    }
    @keyframes toast-in-up {
        from {
            opacity: 0;
            transform: translateY(-16px) scale(0.95);
        }
        to {
            opacity: 1;
            transform: translateY(0) scale(1);
        }
    }
    @keyframes toast-out-up {
        from {
            opacity: 1;
            transform: translateY(0) scale(1);
        }
        to {
            opacity: 0;
            transform: translateY(-16px) scale(0.9);
        }
    }
    @keyframes progress-shrink {
        from {
            width: 100%;
        }
        to {
            width: 0%;
        }
    }

    .toast-enter {
        animation: toast-in-down 0.3s cubic-bezier(0.21, 1.02, 0.73, 1) forwards;
    }
    :global(.flex-col) .toast-enter {
        animation-name: toast-in-up;
    }
    .toast-exit {
        animation: toast-out-down 0.3s cubic-bezier(0.06, 0.71, 0.55, 1)
            forwards;
    }
    .toast-exit.exit-up {
        animation-name: toast-out-up;
    }
    .toast-progress {
        animation: progress-shrink linear forwards;
    }
    .toast-paused {
        animation-play-state: paused;
    }
</style>
