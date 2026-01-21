<script lang="ts">
    import { Input } from "$lib/components/ui/input";
    import { Label } from "$lib/components/ui/label";
    import * as AlertDialog from "$lib/components/ui/alert-dialog";

    import { configViewState as configState } from "$lib/state/configState.svelte";
</script>

<AlertDialog.Root bind:open={configState.authPinDialogOpen}>
    <AlertDialog.Content
        onOpenAutoFocus={(e) => {
            e.preventDefault();
            document.getElementById("auth-pin")?.focus();
        }}
    >
        <AlertDialog.Header>
            <AlertDialog.Title>Authentication Required</AlertDialog.Title>
            <AlertDialog.Description>
                Please enter your FIDO2 PIN to authorize the configuration
                change.
            </AlertDialog.Description>
        </AlertDialog.Header>
        <div class="space-y-4 py-4">
            <div class="space-y-2">
                <Label for="auth-pin">FIDO2 PIN</Label>
                <Input
                    id="auth-pin"
                    type="password"
                    bind:value={configState.authPin}
                    placeholder="Enter your PIN"
                    onkeydown={(e) =>
                        e.key === "Enter" && configState.confirmAuthPinSave()}
                />
            </div>
            {#if configState.authPinError}
                <p class="text-sm text-destructive">
                    {configState.authPinError}
                </p>
            {/if}
        </div>
        <AlertDialog.Footer>
            <AlertDialog.Cancel
                onclick={() => (configState.authPinDialogOpen = false)}
                >Cancel</AlertDialog.Cancel
            >
            <AlertDialog.Action onclick={() => configState.confirmAuthPinSave()}
                >Confirm</AlertDialog.Action
            >
        </AlertDialog.Footer>
    </AlertDialog.Content>
</AlertDialog.Root>
