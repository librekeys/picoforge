<script lang="ts">
  // import { Button } from "$lib/components/ui/button";
  import { Input } from "$lib/components/ui/input";
  import { Label } from "$lib/components/ui/label";
  import * as AlertDialog from "$lib/components/ui/alert-dialog";

  import { configViewState as configState } from "$lib/state/configState.svelte";
</script>

<AlertDialog.Root bind:open={configState.setPinDialogOpen}>
  <AlertDialog.Content
    onOpenAutoFocus={(e) => {
      e.preventDefault();
      const id = configState.isSettingPin ? "new-pin" : "current-pin";
      document.getElementById(id)?.focus();
    }}
  >
    <AlertDialog.Header>
      <AlertDialog.Title
        >{configState.isSettingPin
          ? "Set PIN"
          : "Change PIN"}</AlertDialog.Title
      >
      <AlertDialog.Description>
        {configState.isSettingPin
          ? "Set a PIN for your FIDO2 device. Minimum length is " +
            (configState.minPinLength || 4) +
            " characters."
          : "Enter your current PIN and choose a new one."}
      </AlertDialog.Description>
    </AlertDialog.Header>
    <div class="space-y-4 py-4">
      {#if !configState.isSettingPin}
        <div class="space-y-2">
          <Label for="current-pin">Current PIN</Label>
          <Input
            id="current-pin"
            type="password"
            bind:value={configState.currentPin}
            placeholder="Enter current PIN"
          />
        </div>
      {/if}
      <div class="space-y-2">
        <Label for="new-pin">New PIN</Label>
        <Input
          id="new-pin"
          type="password"
          bind:value={configState.newPin}
          placeholder="Enter new PIN"
        />
      </div>
      <div class="space-y-2">
        <Label for="confirm-pin">Confirm New PIN</Label>
        <Input
          id="confirm-pin"
          type="password"
          bind:value={configState.confirmPin}
          placeholder="Confirm new PIN"
        />
      </div>
      {#if configState.pinError}
        <p class="text-sm text-destructive">{configState.pinError}</p>
      {/if}
    </div>
    <AlertDialog.Footer>
      <AlertDialog.Cancel onclick={() => (configState.setPinDialogOpen = false)}
        >Cancel</AlertDialog.Cancel
      >
      <AlertDialog.Action onclick={() => configState.handlePinChange()}
        >Confirm</AlertDialog.Action
      >
    </AlertDialog.Footer>
  </AlertDialog.Content>
</AlertDialog.Root>
