<script module>
  let s = $state({
    open: false,
    endpoint: "",
    envVars: "",
    projectEnvVars: "",
  });

  export const envDialog = {
    get open() {
      return s.open;
    },
    get endpoint() {
      return s.endpoint;
    },
    get envVars() {
      return s.envVars;
    },
    set envVars(v) {
      s.envVars = v;
    },
    get projectEnvVars() {
      return s.projectEnvVars;
    },
    show(endpoint, envVars = "", projectEnvVars = "") {
      Object.assign(s, { open: true, endpoint, envVars, projectEnvVars });
    },
    close() {
      s.open = false;
    },
  };
</script>

<script>
  import { invalidateAll } from "$app/navigation";
  import { api } from "$lib/api";
  import { toaster } from "$lib/components/Toasts.svelte";
  import { parse } from "$lib/utils/envParser";

  const inputId = "envvars-input";
  const previewId = "envvars-preview";

  let el = $state();
  let parsed = $state(/** @type {Record<string, string>} */ ({}));
  let parsedProject = $state(/** @type {Record<string, string>} */ ({}));

  $effect(() => {
    parsed = parse(envDialog.envVars || "");
  });

  $effect(() => {
    parsedProject = parse(envDialog.projectEnvVars || "");
  });

  $effect(() => {
    if (envDialog.open) el?.showModal();
    else el?.close();
  });

  let merged = $derived(() => {
    const entries = [];
    const appKeys = new Set(Object.keys(parsed));

    for (const [key, value] of Object.entries(parsedProject)) {
      if (appKeys.has(key)) {
        entries.push({ key, value: parsed[key], source: "override" });
      } else {
        entries.push({ key, value, source: "project" });
      }
    }

    for (const [key, value] of Object.entries(parsed)) {
      if (!(key in parsedProject)) {
        entries.push({ key, value, source: "app" });
      }
    }

    return entries;
  });

  function onInput(event) {
    const value = event.target.value;
    envDialog.envVars = value;
    parsed = parse(value || "");
  }

  async function save() {
    try {
      await api.put(envDialog.endpoint, envDialog.envVars);
      toaster.success({ title: "Env vars saved" });
      envDialog.close();
      await invalidateAll();
    } catch (e) {
      toaster.error({ title: e.message });
    }
  }
</script>

<dialog bind:this={el} class="modal" onclose={() => envDialog.close()}>
  <div class="modal-box max-w-4xl">
    <h3 class="text-lg font-semibold mb-1">Environment Variables</h3>
    <p class="text-xs text-base-content/50 mb-4">
      One per line: <kbd class="kbd kbd-xs">KEY=VALUE</kbd>
    </p>

    <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
      <div class="flex flex-col">
        <label class="text-xs font-semibold mb-1" for={inputId}>Input</label>
        <textarea
          id={inputId}
          class="textarea textarea-bordered w-full font-mono text-xs"
          rows="12"
          value={envDialog.envVars}
          oninput={onInput}
          placeholder="DATABASE_URL=..."
        ></textarea>
      </div>

      <div class="flex flex-col">
        <div class="flex items-center justify-between mb-1">
          <label class="text-xs font-semibold" for={previewId}>Preview</label>
          {#if merged().length > 0}
            <span class="badge badge-ghost badge-xs"
              >{merged().length} var{merged().length === 1 ? "" : "s"}</span
            >
          {/if}
        </div>
        <div
          id={previewId}
          class="border border-base-300 rounded-lg w-full h-full overflow-auto bg-base-200/40"
          style="min-height: 12lh"
        >
          {#if merged().length === 0}
            <div
              class="flex items-center justify-center h-full text-base-content/30 text-xs italic py-12"
            >
              Paste env vars on the left to preview
            </div>
          {:else}
            <ul class="flex flex-col gap-px">
              {#each merged() as { key, value, source }, i}
                <li
                  class="
                    flex items-baseline gap-3 px-3 py-2 {i % 2 === 0
                    ? 'bg-base-100/60'
                    : ''}
                  "
                >
                  <code
                    class="shrink-0 px-2 py-px rounded bg-primary/10 text-primary text-xs font-bold select-all"
                    >{key}</code
                  >
                  <div class="flex-1 min-w-0">
                    {#if value}
                      <code
                        class="text-xs text-base-content/70 break-all select-all"
                        >{value}</code
                      >
                    {:else}
                      <span class="text-xs text-warning/60 italic">empty</span>
                    {/if}
                    {#if source === "project"}
                      <span class="text-[10px] text-base-content/30 ml-1"
                        >from project</span
                      >
                    {:else if source === "override"}
                      <span class="text-[10px] text-warning/60 ml-1"
                        >overrides project</span
                      >
                    {/if}
                  </div>
                </li>
              {/each}
            </ul>
          {/if}
        </div>
      </div>
    </div>

    <div class="modal-action mt-4">
      <button class="btn btn-sm" onclick={() => envDialog.close()}>
        Cancel
      </button>
      <button class="btn btn-sm btn-primary" onclick={save}> Save </button>
    </div>
  </div>
  <form method="dialog" class="modal-backdrop"><button>close</button></form>
</dialog>
