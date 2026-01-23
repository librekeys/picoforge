<script lang="ts">
  // import { Button } from "$lib/components/ui/button";
  // import { Input } from "$lib/components/ui/input";
  // import { Label } from "$lib/components/ui/label";
  import * as AlertDialog from "$lib/components/ui/alert-dialog";

  import { configViewState as state } from "$lib/state/configState.svelte";
</script>

<AlertDialog.Root bind:open={state.dialogOpen}>
  <AlertDialog.Content>
    <AlertDialog.Header>
      <AlertDialog.Title
        class={state.dialogType === "error" ? "text-red-500" : ""}
        >{state.dialogTitle}</AlertDialog.Title
      >
      <AlertDialog.Description>
        {state.dialogMessage}
      </AlertDialog.Description>
    </AlertDialog.Header>
    <AlertDialog.Footer>
      <AlertDialog.Action onclick={() => (state.dialogOpen = false)}
        >Okay</AlertDialog.Action
      >
    </AlertDialog.Footer>
  </AlertDialog.Content>
</AlertDialog.Root>
