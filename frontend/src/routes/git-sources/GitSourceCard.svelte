<script>
  import GithubSourceCard from "./github/Card.svelte";
  import GenericSourceCard from "./GenericSourceCard.svelte";

  let { source, onRename, onDelete } = $props();
</script>

{#if source?.provider === "github"}
  <GithubSourceCard {source} {onRename} {onDelete} />
{:else}
  <GenericSourceCard {source} {onRename} {onDelete} />
{/if}
