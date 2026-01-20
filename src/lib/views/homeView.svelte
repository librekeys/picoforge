<script lang="ts">
  import { Separator } from "$lib/components/ui/separator";
  import { Badge } from "$lib/components/ui/badge";
  import { Progress } from "$lib/components/ui/progress/index.js";
  import * as Card from "$lib/components/ui/card";
  import * as Alert from "$lib/components/ui/alert";

  import { device } from "$lib/device/manager.svelte";

  import {
    Cpu,
    Lock,
    LockOpen,
    Microchip,
    ShieldCheck,
    TriangleAlert,
    Shield,
  } from "@lucide/svelte";
  import NoDeviceStatus from "$lib/components/device/NoDeviceStatus.svelte";
</script>

<div class="space-y-6">
  <div>
    <h1 class="text-3xl font-bold tracking-tight">Device Overview</h1>
    <p class="text-muted-foreground">
      Quick view of your device status and specifications.
    </p>
  </div>

  {#if !device.connected}
    <NoDeviceStatus />
  {:else}
    <div class="grid gap-6 md:grid-cols-2">
      <Card.Root>
        <Card.Header>
          <Card.Title class="flex items-center gap-2">
            <Cpu class="h-5 w-5" />
            Device Information
          </Card.Title>
        </Card.Header>
        <Card.Content class="space-y-4">
          <div class="grid grid-cols-2 gap-4 text-sm">
            <div class="space-y-1">
              <p class="text-muted-foreground">Serial Number</p>
              <p class="font-mono font-medium">{device.info.serial}</p>
            </div>
            <div class="space-y-1">
              <p class="text-muted-foreground">Firmware Version</p>
              <p class="font-mono font-medium">
                v{device.info.firmwareVersion}
              </p>
            </div>
            <div class="space-y-1">
              <p class="text-muted-foreground">VID:PID</p>
              <p class="font-mono font-medium">
                {device.config.vid}:{device.config.pid}
              </p>
            </div>
            <div class="space-y-1">
              <p class="text-muted-foreground">Product Name</p>
              <p class="font-medium truncate">{device.config.productName}</p>
            </div>
            <div class="space-y-1">
              <p class="text-muted-foreground">Connection Method</p>
              <Badge variant="outline" class="font-mono">{device.method}</Badge>
            </div>
          </div>

          <Separator />

          <div class="space-y-2">
            <div class="flex justify-between text-sm">
              <span class="text-muted-foreground">Flash Memory</span>
              <span class="font-medium">
                {device.info.flashUsed} / {device.info.flashTotal} KB
              </span>
            </div>
            <Progress
              value={(device.info.flashUsed / device.info.flashTotal) * 100}
              class="h-2"
            />
          </div>
        </Card.Content>
      </Card.Root>

      <Card.Root>
        <Card.Header>
          <Card.Title class="flex items-center gap-2">
            <Shield class="h-5 w-5" />
            FIDO2 Information
          </Card.Title>
        </Card.Header>
        <Card.Content class="space-y-4">
          {#if device.fidoInfo}
            <div class="grid grid-cols-2 gap-4 text-sm">
              <div class="space-y-1">
                <p class="text-muted-foreground">FIDO Version</p>
                <p class="font-medium">
                  {device.fidoInfo.versions[0] || "N/A"}
                </p>
              </div>
              <div class="space-y-1">
                <p class="text-muted-foreground">PIN Set</p>
                <p class="font-medium">
                  {device.fidoInfo.options?.clientPin ? "Yes" : "No"}
                </p>
              </div>
              <div class="space-y-1">
                <p class="text-muted-foreground">Min PIN Length</p>
                <p class="font-medium">{device.fidoInfo.minPinLength}</p>
              </div>
              <div class="space-y-1">
                <p class="text-muted-foreground">Resident Keys</p>
                <p class="font-medium">
                  {device.fidoInfo.options?.rk ? "Supported" : "Not Supported"}
                </p>
              </div>
            </div>

            <Separator />

            <div class="space-y-1">
              <p class="text-muted-foreground text-sm">AAGUID</p>
              <p class="font-mono text-xs break-all">
                {device.fidoInfo.aaguid}
              </p>
            </div>
          {:else}
            <p class="text-muted-foreground text-sm">
              FIDO information not available
            </p>
          {/if}
        </Card.Content>
      </Card.Root>

      <Card.Root>
        <Card.Header>
          <Card.Title class="flex items-center gap-2">
            <Microchip class="h-5 w-5" />
            LED Configuration
          </Card.Title>
        </Card.Header>
        <Card.Content class="space-y-3 text-sm">
          <div class="flex justify-between">
            <span class="text-muted-foreground">LED GPIO Pin</span>
            <span class="font-medium">GPIO {device.config.ledGpio}</span>
          </div>
          <div class="flex justify-between">
            <span class="text-muted-foreground">LED Brightness</span>
            <span class="font-medium">{device.config.ledBrightness}</span>
          </div>
          <div class="flex justify-between">
            <span class="text-muted-foreground">Presence Touch Timeout</span>
            <span class="font-medium">{device.config.touchTimeout}s</span>
          </div>
          <div class="flex justify-between">
            <span class="text-muted-foreground">LED Dimmable</span>
            <Badge
              variant={device.config.ledDimmable ? "default" : "secondary"}
            >
              {device.config.ledDimmable ? "Yes" : "No"}
            </Badge>
          </div>
          <div class="flex justify-between">
            <span class="text-muted-foreground">LED Steady Mode</span>
            <Badge variant={device.config.ledSteady ? "default" : "secondary"}>
              {device.config.ledSteady ? "On" : "Off"}
            </Badge>
          </div>
        </Card.Content>
      </Card.Root>

      <Card.Root>
        <Card.Header>
          <Card.Title class="flex items-center gap-2">
            <ShieldCheck class="h-5 w-5" />
            Security Status
          </Card.Title>
        </Card.Header>
        <Card.Content class="space-y-3 text-sm">
          <div class="flex justify-between items-center">
            <span class="text-muted-foreground">Boot Mode</span>
            <div class="flex items-center gap-2">
              {#if device.security.secureBoot}
                <Lock class="h-3 w-3 text-green-500" />
              {:else}
                <LockOpen class="h-3 w-3 text-amber-500" />
              {/if}
              <Badge
                variant={device.security.secureBoot ? "default" : "secondary"}
              >
                {device.security.secureBoot ? "Secure Boot" : "Development"}
              </Badge>
            </div>
          </div>
          <div class="flex justify-between items-center">
            <span class="text-muted-foreground">Debug Interface</span>
            <span class="font-medium">
              {device.security.secureLock ? "Read-out Locked" : "Debug Enabled"}
            </span>
          </div>
          <div class="flex justify-between items-center">
            <span class="text-muted-foreground">Secure Lock</span>
            <Badge
              variant={device.security.confirmed ? "destructive" : "outline"}
            >
              {device.security.confirmed ? "Acknowledged" : "Pending"}
            </Badge>
          </div>
        </Card.Content>
      </Card.Root>
    </div>
  {/if}
</div>
