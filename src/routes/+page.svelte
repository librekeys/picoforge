<script lang="ts">
  import { onMount, tick } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { open } from "@tauri-apps/plugin-shell";

  import {
    Cpu,
    Github,
    Home,
    Info,
    Lock,
    LockOpen,
    Maximize,
    Microchip,
    Minimize,
    Minus,
    Orbit,
    RefreshCw,
    Save,
    ScrollText,
    Settings,
    ShieldCheck,
    Tag,
    Terminal,
    TriangleAlert,
    X,
    Key,
    Shield,
  } from "@lucide/svelte";

  import { Button } from "$lib/components/ui/button";
  import { Input } from "$lib/components/ui/input";
  import { Label } from "$lib/components/ui/label";
  import { Switch } from "$lib/components/ui/switch";
  import { Separator } from "$lib/components/ui/separator";
  import { Badge } from "$lib/components/ui/badge";
  import { ScrollArea } from "$lib/components/ui/scroll-area";
  import { Slider } from "$lib/components/ui/slider/index.js";
  import { Progress } from "$lib/components/ui/progress/index.js";
  import * as Card from "$lib/components/ui/card";
  import * as Alert from "$lib/components/ui/alert";
  import * as Select from "$lib/components/ui/select";
  import * as AlertDialog from "$lib/components/ui/alert-dialog";

  import { logger } from "$lib/utils/log.svelte";
  import { device } from "$lib/device/manager.svelte";
  import { LED_DRIVERS, VENDORS } from "$lib/device/constants.svelte";

  type View = "home" | "config" | "security" | "logs" | "about";
  let currentView: View = $state("home");

  let isMaximized = $state(false);
  let unlistenResize: () => void;

  let dialogOpen = $state(false);
  let dialogTitle = $state("");
  let dialogMessage = $state("");

  let pinDialogOpen = $state(false);
  let currentPin = $state("");
  let newPin = $state("");
  let confirmPin = $state("");
  let pinError = $state("");
  let isSettingPin = $state(false);

  let minPinDialogOpen = $state(false);
  let minPinCurrentPin = $state("");
  let minPinNewPin = $state("");
  let minPinConfirmPin = $state("");
  let minPinLength = $state(4);
  let minPinError = $state("");

  function showStatusDialog(title: string, message: string) {
    dialogTitle = title;
    dialogMessage = message;
    dialogOpen = true;
  }

  $effect(() => {
    logger.logs.length;

    tick().then(() => {
      const viewport = document.querySelector("[data-radix-scroll-area-viewport]");
      if (viewport) {
        viewport.scrollTop = viewport.scrollHeight;
      }
    });
  });

  async function handleSave() {
    const result = await device.save();
    if (result) {
      showStatusDialog(result.success ? "Success" : "Write Failed", result.msg);
    }
  }

  async function openPinDialog() {
    pinError = "";
    currentPin = "";
    newPin = "";
    confirmPin = "";

    if (device.fidoInfo?.options?.clientPin) {
      isSettingPin = false;
    } else {
      isSettingPin = true;
    }

    pinDialogOpen = true;
  }

  async function handlePinChange() {
    pinError = "";

    if (newPin.length < (device.fidoInfo?.minPinLength || 4)) {
      pinError = `PIN must be at least ${device.fidoInfo?.minPinLength || 4} characters`;
      return;
    }

    if (newPin !== confirmPin) {
      pinError = "PINs do not match";
      return;
    }

    const result = await device.changePin(isSettingPin ? null : currentPin, newPin);

    if (result.success) {
      pinDialogOpen = false;
      showStatusDialog("Success", isSettingPin ? "PIN set successfully" : "PIN changed successfully");
    } else {
      pinError = result.msg as string;
    }
  }

  async function openMinPinDialog() {
    minPinError = "";
    minPinCurrentPin = "";
    minPinNewPin = "";
    minPinConfirmPin = "";
    minPinLength = device.fidoInfo?.minPinLength || 4;
    minPinDialogOpen = true;
  }

  async function handleMinPinChange() {
    minPinError = "";

    if (minPinLength < 4 || minPinLength > 63) {
      minPinError = "Minimum PIN length must be between 4 and 63";
      return;
    }

    if (minPinNewPin.length < minPinLength) {
      minPinError = `New PIN must be at least ${minPinLength} characters`;
      return;
    }

    if (minPinNewPin !== minPinConfirmPin) {
      minPinError = "New PINs do not match";
      return;
    }

    const minLengthResult = await device.updateMinPinLength(minPinCurrentPin, minPinLength);

    if (!minLengthResult.success) {
      minPinError = minLengthResult.msg as string;
      return;
    }

    const pinResult = await device.changePin(minPinCurrentPin, minPinNewPin);

    if (pinResult.success) {
      minPinDialogOpen = false;
      showStatusDialog("Success", `Minimum PIN length updated to ${minPinLength} and PIN changed successfully`);
    } else {
      minPinError = pinResult.msg as string;
    }
  }

  const appWindow = getCurrentWindow();

  function minimize() {
    appWindow.minimize();
  }

  async function toggleMaximize() {
    await appWindow.toggleMaximize();
  }

  function closeApp() {
    appWindow.close();
  }

  async function lockDevice() {
    logger.add("Action Blocked: Secure Boot toggle attempted but feature is disabled.", "warning");
    return;
  }

  async function openGithub() {
    await open("https://github.com/librekeys/picoforge");
  }

  async function openWebsite() {
    await open("https://github.com/librekeys/picoforge");
  }

  onMount(() => {
    document.documentElement.classList.add("dark");
    if (logger.logs.length === 0) logger.add("Application started.", "info");

    const setupWindow = async () => {
      isMaximized = await appWindow.isMaximized();
      unlistenResize = await appWindow.onResized(async () => {
        isMaximized = await appWindow.isMaximized();
      });
    };

    setupWindow();
    device.refresh();

    return () => {
      if (unlistenResize) unlistenResize();
    };
  });
