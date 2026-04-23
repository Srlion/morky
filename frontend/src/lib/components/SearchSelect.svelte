<!-- $lib/components/SearchSelect.svelte -->
<script>
  import { tick } from "svelte";

  let {
    value = $bindable(""),
    items = [],
    loading = false,
    placeholder = "Select...",
    searchPlaceholder = "Search...",
    labelKey = null,
    onopen = null,
    class: cls = "",
  } = $props();

  let search = $state("");
  let open = $state(false);
  let dropUp = $state(false);
  let rootEl = $state();
  let searchEl = $state();

  let filtered = $derived(
    search
      ? items.filter((i) =>
          label(i).toLowerCase().includes(search.toLowerCase()),
        )
      : items,
  );

  function label(item) {
    return labelKey ? item[labelKey] : String(item);
  }

  async function toggle() {
    if (open) return close();
    onopen?.();
    open = true;

    await tick();

    if (rootEl) {
      const rect = rootEl.getBoundingClientRect();
      const spaceBelow = window.innerHeight - rect.bottom;
      const dropdownEl = rootEl.querySelector(".absolute");
      const dropdownHeight = dropdownEl?.offsetHeight ?? 200;
      dropUp = spaceBelow < dropdownHeight + 10;
    }

    searchEl?.focus();
  }

  function close() {
    open = false;
    search = "";
    dropUp = false;
  }

  function pick(item) {
    value = item;
    close();
  }
</script>

<svelte:window
  onclick={(e) => open && !rootEl?.contains(e.target) && close()}
  onkeydown={(e) => e.key === "Escape" && open && close()}
/>

<div bind:this={rootEl} class="relative w-full {cls}">
  <button
    type="button"
    class="select select-bordered w-full text-left font-mono text-sm"
    onclick={toggle}
  >
    {value ? label(value) : placeholder}
  </button>

  {#if open}
    <div
      class="absolute z-50 w-full bg-base-100 border border-base-300 rounded-box shadow-lg"
      class:mt-1={!dropUp}
      class:bottom-full={dropUp}
      class:mb-1={dropUp}
    >
      <div class="p-2 border-b border-base-300">
        <input
          bind:this={searchEl}
          type="text"
          class="input input-bordered input-sm w-full font-mono"
          placeholder={searchPlaceholder}
          bind:value={search}
        />
      </div>
      <ul
        class="menu w-full max-h-48 overflow-y-auto py-1 text-sm font-mono [&_li]:w-full [&_li>button]:w-full"
      >
        {#if loading}
          <li class="px-3 py-4 flex justify-center">
            <span class="loading loading-spinner loading-sm"></span>
          </li>
        {:else if filtered.length === 0}
          <li class="px-3 py-2 text-xs text-base-content/50">No results</li>
        {:else}
          {#each filtered as item}
            <li class="w-full">
              <button
                class="
                  w-full justify-start {item === value ||
                (labelKey && item[labelKey] === value?.[labelKey])
                  ? 'active'
                  : ''}
                "
                onclick={(e) => {
                  e.stopPropagation();
                  pick(item);
                }}
              >
                {label(item)}{#if item.private}
                  🔒{/if}
              </button>
            </li>
          {/each}
        {/if}
      </ul>
    </div>
  {/if}
</div>
