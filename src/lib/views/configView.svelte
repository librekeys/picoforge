<script lang="ts">
  import { Button } from "$lib/components/ui/button";
  import { Input } from "$lib/components/ui/input";
  import { Label } from "$lib/components/ui/label";
  import { Switch } from "$lib/components/ui/switch";
  import { Separator } from "$lib/components/ui/separator";
  import { Slider } from "$lib/components/ui/slider/index.js";
  import * as Card from "$lib/components/ui/card";
  import * as Alert from "$lib/components/ui/alert";
  import * as Select from "$lib/components/ui/select";

  import { device } from "$lib/device/manager.svelte";
  import { LED_DRIVERS, VENDORS } from "$lib/device/constants.svelte";

  import {
    Microchip,
    RefreshCw,
    Save,
    Settings,
    Tag,
    TriangleAlert,
    X,
  } from "@lucide/svelte";

  import { configViewState as state } from "$lib/state/configState.svelte";
  import NoDeviceStatus from "$lib/components/device/NoDeviceStatus.svelte";
</script>

<div class="space-y-4">
  <div>
    <h1 class="text-3xl font-bold tracking-tight">Configuration</h1>
    <p class="text-muted-foreground">Customize device settings and behavior.</p>
  </div>

  {#if !device.connected}
    <NoDeviceStatus
      message="Connect your device to access configuration options."
    />
  {:else}
    <div class="grid gap-6 lg:grid-cols-2">
      <Card.Root>
        <Card.Header>
          <Card.Title class="flex items-center gap-2">
            <Tag class="h-5 w-5" />
            Identity
          </Card.Title>
          <Card.Description>USB Identification settings</Card.Description>
        </Card.Header>
        <Card.Content class="space-y-4">
          <div class="space-y-2">
            <Label>Vendor Preset</Label>
            <Select.Root
              type="single"
              value={device.selectedVendor}
              onValueChange={(v) => device.setVendor(v)}
              disabled={!device.connected}
            >
              <Select.Trigger class="w-full">
                {VENDORS.find((v) => v.value === device.selectedVendor)
                  ?.label ?? "Select a vendor"}
              </Select.Trigger>
              <Select.Content>
                {#each VENDORS as vendor}
                  <Select.Item value={vendor.value} label={vendor.label}>
                    {vendor.label}
                  </Select.Item>
                {/each}
              </Select.Content>
            </Select.Root>
          </div>

          <div class="grid grid-cols-2 gap-4">
            <div class="space-y-2">
              <Label for="vid">Vendor ID (HEX)</Label>
              <Input
                id="vid"
                bind:value={device.config.vid}
                maxlength={4}
                placeholder="CAFE"
                disabled={!device.connected ||
                  device.selectedVendor !== "custom"}
                class="font-mono"
              />
            </div>
            <div class="space-y-2">
              <Label for="pid">Product ID (HEX)</Label>
              <Input
                id="pid"
                bind:value={device.config.pid}
                maxlength={4}
                placeholder="4242"
                disabled={!device.connected ||
                  device.selectedVendor !== "custom"}
                class="font-mono"
              />
            </div>
          </div>

          <Separator />

          <div class="space-y-2">
            <Label for="product">Product Name</Label>
            <Input
              id="product"
              bind:value={device.config.productName}
              placeholder="My Key"
              disabled={!device.connected}
            />
          </div>
        </Card.Content>
      </Card.Root>

      <Card.Root>
        <Card.Header>
          <Card.Title class="flex items-center gap-2">
            <Microchip class="h-5 w-5" />
            LED Settings
          </Card.Title>
          <Card.Description>Adjust visual feedback behavior</Card.Description>
        </Card.Header>
        <Card.Content class="space-y-4">
          <div class="space-y-2">
            <Label for="led-gpio">LED GPIO Pin</Label>
            <Input
              id="led-gpio"
              type="number"
              bind:value={device.config.ledGpio}
              min="0"
              max="29"
            />
          </div>

          <div class="space-y-2">
            <Label for="led-driver">LED Driver</Label>
            <Select.Root
              type="single"
              bind:value={device.config.ledDriver}
              disabled={!device.connected}
            >
              <Select.Trigger class="w-full">
                {LED_DRIVERS.find((d) => d.value === device.config.ledDriver)
                  ?.label ?? "Select driver"}
              </Select.Trigger>
              <Select.Content>
                {#each LED_DRIVERS as driver}
                  <Select.Item value={driver.value} label={driver.label}>
                    {driver.label}
                  </Select.Item>
                {/each}
              </Select.Content>
            </Select.Root>
          </div>

          <Separator />

          <div class="space-y-2">
            <Label for="led-brightness">Brightness (0-15)</Label>
            <div class="flex items-center gap-4">
              <Slider
                type="single"
                bind:value={device.config.ledBrightness}
                max={15}
                step={1}
                disabled={!device.connected}
                class="flex-1"
              />
              <span class="text-xs text-muted-foreground min-w-[4ch]"
                >Level {device.config.ledBrightness}</span
              >
            </div>
          </div>

          <div class="flex items-center justify-between space-x-2">
            <div class="space-y-0.5">
              <Label>LED Dimmable</Label>
              <p class="text-sm text-muted-foreground">
                Allow brightness adjustment
              </p>
            </div>
            <Switch bind:checked={device.config.ledDimmable} />
          </div>

          <div class="flex items-center justify-between space-x-2">
            <div class="space-y-0.5">
              <Label>LED Steady Mode</Label>
              <p class="text-sm text-muted-foreground">
                Keep LED on constantly
              </p>
            </div>
            <Switch bind:checked={device.config.ledSteady} />
          </div>
        </Card.Content>
      </Card.Root>

      <Card.Root>
        <Card.Header>
          <Card.Title class="flex items-center gap-2">
            <Settings class="h-5 w-5" />
            Touch & Timing
          </Card.Title>
          <Card.Description>Configure interaction timeouts</Card.Description>
        </Card.Header>
        <Card.Content class="space-y-4">
          <div class="space-y-2">
            <Label for="touch-timeout">Touch Timeout (seconds)</Label>
            <Input
              id="touch-timeout"
              type="number"
              bind:value={device.config.touchTimeout}
              min="1"
              max="255"
            />
          </div>
        </Card.Content>
      </Card.Root>

      <Card.Root>
        <Card.Header>
          <Card.Title class="flex items-center gap-2">
            <Settings class="h-5 w-5" />
            Device Options
          </Card.Title>
          <Card.Description>Toggle advanced features</Card.Description>
        </Card.Header>
        <Card.Content class="space-y-4">
          <div class="flex items-center justify-between space-x-2">
            <div class="space-y-0.5">
              <Label>Power Cycle on Reset</Label>
              <p class="text-sm text-muted-foreground">
                Restart device on reset
              </p>
            </div>
            <Switch bind:checked={device.config.powerCycleOnReset} />
          </div>

          <div class="flex items-center justify-between space-x-2">
            <div class="space-y-0.5">
              <Label>Enable Secp256k1</Label>
              <p class="text-sm text-muted-foreground">
                Does not work on Android!
              </p>
            </div>
            <Switch bind:checked={device.config.enableSecp256k1} />
          </div>
        </Card.Content>
      </Card.Root>
    </div>

    <div class="flex justify-end">
      <Button onclick={() => state.handleSave()} disabled={device.loading}>
        {#if device.loading}
          <RefreshCw class="mr-2 h-4 w-4 animate-spin" />
        {:else}
          <Save class="mr-2 h-4 w-4" />
        {/if}
        Apply Changes
      </Button>
    </div>
  {/if}
</div>