</script>

<div class="flex flex-col h-screen w-full bg-background text-foreground overflow-hidden border border-border">
  <header data-tauri-drag-region class="h-10 bg-muted/50 border-b flex items-center justify-between px-2 select-none">
    <div class="text-xs font-medium text-muted-foreground pointer-events-none flex items-center gap-2"></div>

    <div class="flex items-center gap-1">
      <Button variant="ghost" size="icon" class="h-8 w-8 hover:bg-muted" onclick={minimize}>
        <Minus class="h-4 w-4" />
      </Button>

      <Button variant="ghost" size="icon" class="h-8 w-8 hover:bg-muted" onclick={toggleMaximize}>
        {#if isMaximized}
          <Minimize class="h-3.5 w-3.5 rotate-180" />
        {:else}
          <Maximize class="h-3.5 w-3.5" />
        {/if}
      </Button>

      <Button variant="ghost" size="icon" class="h-8 w-8 hover:bg-red-500 hover:text-white transition-colors" onclick={closeApp}>
        <X class="h-4 w-4" />
      </Button>
    </div>
  </header>

  <div class="flex flex-1 overflow-hidden">
    <aside class="w-64 border-r bg-muted/30 flex flex-col">
      <div class="p-6 flex items-center gap-3">
        <img src="/pico-forge.svg" alt="PicoForge Logo" class="h-10 w-10 rounded-lg shadow-sm" />
        <span class="font-bold text-xl tracking-tight">PicoForge</span>
      </div>

      <nav class="flex-1 px-4 space-y-2">
        <Button
          variant={currentView === "home" ? "secondary" : "ghost"}
          class="w-full justify-start gap-3 font-medium"
          onclick={() => (currentView = "home")}
        >
          <Home class="h-4 w-4" />
          Home
        </Button>

        <Button
          variant={currentView === "config" ? "secondary" : "ghost"}
          class="w-full justify-start gap-3 font-medium"
          onclick={() => (currentView = "config")}
        >
          <Settings class="h-4 w-4" />
          Configuration
        </Button>

        <Button
          variant={currentView === "security" ? "secondary" : "ghost"}
          class="w-full justify-start gap-3 font-medium"
          onclick={() => (currentView = "security")}
        >
          <ShieldCheck class="h-4 w-4" />
          Security
        </Button>

        <Button
          variant={currentView === "logs" ? "secondary" : "ghost"}
          class="w-full justify-start gap-3 font-medium"
          onclick={() => (currentView = "logs")}
        >
          <ScrollText class="h-4 w-4" />
          Logs
        </Button>

        <Button
          variant={currentView === "about" ? "secondary" : "ghost"}
          class="w-full justify-start gap-3 font-medium"
          onclick={() => (currentView = "about")}
        >
          <Info class="h-4 w-4" />
          About
        </Button>
      </nav>

      <div class="p-4 border-t bg-background/50">
        <div class="space-y-3">
          <div class="flex items-center justify-between">
            <span class="text-sm font-medium">Device Status</span>
            {#if device.connected}
              <Badge variant="default" class="bg-green-600 hover:bg-green-600">Online</Badge>
            {:else}
              <Badge variant="destructive">Offline</Badge>
            {/if}
          </div>
          <Button variant="outline" class="w-full gap-2" disabled={device.loading} onclick={() => device.refresh()}>
            {#if device.loading}
              <RefreshCw class="h-4 w-4 animate-spin" />
            {:else}
              <RefreshCw class="h-4 w-4" />
            {/if}
            Refresh
          </Button>
        </div>
      </div>
    </aside>

    <main class="flex-1 bg-background overflow-hidden">
      <ScrollArea class="h-full mr-1">
        <div class="container mx-auto py-8 px-8 max-w-6xl">
          <div class="space-y-8">
            {#if currentView === "home"}
              <div class="space-y-6">
                <div>
                  <h1 class="text-3xl font-bold tracking-tight">Device Overview</h1>
                  <p class="text-muted-foreground">Quick view of your device status and specifications.</p>
                </div>

                {#if !device.connected}
                  <Alert.Root>
                    <TriangleAlert class="h-4 w-4" />
                    <Alert.Title>No Device Connected</Alert.Title>
                    <Alert.Description>Please connect your device and click Refresh to begin.</Alert.Description>
                  </Alert.Root>
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
                            <p class="font-mono font-medium">v{device.info.firmwareVersion}</p>
                          </div>
                          <div class="space-y-1">
                            <p class="text-muted-foreground">VID:PID</p>
                            <p class="font-mono font-medium">{device.config.vid}:{device.config.pid}</p>
                          </div>
                          <div class="space-y-1">
                            <p class="text-muted-foreground">Product Name</p>
                            <p class="font-medium truncate">{device.config.productName}</p>
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
                          <Progress value={(device.info.flashUsed / device.info.flashTotal) * 100} class="h-2" />
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
                              <p class="font-medium">{device.fidoInfo.versions[0] || "N/A"}</p>
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
                            <p class="font-mono text-xs break-all">{device.fidoInfo.aaguid}</p>
                          </div>
                        {:else}
                          <p class="text-muted-foreground text-sm">FIDO information not available</p>
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
                          <Badge variant={device.config.ledDimmable ? "default" : "secondary"}>
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
                            <Badge variant={device.security.secureBoot ? "default" : "secondary"}>
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
                          <Badge variant={device.security.confirmed ? "destructive" : "outline"}>
                            {device.security.confirmed ? "Acknowledged" : "Pending"}
                          </Badge>
                        </div>
                      </Card.Content>
                    </Card.Root>
                  </div>
                {/if}
              </div>
            {/if}

            {#if currentView === "config"}
              <div class="space-y-6">
                <div>
                  <h1 class="text-3xl font-bold tracking-tight">Configuration</h1>
                  <p class="text-muted-foreground">Customize device settings and behavior.</p>
                </div>

                {#if !device.connected}
                  <Alert.Root>
                    <TriangleAlert class="h-4 w-4" />
                    <Alert.Title>No Device Connected</Alert.Title>
                    <Alert.Description>Connect your device to access configuration options.</Alert.Description>
                  </Alert.Root>
                {:else}
                  <div class="grid gap-6 lg:grid-cols-2">
                    <Card.Root class="lg:col-span-2">
                      <Card.Header>
                        <Card.Title class="flex items-center gap-2">
                          <Key class="h-5 w-5" />
                          PIN Management
                        </Card.Title>
                        <Card.Description>Configure FIDO2 PIN security</Card.Description>
                      </Card.Header>
                      <Card.Content class="space-y-4">
                        <div class="flex items-center justify-between p-4 border rounded-lg">
                          <div class="space-y-1">
                            <p class="font-medium">Current PIN Status</p>
                            <p class="text-sm text-muted-foreground">
                              {device.fidoInfo?.options?.clientPin ? "PIN is set" : "No PIN configured"}
                            </p>
                          </div>
                          <Button variant="outline" onclick={openPinDialog}>
                            {device.fidoInfo?.options?.clientPin ? "Change PIN" : "Set PIN"}
                          </Button>
                        </div>

                        <div class="flex items-center justify-between p-4 border rounded-lg opacity-60">
                          <div class="space-y-1">
                            <p class="font-medium">Minimum PIN Length</p>
                            <p class="text-sm text-muted-foreground">
                              Current: {device.fidoInfo?.minPinLength || 4} characters
                            </p>
                          </div>
                          <Button variant="outline" disabled={true}>Update Minimum Length</Button>
                        </div>
                      </Card.Content>
                    </Card.Root>

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
                              {VENDORS.find((v) => v.value === device.selectedVendor)?.label ?? "Select a vendor"}
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
                              disabled={!device.connected || device.selectedVendor !== "custom"}
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
                              disabled={!device.connected || device.selectedVendor !== "custom"}
                              class="font-mono"
                            />
                          </div>
                        </div>

                        <Separator />

                        <div class="space-y-2">
                          <Label for="product">Product Name</Label>
                          <Input id="product" bind:value={device.config.productName} placeholder="My Key" disabled={!device.connected} />
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
                          <Input id="led-gpio" type="number" bind:value={device.config.ledGpio} min="0" max="29" />
                        </div>

                        <div class="space-y-2">
                          <Label for="led-driver">LED Driver</Label>
                          <Select.Root type="single" bind:value={device.config.ledDriver} disabled={!device.connected}>
                            <Select.Trigger class="w-full">
                              {LED_DRIVERS.find((d) => d.value === device.config.ledDriver)?.label ?? "Select driver"}
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
                            <span class="text-xs text-muted-foreground min-w-[4ch]">Level {device.config.ledBrightness}</span>
                          </div>
                        </div>

                        <div class="flex items-center justify-between space-x-2">
                          <div class="space-y-0.5">
                            <Label>LED Dimmable</Label>
                            <p class="text-sm text-muted-foreground">Allow brightness adjustment</p>
                          </div>
                          <Switch bind:checked={device.config.ledDimmable} />
                        </div>

                        <div class="flex items-center justify-between space-x-2">
                          <div class="space-y-0.5">
                            <Label>LED Steady Mode</Label>
                            <p class="text-sm text-muted-foreground">Keep LED on constantly</p>
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
                          <Input id="touch-timeout" type="number" bind:value={device.config.touchTimeout} min="1" max="255" />
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
                            <p class="text-sm text-muted-foreground">Restart device on reset</p>
                          </div>
                          <Switch bind:checked={device.config.powerCycleOnReset} />
                        </div>

                        <div class="flex items-center justify-between space-x-2">
                          <div class="space-y-0.5">
                            <Label>Enable Secp256k1</Label>
                            <p class="text-sm text-muted-foreground">Does not work on Android!</p>
                          </div>
                          <Switch bind:checked={device.config.enableSecp256k1} />
                        </div>
                      </Card.Content>
                    </Card.Root>
                  </div>

                  <div class="flex justify-end">
                    <Button onclick={handleSave} disabled={device.loading}>
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
            {/if}

            {#if currentView === "security"}
              <div class="space-y-6">
                <div>
                  <h1 class="text-3xl font-bold tracking-tight text-destructive">Secure Boot</h1>
                  <p class="text-muted-foreground">Permanently lock this device to the current firmware vendor.</p>
                </div>

                <Alert.Root variant="destructive">
                  <TriangleAlert class="h-4 w-4" />
                  <Alert.Title>Feature Unstable</Alert.Title>
                  <Alert.Description>This feature is currently under work and disabled for safety.</Alert.Description>
                </Alert.Root>

                <Card.Root class="border-destructive/30 opacity-60">
                  <Card.Header>
                    <Card.Title>Lock Settings</Card.Title>
                  </Card.Header>
                  <Card.Content class="space-y-6 pointer-events-none">
                    <div class="flex items-center justify-between space-x-2">
                      <div class="flex flex-col space-y-1">
                        <Label for="secure-boot">Enable Secure Boot</Label>
                        <p class="font-normal text-xs text-muted-foreground">Verifies firmware signature on startup</p>
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
                        <p class="font-normal text-xs text-muted-foreground">Prevents reading key material via debug ports</p>
                      </div>
                      <Switch id="secure-lock" bind:checked={device.security.secureLock} disabled={true} />
                    </div>

                    <Separator />

                    <div class="flex items-center space-x-2 bg-destructive/10 p-4 rounded-md border border-destructive/20">
                      <Switch id="confirm" bind:checked={device.security.confirmed} disabled={true} />
                      <Label for="confirm" class="text-destructive font-medium">I understand the risks of bricking my device.</Label>
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
            {/if}

            {#if currentView === "logs"}
              <div class="space-y-6 h-full flex flex-col">
                <div class="flex items-center justify-between">
                  <div>
                    <h1 class="text-3xl font-bold tracking-tight">System Logs</h1>
                    <p class="text-muted-foreground">Real-time device communication and application events.</p>
                  </div>
                  <Button variant="outline" size="sm" onclick={() => (logger.logs = [])}>Clear Logs</Button>
                </div>

                <Card.Root class="flex-1 flex flex-col min-h-[500px] bg-black border-zinc-800">
                  <Card.Content class="p-0 flex-1 flex flex-col">
                    {#if logger.logs.length === 0}
                      <div class="flex-1 flex flex-col items-center justify-center text-zinc-500">
                        <Terminal class="h-12 w-12 mb-4 opacity-50" />
                        <p>No events recorded yet.</p>
                      </div>
                    {:else}
                      <ScrollArea class="h-[500px] p-4 font-mono text-sm" orientation="vertical">
                        <div class="space-y-1">
                          {#each logger.logs as log}
                            <div class="flex gap-3">
                              <span class="text-zinc-500 shrink-0">[{log.timestamp}]</span>
                              <span
                                class={`break-all ${
                                  log.type === "error"
                                    ? "text-red-400"
                                    : log.type === "success"
                                      ? "text-green-400"
                                      : log.type === "warning"
                                        ? "text-yellow-400"
                                        : "text-zinc-300"
                                }`}
                              >
                                {#if log.type === "success"}➜{/if}
                                {#if log.type === "error"}✖{/if}
                                {log.message}
                              </span>
                            </div>
                          {/each}
                        </div>
                      </ScrollArea>
                    {/if}
                  </Card.Content>
                </Card.Root>
              </div>
            {/if}

            {#if currentView === "about"}
              <div class="space-y-6">
                <div>
                  <h1 class="text-3xl font-bold tracking-tight">About</h1>
                </div>

                <Card.Root>
                  <Card.Content class="pt-6">
                    <div class="flex flex-col items-center justify-center space-y-4 text-center py-8">
                      <div class="bg-primary text-primary p-6 rounded-2xl">
                        <img src="/build-configure-symbolic.svg" alt="PicoForge Logo" class="h-12 w-12" />
                      </div>

                      <h2 class="text-2xl font-bold">PicoForge</h2>
                      <Badge variant="secondary" class="px-4 py-1">v0.1.3-Alpha</Badge>
                      <p class="text-muted-foreground max-w-md">
                        An open source commissioning tool for Pico FIDO security keys. Developed with Rust, Tauri, and Svelte.
                      </p>

                      <div class="text-sm text-muted-foreground space-y-1 pt-4 border-t w-full max-w-xs">
                        <div class="flex justify-between">
                          <span>Code By:</span> <span class="font-medium text-foreground">Suyog Tandel</span>
                        </div>
                        <div class="flex justify-between items-center pt-2 mt-2">
                          <span class="flex items-center gap-1">Copyright:</span>
                          <span class="font-medium text-foreground">© 2026 Suyog Tandel</span>
                        </div>
                      </div>

                      <div class="flex gap-4 pt-6">
                        <Button variant="outline" size="sm" class="gap-2" onclick={openGithub}>
                          <Github class="h-4 w-4" />
                          GitHub
                        </Button>
                        <!-- <Button variant="outline" size="sm" class="gap-2" onclick={openWebsite}>
                          <Home class="h-4 w-4" />
                          Website
                        </Button> -->
                      </div>
                    </div>
                  </Card.Content>
                </Card.Root>
              </div>
            {/if}
          </div>
        </div>
      </ScrollArea>
    </main>
  </div>

  <AlertDialog.Root bind:open={dialogOpen}>
    <AlertDialog.Content>
      <AlertDialog.Header>
        <AlertDialog.Title>{dialogTitle}</AlertDialog.Title>
        <AlertDialog.Description>
          {dialogMessage}
        </AlertDialog.Description>
      </AlertDialog.Header>
      <AlertDialog.Footer>
        <AlertDialog.Action onclick={() => (dialogOpen = false)}>Okay</AlertDialog.Action>
      </AlertDialog.Footer>
    </AlertDialog.Content>
  </AlertDialog.Root>

  <AlertDialog.Root bind:open={pinDialogOpen}>
    <AlertDialog.Content>
      <AlertDialog.Header>
        <AlertDialog.Title>{isSettingPin ? "Set PIN" : "Change PIN"}</AlertDialog.Title>
        <AlertDialog.Description>
          {isSettingPin
            ? "Set a PIN for your FIDO2 device. Minimum length is " + (device.fidoInfo?.minPinLength || 4) + " characters."
            : "Enter your current PIN and choose a new one."}
        </AlertDialog.Description>
      </AlertDialog.Header>
      <div class="space-y-4 py-4">
        {#if !isSettingPin}
          <div class="space-y-2">
            <Label for="current-pin">Current PIN</Label>
            <Input id="current-pin" type="password" bind:value={currentPin} placeholder="Enter current PIN" />
          </div>
        {/if}
        <div class="space-y-2">
          <Label for="new-pin">New PIN</Label>
          <Input id="new-pin" type="password" bind:value={newPin} placeholder="Enter new PIN" />
        </div>
        <div class="space-y-2">
          <Label for="confirm-pin">Confirm New PIN</Label>
          <Input id="confirm-pin" type="password" bind:value={confirmPin} placeholder="Confirm new PIN" />
        </div>
        {#if pinError}
          <p class="text-sm text-destructive">{pinError}</p>
        {/if}
      </div>
      <AlertDialog.Footer>
        <AlertDialog.Cancel onclick={() => (pinDialogOpen = false)}>Cancel</AlertDialog.Cancel>
        <AlertDialog.Action onclick={handlePinChange}>Confirm</AlertDialog.Action>
      </AlertDialog.Footer>
    </AlertDialog.Content>
  </AlertDialog.Root>

  <AlertDialog.Root bind:open={minPinDialogOpen}>
    <AlertDialog.Content>
      <AlertDialog.Header>
        <AlertDialog.Title>Update Minimum PIN Length</AlertDialog.Title>
        <AlertDialog.Description>
          Set the minimum allowed PIN length (4-63 characters) and enter a new PIN that meets this requirement.
        </AlertDialog.Description>
      </AlertDialog.Header>
      <div class="space-y-4 py-4">
        <div class="space-y-2">
          <Label for="min-pin-length">Minimum PIN Length ({minPinLength})</Label>
          <Slider type="single" bind:value={minPinLength} min={4} max={63} step={1} />
        </div>
        <div class="space-y-2">
          <Label for="min-pin-current">Current PIN</Label>
          <Input id="min-pin-current" type="password" bind:value={minPinCurrentPin} placeholder="Enter current PIN" />
        </div>
        <div class="space-y-2">
          <Label for="min-pin-new">New PIN (min {minPinLength} chars)</Label>
          <Input id="min-pin-new" type="password" bind:value={minPinNewPin} placeholder="Enter new PIN" />
        </div>
        <div class="space-y-2">
          <Label for="min-pin-confirm">Confirm New PIN</Label>
          <Input id="min-pin-confirm" type="password" bind:value={minPinConfirmPin} placeholder="Confirm new PIN" />
        </div>
        {#if minPinError}
          <p class="text-sm text-destructive">{minPinError}</p>
        {/if}
      </div>
      <AlertDialog.Footer>
        <AlertDialog.Cancel onclick={() => (minPinDialogOpen = false)}>Cancel</AlertDialog.Cancel>
        <AlertDialog.Action onclick={handleMinPinChange}>Update</AlertDialog.Action>
      </AlertDialog.Footer>
    </AlertDialog.Content>
  </AlertDialog.Root>
</div>
