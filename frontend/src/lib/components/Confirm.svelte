<script>
  let { message = "Are you sure?", onconfirm } = $props();
  let el;
  export function show() {
    el.showModal();
  }
</script>

<dialog bind:this={el} class="modal">
  <div class="modal-box">
    <p class="text-sm">{message}</p>
    <div class="modal-action">
      <button class="btn btn-sm" onclick={() => el.close()}>Cancel</button>
      <button
        class="btn btn-sm btn-error"
        onclick={() => {
          el.close();
          onconfirm();
        }}
      >
        Confirm
      </button>
    </div>
  </div>
  <form method="dialog" class="modal-backdrop"><button>close</button></form>
</dialog>
