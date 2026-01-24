<script lang="ts">
    import * as AlertDialog from "$lib/components/ui/alert-dialog";
    import { Button } from "$lib/components/ui/button";
    import { Input } from "$lib/components/ui/input";
    import { Label } from "$lib/components/ui/label";
    import { Loader2 } from "@lucide/svelte";
    import type { StoredCredential } from "$lib/device/types.svelte";

    interface Props {
        open: boolean;
        credential: StoredCredential | null;
        loading: boolean;
        onDelete: () => void;
    }

    let { open = $bindable(), credential, loading, onDelete }: Props = $props();

    let deleteConfirmationInput = $state("");

    $effect(() => {
        if (open) {
            deleteConfirmationInput = "";
        }
    });
</script>

<AlertDialog.Root bind:open>
    <AlertDialog.Content>
        <AlertDialog.Header>
            <AlertDialog.Title>Are you sure?</AlertDialog.Title>
            <AlertDialog.Description>
                This action cannot be undone. This will permanently delete the
                passkey for
                <span class="font-semibold text-foreground"
                    >{credential?.rpId}</span
                >.
            </AlertDialog.Description>
        </AlertDialog.Header>

        <div class="py-4 space-y-3">
            <Label for="confirm-delete">
                Type <span
                    class="font-mono text-xs bg-muted px-1 py-0.5 rounded"
                    >{credential?.rpId}</span
                > to confirm
            </Label>
            <Input
                id="confirm-delete"
                bind:value={deleteConfirmationInput}
                placeholder={credential?.rpId}
                autocomplete="off"
                class="font-mono"
            />
        </div>

        <AlertDialog.Footer>
            <AlertDialog.Cancel disabled={loading}>Cancel</AlertDialog.Cancel>
            <Button
                variant="destructive"
                onclick={onDelete}
                disabled={deleteConfirmationInput !== credential?.rpId ||
                    loading}
            >
                {#if loading}
                    <Loader2 class="mr-2 h-4 w-4 animate-spin" />
                    Deleting...
                {:else}
                    Delete Passkey
                {/if}
            </Button>
        </AlertDialog.Footer>
    </AlertDialog.Content>
</AlertDialog.Root>
