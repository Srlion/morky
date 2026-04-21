<script>
  import { fmtDateTime, fmtRelativeTime } from "$lib/util.js";

  let { source, onRename, onDelete } = $props();
</script>

<div
  class="group card bg-base-100 border border-base-300 p-4 transition-all hover:shadow-md"
>
  <div class="flex items-center gap-2 mb-2.5">
    <span class="badge badge-ghost text-[11px]">{source.provider}</span>
  </div>

  <div class="font-semibold text-sm flex items-center gap-2">
    {source.name}
    <button
      class="btn btn-square btn-ghost btn-xs opacity-0 group-hover:opacity-100 transition-opacity"
      onclick={() => onRename?.(source)}
      aria-label="Rename"
    >
      <span class="icon-[lucide--pencil] size-3"></span>
    </button>
  </div>

  <div
    class="mt-3 pt-3 border-t border-base-300 flex items-center justify-between"
  >
    <div
      class="text-xs text-base-content/40"
      title={fmtDateTime(source.created_at)}
    >
      Created {fmtRelativeTime(source.created_at)}
    </div>

    <button
      class="btn btn-square btn-ghost btn-xs text-error opacity-0 group-hover:opacity-100 transition-opacity"
      onclick={() => onDelete?.(source.id)}
      aria-label="Delete"
    >
      <span class="icon-[lucide--trash-2] size-3.5"></span>
    </button>
  </div>
</div>
