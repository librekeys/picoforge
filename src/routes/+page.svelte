<script lang="ts">
  // Tauri and svelte imports
  import { onMount, tick } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { open } from "@tauri-apps/plugin-shell";

  // Lucide Icon Imports
  import {
    Copyright,
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
  } from "@lucide/svelte";

  // Shadcn Components
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

  // Lib Imports:
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
  <header
    data-tauri-drag-region
    class="h-10 bg-muted/50 border-b flex items-center justify-between px-2 select-none"
  >
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

      <Button
        variant="ghost"
        size="icon"
        class="h-8 w-8 hover:bg-red-500 hover:text-white transition-colors"
        onclick={closeApp}
      >
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
          onclick={() => currentView = "home"}
        >
          <Home class="h-4 w-4" />
          Home
        </Button>

        <Button
          variant={currentView === "config" ? "secondary" : "ghost"}
          class="w-full justify-start gap-3 font-medium"
          onclick={() => currentView = "config"}
        >
          <Settings class="h-4 w-4" />
          Configuration
        </Button>

        <Button
          variant={currentView === "security" ? "secondary" : "ghost"}
          class="w-full justify-start gap-3 font-medium"
          onclick={() => currentView = "security"}
        >
          <ShieldCheck class="h-4 w-4" />
          Security
        </Button>

        <Button
          variant={currentView === "logs" ? "secondary" : "ghost"}
          class="w-full justify-start gap-3 font-medium"
          onclick={() => currentView = "logs"}
        >
          <ScrollText class="h-4 w-4" />
          Logs
        </Button>

        <Button
          variant={currentView === "about" ? "secondary" : "ghost"}
          class="w-full justify-start gap-3 font-medium"
          onclick={() => currentView = "about"}
        >
          <Info class="h-4 w-4" />
          About
        </Button>
      </nav>

      <div class="p-4 border-t bg-background/50">
        <div class="flex items-center gap-3 px-2">
          <div
            class={`h-2.5 w-2.5 rounded-full shadow-sm ${device.connected ? "bg-green-500" : "bg-red-500"}`}
          >
          </div>
          <span class="text-sm font-medium text-muted-foreground">
            {device.connected ? "Connected" : "Disconnected"}
          </span>
        </div>
      </div>
    </aside>

    <main class="flex-1 overflow-hidden bg-background">
      <ScrollArea class="h-full">
        <div class="p-8">
          <div class="max-w-4xl mx-auto space-y-8">
            {#if currentView === "home"}
              <div class="space-y-6">
                <div class="flex items-center justify-between">
                  <div>
                    <h1 class="text-3xl font-bold tracking-tight">Device Status</h1>
                    <p class="text-muted-foreground mt-1">Overview of connected hardware.</p>
                  </div>
                  <Button variant="outline" size="icon" onclick={() => device.refresh()} disabled={device.loading}>
                    <RefreshCw
                      class={`h-4 w-4 ${device.loading ? "animate-spin" : ""}`}
                    />
                  </Button>
                </div>

                <div class="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
                  <Card.Root>
                    <Card.Header class="flex flex-row items-center justify-between space-y-0 pb-2">
                      <Card.Title class="text-sm font-medium">Product Name</Card.Title>
                      <Tag class="h-4 w-4 text-muted-foreground" />
                    </Card.Header>
                    <Card.Content>
                      <div class="text-lg font-bold truncate">
                        {device.connected ? device.config.productName : "---"}
                      </div>
                      <p class="text-xs text-muted-foreground mt-1">
                        VID: {device.connected ? device.config.vid : "--"} | PID: {
                          device.connected ? device.config.pid : "--"
                        }
                      </p>
                    </Card.Content>
                  </Card.Root>

                  <Card.Root>
                    <Card.Header class="flex flex-row items-center justify-between space-y-0 pb-2">
                      <Card.Title class="text-sm font-medium">Serial Number</Card.Title>
                      <Cpu class="h-4 w-4 text-muted-foreground" />
                    </Card.Header>
                    <Card.Content>
                      <div
                        class="text-xl font-bold font-mono tracking-wide break-all leading-tight"
                      >
                        {device.connected ? device.info.serial : "---"}
                      </div>
                    </Card.Content>
                  </Card.Root>

                  <Card.Root>
                    <Card.Header class="flex flex-row items-center justify-between space-y-0 pb-2">
                      <Card.Title class="text-sm font-medium">Flash Usage</Card.Title>
                      <Microchip class="h-4 w-4 text-muted-foreground" />
                    </Card.Header>
                    <Card.Content>
                      <div class="text-2xl font-bold">
                        {
                          device.connected
                            ? `${device.info.flashUsed} / ${device.info.flashTotal} KB`
                            : "---"
                        }
                      </div>
                      <Progress
                        value={device.connected &&
                            device.info.flashTotal > 0
                          ? (device.info.flashUsed /
                            device.info.flashTotal) *
                            100
                          : 0}
                        max={100}
                        class="mt-3"
                      />
                    </Card.Content>
                  </Card.Root>

                  <Card.Root>
                    <Card.Header class="flex flex-row items-center justify-between space-y-0 pb-2">
                      <Card.Title class="text-sm font-medium">Firmware</Card.Title>
                      <Info class="h-4 w-4 text-muted-foreground" />
                    </Card.Header>
                    <Card.Content>
                      <div class="text-2xl font-bold">
                        {device.connected ? `v${device.info.firmwareVersion}` : "---"}
                      </div>
                    </Card.Content>
                  </Card.Root>

                  <Card.Root>
                    <Card.Header class="flex flex-row items-center justify-between space-y-0 pb-2">
                      <Card.Title class="text-sm font-medium">Security Status</Card.Title>
                      {#if device.security.secureBoot}
                        <Lock class="h-4 w-4 text-green-500" />
                      {:else}
                        <LockOpen class="h-4 w-4 text-amber-500" />
                      {/if}
                    </Card.Header>
                    <Card.Content>
                      <div class="text-xl font-bold">
                        {
                          device.connected
                            ? (device.security.secureBoot ? "Secure Boot" : "Development")
                            : "---"
                        }
                      </div>
                      <p class="text-xs text-muted-foreground mt-1">
                        {
                          device.connected
                            ? (device.security.secureLock ? "Read-out Locked" : "Debug Enabled")
                            : ""
                        }
                      </p>
                    </Card.Content>
                  </Card.Root>
                </div>

                {#if !device.connected}
                  <Alert.Root class="bg-amber-500/10 border-amber-500/50 text-amber-500">
                    <TriangleAlert class="h-4 w-4" />
                    <Alert.Title>No Device Detected</Alert.Title>
                    <Alert.Description>
                      Please plug in your Pico FIDO key. If you are on Linux, ensure you have the correct udev rules
                      installed.
                    </Alert.Description>
                  </Alert.Root>
                {/if}
              </div>
            {/if}

            {#if currentView === "config"}
              <div class="space-y-6">
                <div>
                  <h1 class="text-3xl font-bold tracking-tight">Configuration</h1>
                  <p class="text-muted-foreground">
                    Customize your key's identity and hardware behavior.
                  </p>
                </div>

                <Card.Root>
                  <Card.Header>
                    <Card.Title>Identity</Card.Title>
                    <Card.Description>USB Identification settings</Card.Description>
                  </Card.Header>
                  <Card.Content class="space-y-4">
                    <div class="space-y-2">
                      <Label>Vendor Preset</Label>
                      <Select.Root
                        type="single"
                        bind:value={device.selectedVendor}
                        onValueChange={device.setVendor}
                        disabled={!device.connected}
                      >
                        <Select.Trigger class="w-full">
                          {
                            VENDORS.find(
                              (v) =>
                                v.value ===
                                  device.selectedVendor,
                            )?.label ??
                              "Select a vendor"
                          }
                        </Select.Trigger>
                        <Select.Content>
                          {#each VENDORS as vendor}
                            <Select.Item value={vendor.value} label={vendor.label}>
                              {
                                vendor
                                  .label
                              }
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
                            device.selectedVendor !==
                              "custom"}
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
                            device.selectedVendor !==
                              "custom"}
                          class="font-mono"
                        />
                      </div>
                    </div>
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
                    <Card.Title>Curves</Card.Title>
                    <Card.Description>Cryptographic Curve Settings</Card.Description>
                  </Card.Header>
                  <Card.Content class="space-y-4">
                    <div class="flex items-center justify-between space-x-2">
                      <Label for="secp256k1" class="flex flex-col space-y-1">
                        <span>Enable secp256k1</span>
                      </Label>
                      <Switch
                        id="secp256k1"
                        bind:checked={device.config.enableSecp256k1}
                        disabled={!device.connected}
                      />
                    </div>
                  </Card.Content>
                </Card.Root>

                <Card.Root>
                  <Card.Header>
                    <Card.Title>Hardware & Behavior</Card.Title>
                    <Card.Description>LED settings and timeouts</Card.Description>
                  </Card.Header>
                  <Card.Content class="space-y-4">
                    <div class="grid grid-cols-2 gap-4">
                      <div class="space-y-2">
                        <Label for="gpio">LED GPIO Pin</Label>
                        <Input
                          id="gpio"
                          type="number"
                          bind:value={device.config.ledGpio}
                          disabled={!device.connected}
                        />
                      </div>
                      <div class="space-y-2">
                        <Label for="timeout">Touch Timeout (Seconds)</Label>
                        <Input
                          id="timeout"
                          type="number"
                          bind:value={device.config.touchTimeout}
                          disabled={!device.connected}
                        />
                      </div>
                    </div>

                    <div class="space-y-2 pt-2">
                      <Label>LED Driver</Label>
                      <Select.Root
                        type="single"
                        bind:value={device.config.ledDriver}
                        disabled={!device.connected}
                      >
                        <Select.Trigger class="w-full">
                          {
                            LED_DRIVERS.find(
                              (d) =>
                                d.value ===
                                  device.config
                                    .ledDriver,
                            )?.label ??
                              "Select Driver"
                          }
                        </Select.Trigger>
                        <Select.Content>
                          {#each LED_DRIVERS as driver}
                            <Select.Item value={driver.value} label={driver.label}>
                              {
                                driver
                                  .label
                              }
                            </Select.Item>
                          {/each}
                        </Select.Content>
                      </Select.Root>
                    </div>

                    <Separator class="my-2" />

                    <div class="flex items-center justify-between space-x-2 mt-4">
                      <Label for="led-dimmable" class="flex flex-col space-y-1">
                        <span>LED Dimmable</span>
                      </Label>
                      <Switch
                        id="led-dimmable"
                        bind:checked={device.config.ledDimmable}
                        disabled={!device.connected}
                      />
                    </div>

                    <div class="flex items-center justify-between space-x-2">
                      <Label for="led-steady" class="flex flex-col space-y-1">
                        <span>LED Steady (No Blink)</span>
                      </Label>
                      <Switch
                        id="led-steady"
                        bind:checked={device.config.ledSteady}
                        disabled={!device.connected}
                      />
                    </div>

                    <div class="flex items-center justify-between space-x-2">
                      <Label for="power-cycle" class="flex flex-col space-y-1">
                        <span>Power Cycle on Reset</span>
                      </Label>
                      <Switch
                        id="power-cycle"
                        bind:checked={device.config.powerCycleOnReset}
                        disabled={!device.connected}
                      />
                    </div>

                    <div class="space-y-3 pt-4">
                      <div class="flex justify-between">
                        <Label>LED Brightness (0-15)</Label>
                        <span class="text-xs text-muted-foreground">Level {
                            device.config
                              .ledBrightness
                          }</span>
                      </div>
                      <Slider
                        type="single"
                        bind:value={device.config.ledBrightness}
                        max={15}
                        step={1}
                        disabled={!device.connected}
                      />
                    </div>
                  </Card.Content>
                  <Card.Footer class="border-t bg-muted/20 px-6 py-4 flex justify-end">
                    <Button onclick={handleSave} disabled={!device.connected || device.loading}>
                      {#if device.loading}
                        <RefreshCw class="mr-2 h-4 w-4 animate-spin" />
                      {/if}
                      <Save class="mr-2 h-4 w-4" />
                      Apply Changes
                    </Button>
                  </Card.Footer>
                </Card.Root>
              </div>
            {/if}

            {#if currentView === "security"}
              <div class="space-y-6">
                <div>
                  <h1 class="text-3xl font-bold tracking-tight text-destructive">Secure Boot</h1>
                  <p class="text-muted-foreground">
                    Permanently lock this device to the current firmware vendor.
                  </p>
                </div>

                <Alert.Root variant="destructive">
                  <TriangleAlert class="h-4 w-4" />
                  <Alert.Title>Feature Unstable</Alert.Title>
                  <Alert.Description>
                    This feature is currently under work and disabled for safety.
                  </Alert.Description>
                </Alert.Root>

                <Card.Root class="border-destructive/30 opacity-60">
                  <Card.Header>
                    <Card.Title>Lock Settings</Card.Title>
                  </Card.Header>
                  <Card.Content class="space-y-6 pointer-events-none">
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
                      <Switch id="confirm" bind:checked={device.security.confirmed} disabled={true} />
                      <Label for="confirm" class="text-destructive font-medium"
                      >I understand the risks of bricking my device.</Label>
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
                    <p class="text-muted-foreground">
                      Real-time device communication and application events.
                    </p>
                  </div>
                  <Button variant="outline" size="sm" onclick={() => logger.logs = []}>
                    Clear Logs
                  </Button>
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
                                  log.type ===
                                      "error"
                                    ? "text-red-400"
                                    : log
                                        .type ===
                                        "success"
                                    ? "text-green-400"
                                    : log
                                        .type ===
                                        "warning"
                                    ? "text-yellow-400"
                                    : "text-zinc-300"
                                }`}
                              >
                                {#if                             log.type ===
                              "success"}➜
                                {/if}
                                {#if                             log.type ===
                              "error"}✖
                                {/if}
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
                    <div
                      class="flex flex-col items-center justify-center space-y-4 text-center py-8"
                    >
                      <div class="bg-primary text-primary p-6 rounded-2xl">
                        <img
                          src="/build-configure-symbolic.svg"
                          alt="PicoForge Logo"
                          class="h-12 w-12"
                        />
                      </div>

                      <h2 class="text-2xl font-bold">PicoForge</h2>
                      <Badge variant="secondary" class="px-4 py-1">v0.1.0-Alpha</Badge>
                      <p class="text-muted-foreground max-w-md">
                        An open source commissioning tool for Pico FIDO security keys. Developed with Rust, Tauri, and
                        Svelte.
                      </p>

                      <div
                        class="text-sm text-muted-foreground space-y-1 pt-4 border-t w-full max-w-xs"
                      >
                        <div class="flex justify-between">
                          <span>Code By:</span> <span class="font-medium text-foreground">Suyog Tandel</span>
                        </div>
                        <div class="flex justify-between">
                          <span>Artwork:</span> <span class="font-medium text-foreground">Suyog Tandel</span>
                        </div>
                        <div
                          class="flex justify-between items-center pt-2 mt-2 border-t border-dashed"
                        >
                          <span class="flex items-center gap-1"><Copyright class="h-3 w-3" />
                            Copyright:</span>
                          <span class="font-medium text-foreground">2026 Suyog Tandel</span>
                        </div>
                      </div>

                      <div class="flex gap-4 pt-6">
                        <Button variant="outline" size="sm" class="gap-2" onclick={openGithub}>
                          <Github class="h-4 w-4" />
                          GitHub
                        </Button>
                        <Button variant="outline" size="sm" class="gap-2 onclick={openWebsite}">
                          <Home class="h-4 w-4" />
                          Website
                        </Button>
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
        <AlertDialog.Action onclick={() => dialogOpen = false}>Okay</AlertDialog.Action>
      </AlertDialog.Footer>
    </AlertDialog.Content>
  </AlertDialog.Root>
</div>
