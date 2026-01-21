<script lang="ts">
  // import { Button } from "$lib/components/ui/button";
  import { Input } from "$lib/components/ui/input";
  import { Label } from "$lib/components/ui/label";
  import { Slider } from "$lib/components/ui/slider/index.js";
  import * as AlertDialog from "$lib/components/ui/alert-dialog";

  import { configViewState as configState } from "$lib/state/configState.svelte";
</script>

<AlertDialog.Root bind:open={configState.minPinDialogOpen}>
  <AlertDialog.Content
    onOpenAutoFocus={(e) => {
      e.preventDefault();
      document.getElementById("min-pin-current")?.focus();
    }}
  >
    <AlertDialog.Header>
      <AlertDialog.Title>Update Minimum PIN Length</AlertDialog.Title>
      <AlertDialog.Description>
        Set the minimum allowed PIN length (4-63 characters) and enter a new PIN
        that meets this requirement.
      </AlertDialog.Description>
    </AlertDialog.Header>
    <div class="space-y-4 py-4">
      <div class="space-y-2">
        <Label for="min-pin-length"
          >Minimum PIN Length ({configState.minPinLength})</Label
        >
        <Slider
          type="single"
          bind:value={configState.minPinLength}
          min={4}
          max={63}
          step={1}
        />
      </div>
      <div class="space-y-2">
        <Label for="min-pin-current">Current PIN</Label>
        <Input
          id="min-pin-current"
          type="password"
          bind:value={configState.minPinCurrentPin}
          placeholder="Enter current PIN"
        />
      </div>
      <div class="space-y-2">
        <Label for="min-pin-new"
          >New PIN (min {configState.minPinLength} chars)</Label
        >
        <Input
          id="min-pin-new"
          type="password"
          bind:value={configState.minPinNewPin}
          placeholder="Enter new PIN"
        />
      </div>
      <div class="space-y-2">
        <Label for="min-pin-confirm">Confirm New PIN</Label>
        <Input
          id="min-pin-confirm"
          type="password"
          bind:value={configState.minPinConfirmPin}
          placeholder="Confirm new PIN"
        />
      </div>
      {#if configState.minPinError}
        <p class="text-sm text-destructive">{configState.minPinError}</p>
      {/if}
    </div>
    <AlertDialog.Footer>
      <AlertDialog.Cancel onclick={() => (configState.minPinDialogOpen = false)}
        >Cancel</AlertDialog.Cancel
      >
      <AlertDialog.Action onclick={configState.handleMinPinChange}
        >Update</AlertDialog.Action
      >
    </AlertDialog.Footer>
  </AlertDialog.Content>
</AlertDialog.Root>
