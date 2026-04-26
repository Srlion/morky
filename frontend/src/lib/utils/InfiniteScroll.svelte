<script>
    import { onDestroy, tick } from "svelte";
    let {
        threshold = 0,
        horizontal = false,
        elementScroll = null,
        hasMore = true,
        reverse = false,
        onLoadMore,
    } = $props();

    // "idle" -> "ready" (at edge, showing hint) -> "armed" (hint visible long enough, next scroll loads) -> "loading"
    let phase = $state("idle");
    let component = $state();
    let beforeScrollHeight = $state(0);
    let beforeScrollTop = $state(0);
    let readyTimer = null;
    const READY_DELAY = 400; // ms the hint stays visible before load can fire

    $effect(() => {
        if (component || elementScroll) {
            const element = elementScroll ?? component.parentNode;
            if (reverse) {
                element.scrollTop = element.scrollHeight;
            }
            element.addEventListener("scroll", onScroll);
            element.addEventListener("resize", onScroll);
            return () => {
                element.removeEventListener("scroll", onScroll);
                element.removeEventListener("resize", onScroll);
            };
        }
    });

    // Always listen for wheel/touchmove - fires even when at the edge
    // where scroll events won't fire (scrollTop already 0 or maxed)
    $effect(() => {
        const element = getElement();
        if (!element) return;
        element.addEventListener("wheel", onEdgeWheel, { passive: true });
        element.addEventListener("touchmove", onEdgeTouch, { passive: true });
        return () => {
            element.removeEventListener("wheel", onEdgeWheel);
            element.removeEventListener("touchmove", onEdgeTouch);
        };
    });

    // After a load completes, restore scroll and re-check edge
    $effect(() => {
        if (phase === "idle" && hasMore) {
            tick().then(checkEdge);
        }
    });

    function getElement() {
        return elementScroll ?? component?.parentNode;
    }

    function getOffset(element) {
        if (reverse) {
            return horizontal ? element.scrollLeft : element.scrollTop;
        }
        return horizontal
            ? element.scrollWidth - element.clientWidth - element.scrollLeft
            : element.scrollHeight - element.clientHeight - element.scrollTop;
    }

    function enterReady() {
        if (phase !== "idle") return;
        phase = "ready";
        if (readyTimer) clearTimeout(readyTimer);
        readyTimer = setTimeout(() => {
            if (phase === "ready") phase = "armed";
            readyTimer = null;
        }, READY_DELAY);
    }

    function leaveReady() {
        if (readyTimer) {
            clearTimeout(readyTimer);
            readyTimer = null;
        }
        phase = "idle";
    }

    function checkEdge() {
        const element = getElement();
        if (!element || !hasMore || phase !== "idle") return;
        const offset = getOffset(element);
        if (offset <= threshold) {
            enterReady();
        }
    }

    function onEdgeWheel(e) {
        if (phase !== "armed" || !hasMore) return;
        const element = getElement();
        if (!element) return;

        const offset = getOffset(element);
        if (offset > threshold) return;

        const scrollingTowardEdge = reverse
            ? horizontal
                ? e.deltaX < 0
                : e.deltaY < 0
            : horizontal
              ? e.deltaX > 0
              : e.deltaY > 0;

        if (scrollingTowardEdge) {
            triggerLoad();
        }
    }

    let lastTouchY = null;
    function onEdgeTouch(e) {
        if (!hasMore) return;
        const touch = e.touches[0];
        const pos = horizontal ? touch.clientX : touch.clientY;
        if (lastTouchY !== null && phase === "armed") {
            const element = getElement();
            if (!element) {
                lastTouchY = pos;
                return;
            }
            const offset = getOffset(element);
            if (offset <= threshold) {
                const delta = lastTouchY - pos;
                const scrollingTowardEdge = reverse ? delta < 0 : delta > 0;
                if (scrollingTowardEdge) {
                    triggerLoad();
                }
            }
        }
        lastTouchY = pos;
    }

    function triggerLoad() {
        const element = getElement();
        if (!element || phase !== "armed" || !hasMore) return;

        phase = "loading";
        beforeScrollHeight = element.scrollHeight;
        beforeScrollTop = element.scrollTop;
        lastTouchY = null;

        const result = onLoadMore?.();
        const done = () => {
            if (reverse) {
                const el = getElement();
                if (el && beforeScrollHeight > 0) {
                    el.scrollTop =
                        el.scrollHeight - beforeScrollHeight + beforeScrollTop;
                    beforeScrollHeight = 0;
                }
            }
            phase = "idle";
        };

        if (result && typeof result.then === "function") {
            result.finally(done);
        } else {
            requestAnimationFrame(done);
        }
    }

    const onScroll = () => {
        if (!hasMore) return;
        const element = getElement();
        if (!element) return;
        const offset = getOffset(element);

        if (offset <= threshold) {
            if (phase === "idle") {
                enterReady();
            }
        } else {
            if (phase === "ready" || phase === "armed") {
                leaveReady();
                lastTouchY = null;
            }
        }
    };

    onDestroy(() => {
        if (readyTimer) clearTimeout(readyTimer);
        if (component || elementScroll) {
            const element = elementScroll ?? component.parentNode;
            element.removeEventListener("scroll", onScroll);
            element.removeEventListener("resize", onScroll);
        }
    });
</script>

<div bind:this={component} style="width:0;height:0"></div>
{#if hasMore}
    {#if phase === "ready" || phase === "armed"}
        <div
            class="flex items-center justify-center py-2 text-base-content/30 text-xs select-none"
        >
            {reverse ? "↑ Scroll up for more" : "↓ Scroll down for more"}
        </div>
    {:else if phase === "loading"}
        <div
            class="flex items-center justify-center gap-2 py-2 text-base-content/40 text-xs"
        >
            <span class="loading loading-spinner loading-xs"></span>
            Loading…
        </div>
    {/if}
{/if}
