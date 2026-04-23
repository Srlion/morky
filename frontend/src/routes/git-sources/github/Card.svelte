<script>
  import { fmtDateTime, fmtRelativeTime } from "$lib/util.js";

  let { source, onRename, onDelete } = $props();

  let providerData = $derived(source?.provider_data ?? {});
  let installationId = $derived(providerData.installation_id ?? null);
  let ownerType = $derived(providerData.owner_type ?? "");
  let ownerLogin = $derived(providerData.owner_login ?? "");
  let htmlUrl = $derived(providerData.html_url ?? "");
</script>

<div
  class="
    group card bg-base-100 border border-base-300 p-4 transition-all hover:shadow-md
    {installationId ? '' : 'border-error'}
  "
>
  <div class="flex items-center gap-2 mb-2.5">
    <span class="badge badge-ghost text-[11px]">{source.provider}</span>
    {#if ownerType === "Organization"}
      <span class="badge badge-primary badge-outline text-[11px]">org</span>
    {/if}
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

  {#if ownerLogin}
    <div class="text-xs text-base-content/60 mt-0.5">{ownerLogin}</div>
  {/if}

  {#if !installationId}
    <div class="mt-3 alert alert-error text-xs py-2">
      App not installed - click Install to set up.
    </div>
  {/if}

  <div
    class="mt-3 pt-3 border-t border-base-300 flex items-center justify-between"
  >
    <div class="flex flex-col gap-1 text-xs">
      <div class="flex items-center gap-2">
        {#if htmlUrl}
          <a
            href={htmlUrl}
            target="_blank"
            class="link link-primary flex items-center gap-1"
          >
            App <span class="icon-[lucide--external-link] size-3"></span>
          </a>
        {/if}

        {#if installationId}
          {#if ownerType === "Organization"}
            <a
              href="https://github.com/organizations/{ownerLogin}/settings/installations/{installationId}"
              target="_blank"
              class="link link-primary flex items-center gap-1"
            >
              Install <span class="icon-[lucide--external-link] size-3"></span>
            </a>
          {:else}
            <a
              href="https://github.com/settings/installations/{installationId}"
              target="_blank"
              class="link link-primary flex items-center gap-1"
            >
              Manage <span class="icon-[lucide--external-link] size-3"></span>
            </a>
          {/if}
        {/if}
      </div>

      <div
        class="text-xs text-base-content/40"
        title={fmtDateTime(source.created_at)}
      >
        Created {fmtRelativeTime(source.created_at)}
      </div>
    </div>

    <div class="flex gap-1">
      {#if !installationId}
        <form action="/git-sources/github/{source.id}/install" method="POST">
          <button class="btn btn-xs btn-warning">Install</button>
        </form>
      {/if}

      <button
        class="btn btn-square btn-ghost btn-xs text-error opacity-0 group-hover:opacity-100 transition-opacity"
        onclick={() => onDelete?.(source.id)}
        aria-label="Delete"
      >
        <span class="icon-[lucide--trash-2] size-3.5"></span>
      </button>
    </div>
  </div>
</div>
