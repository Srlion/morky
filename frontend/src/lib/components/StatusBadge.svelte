<script>
  let { status } = $props();
  const config = $derived(getConfig(status));

  function getConfig(s) {
    switch (s) {
      case "running":
        return {
          icon: "icon-[lucide--circle-check]",
          badge: "badge-success",
          cls: "",
        };
      case "deploying":
      case "building":
        return {
          icon: "icon-[lucide--hammer]",
          badge: "badge-warning",
          cls: "animate-pulse",
        };
      case "failed":
      case "cancelled":
        return {
          icon: "icon-[lucide--circle-x]",
          badge: "badge-error",
          cls: "",
        };
      case "done":
        return {
          icon: "icon-[lucide--circle-check]",
          badge: "badge-info",
          cls: "",
        };
      case "idle":
        return {
          icon: "icon-[lucide--square]",
          badge: "badge-neutral",
          cls: "",
        };
      default:
        return {
          icon: "icon-[lucide--circle-dot]",
          badge: "badge-ghost",
          cls: "",
        };
    }
  }
</script>

<span class="badge text-[11px] gap-1 {config.badge} {config.cls}">
  <span class="{config.icon} size-3"></span>
  {status}
</span>
