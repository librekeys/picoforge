<script lang="ts">
  import { Button } from "$lib/components/ui/button";
  import { Label } from "$lib/components/ui/label";
  import { Switch } from "$lib/components/ui/switch";
  import { Separator } from "$lib/components/ui/separator";
  import * as Card from "$lib/components/ui/card";
  import * as Alert from "$lib/components/ui/alert";

  import { device } from "$lib/device/manager.svelte";
  import { logger } from "$lib/services/log.svelte";

  import { Lock, TriangleAlert } from "@lucide/svelte";

  async function lockDevice() {
    logger.add(
      "Action Blocked: Secure Boot toggle attempted but feature is disabled.",
      "warning",
    );
    return;
  }
</script>

<div class="space-y-4">
  <div>
    <h1 class="text-3xl font-bold tracking-tight text-destructive">
      Secure Boot
    </h1>
    <p class="text-muted-foreground">
      Permanently lock this device to the current firmware vendor.
    </p>
  </div>

  <Alert.Root variant="destructive">
    <TriangleAlert class="h-4 w-4" />
    <Alert.Title>Feature Unstable</Alert.Title>
    <Alert.Description
      >This feature is currently under work and disabled for safety.</Alert.Description
    >
  </Alert.Root>

  <Card.Root class="border-destructive/30 opacity-60">
    <Card.Header>
      <Card.Title>Lock Settings</Card.Title>
    </Card.Header>
    <Card.Content class="space-y-4 pointer-events-none">
      <div class="flex items-center justify-between space-x-2">
        <div class="flex flex-col space-y-1">
          <Label for="secure-boot">Enable Secure Boot</Label>
          <p class="font-normal text-xs text-muted-foreground">
            Verifies firmware signature on startup
          </p>
        </div>
        <Switch
          id="secure-boot"
          bind:checked={device.security.secureBoot}
          disabled={true}
          title="Status only - cannot be toggled via software"
        />
      </div>

      <div class="flex items-center justify-between space-x-2">
        <div class="flex flex-col space-y-1">
          <Label for="secure-lock">Secure Lock</Label>
          <p class="font-normal text-xs text-muted-foreground">
            Prevents reading key material via debug ports
          </p>
        </div>
        <Switch
          id="secure-lock"
          bind:checked={device.security.secureLock}
          disabled={true}
        />
      </div>

      <Separator />

      <div
        class="flex items-center space-x-2 bg-destructive/10 p-4 rounded-md border border-destructive/20"
      >
        <Switch
          id="confirm"
          bind:checked={device.security.confirmed}
          disabled={true}
        />
        <Label for="confirm" class="text-destructive font-medium"
          >I understand the risks of bricking my device.</Label
        >
      </div>
    </Card.Content>
    <Card.Footer class="border-t bg-muted/20 px-6 py-4 flex justify-end">
      <Button variant="destructive" disabled={true} onclick={lockDevice}>
        <Lock class="mr-2 h-4 w-4" />
        Permanently Lock Device
      </Button>
    </Card.Footer>
  </Card.Root>
</div>
