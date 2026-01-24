<script lang="ts">
    import { Button } from "$lib/components/ui/button";
    import { Input } from "$lib/components/ui/input";
    import { Label } from "$lib/components/ui/label";
    import * as Dialog from "$lib/components/ui/dialog";
    import * as Alert from "$lib/components/ui/alert";
    import { Loader2, Shield, Unlock } from "@lucide/svelte";
    import { device } from "$lib/device/manager.svelte";

    let { open = $bindable(false), pin = $bindable("") } = $props();

    let error = $state("");
    let loading = $state(false);

    async function handleUnlock() {
        if (!pin) {
            error = "Please enter your PIN";
            return;
        }

        loading = true;
        const res = await device.getCredentials(pin);

        if (res.success) {
            open = false;
            // Reset state for next time
            // pin = ""; // Do not reset pin here, as it might be needed by the parent
            error = "";
        } else {
            error =
                typeof res.msg === "string" ? res.msg : "Failed to verify PIN";
        }

        loading = false;
    }
</script>

<Dialog.Root bind:open>
    <Dialog.Content class="sm:max-w-md">
        <Dialog.Header>
            <Dialog.Title class="flex items-center gap-2">
                <Shield class="h-5 w-5 text-primary" /> Authenticate to View Passkeys
            </Dialog.Title>
            <Dialog.Description
                >Enter your FIDO2 PIN to access and manage stored credentials.</Dialog.Description
            >
        </Dialog.Header>
        <div class="space-y-4 py-4">
            <div class="space-y-2">
                <Label for="auth-pin">Device PIN</Label>
                <Input
                    id="auth-pin"
                    type="password"
                    placeholder="Enter your PIN..."
                    bind:value={pin}
                    disabled={loading}
                    onkeydown={(e) => e.key === "Enter" && handleUnlock()}
                    autofocus
                />
            </div>
            {#if error}
                <Alert.Root
                    variant="destructive"
                    class="animate-in fade-in slide-in-from-top-1"
                >
                    <Alert.Description>{error}</Alert.Description>
                </Alert.Root>
            {/if}
        </div>
        <Dialog.Footer>
            <Button class="w-full" onclick={handleUnlock} disabled={loading}>
                {#if loading}
                    <Loader2 class="mr-2 h-4 w-4 animate-spin" />
                    Verifying...
                {:else}
                    <Unlock class="mr-2 h-4 w-4" /> Unlock Storage
                {/if}
            </Button>
        </Dialog.Footer>
    </Dialog.Content>
</Dialog.Root>
