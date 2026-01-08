<script lang="ts">
  import { onMount, tick } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { invoke } from "@tauri-apps/api/core";
  import { 
    Home, 
    Settings, 
    ShieldCheck, 
    Info, 
    RefreshCw, 
    Save, 
    Lock, 
    LockOpen,
    Cpu,
    Microchip,
    TriangleAlert,
    Minus,
    Maximize, 
    Minimize,
    X,
    Orbit,
    ScrollText,
    Terminal,
    Github,
    Tag,
    Copyright
  } from "@lucide/svelte";
  import { Button } from "$lib/components/ui/button";
  import { Input } from "$lib/components/ui/input";
  import { Label } from "$lib/components/ui/label";
  import { Switch } from "$lib/components/ui/switch";
  import { Separator } from "$lib/components/ui/separator";
  import { Badge } from "$lib/components/ui/badge";
  import { ScrollArea } from "$lib/components/ui/scroll-area";
  import * as Card from "$lib/components/ui/card";
  import * as Alert from "$lib/components/ui/alert";
  import * as Select from "$lib/components/ui/select";
  import * as AlertDialog from "$lib/components/ui/alert-dialog";

  type View = "home" | "config" | "security" | "logs" | "about";
  let currentView: View = "home";
  
  let loading = false;
  let deviceConnected = false;
  let isMaximized = false;
  let unlistenResize: () => void;

  let dialogOpen = false;
  let dialogTitle = "";
  let dialogMessage = "";
  
  function showStatusDialog(title: string, message: string) {
    dialogTitle = title;
    dialogMessage = message;
    dialogOpen = true;
  }

  type LogEntry = {
    timestamp: string;
    message: string;
    type: "info" | "success" | "error" | "warning";
  };
  let logs: LogEntry[] = [];
  let logScrollViewport: HTMLElement;

  function addLog(message: string, type: LogEntry['type'] = "info") {
    const now = new Date();
    const timeString = now.toLocaleTimeString('en-US', { hour12: false, hour: "2-digit", minute: "2-digit", second: "2-digit" });
    logs = [...logs, { timestamp: timeString, message, type }];
    
    tick().then(() => {
        const viewport = document.querySelector('[data-radix-scroll-area-viewport]');
        if (viewport) {
            viewport.scrollTop = viewport.scrollHeight;
        }
    });
  }

  const vendors = [
    { value: "custom", label: "Custom (Manual Entry)", vid: "", pid: "" },
    { value: "generic", label: "Generic (FEFF:FCFD)", vid: "FEFF", pid: "FCFD" },
    { value: "pico", label: "Pico (2E8A:0003)", vid: "2E8A", pid: "0003" },
    { value: "solokeys", label: "SoloKeys (0483:A2CA)", vid: "0483", pid: "A2CA" },
    { value: "nitrohsm", label: "NitroHSM (20A0:4230)", vid: "20A0", pid: "4230" },
    { value: "nitrofido2", label: "NitroFIDO2 (20A0:42D4)", vid: "20A0", pid: "42D4" },
    { value: "nitrostart", label: "NitroStart (20A0:4211)", vid: "20A0", pid: "4211" },
    { value: "nitropro", label: "NitroPro (20A0:4108)", vid: "20A0", pid: "4108" },
    { value: "nitro3", label: "Nitrokey 3 (20A0:42B2)", vid: "20A0", pid: "42B2" },
    { value: "yubikey5", label: "YubiKey 5 (1050:0407)", vid: "1050", pid: "0407" },
    { value: "yubikeyneo", label: "YubiKey Neo (1050:0116)", vid: "1050", pid: "0116" },
    { value: "yubihsm", label: "YubiHSM 2 (1050:0030)", vid: "1050", pid: "0030" },
    { value: "gnuk", label: "Gnuk Token (234B:0000)", vid: "234B", pid: "0000" },
    { value: "gnupg", label: "GnuPG (234B:0000)", vid: "234B", pid: "0000" }
  ];

  const ledDrivers = [
    { value: "1", label: "Pico (Standard GPIO)" },
    { value: "2", label: "Pimoroni (RGB)" },
    { value: "3", label: "WS2812 (Neopixel)" },
    { value: "5", label: "ESP32 Neopixel" }
  ];

  let selectedVendor = "custom";
  function handleVendorChange(value: string) {
    const v = vendors.find(x => x.value === value);
    if (v && value !== "custom") {
        config.vid = v.vid;
        config.pid = v.pid;
        addLog(`Selected vendor preset: ${v.label}`, "info");
    }
  }

  let config = {
    vid: "CAFE",
    pid: "4242",
    productName: "Pico FIDO Key",
    ledGpio: 2,
    ledBrightness: 15,
    touchTimeout: 30,
    ledDimmable: false,
    powerCycleOnReset: false,
    ledSteady: false,
    enableSecp256k1: false,
    ledDriver: "1"
  };
  let originalConfig: any = null;

  let security = {
    secureBoot: false,
    secureLock: false,
    confirmed: false
  };

  let deviceInfo = {
    serial: "---",
    flashUsed: 0,
    flashTotal: 0,
    firmwareVersion: "---"
  };

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

  async function checkConnection() {
    loading = true;
    try {
      addLog("Attempting to connect to device...", "info");
      
      const status: any = await invoke("read_device_details");

      deviceInfo = {
        serial: status.info.serial,
        flashUsed: status.info.flash_used,
        flashTotal: status.info.flash_total,
        firmwareVersion: status.info.firmware_version
      };

      config = {
        vid: status.config.vid,
        pid: status.config.pid,
        productName: status.config.product_name,
        ledGpio: status.config.led_gpio,
        ledBrightness: status.config.led_brightness,
        touchTimeout: status.config.touch_timeout,
        ledDimmable: status.config.led_dimmable,
        powerCycleOnReset: status.config.power_cycle_on_reset,
        ledSteady: status.config.led_steady,
        enableSecp256k1: status.config.enable_secp256k1,
        ledDriver: status.config.led_driver ? String(status.config.led_driver) : "1"
      };
      
      originalConfig = JSON.parse(JSON.stringify(config));

      const match = vendors.find(v => v.vid === config.vid && v.pid === config.pid);
      selectedVendor = match ? match.value : "custom";

      security = {
        secureBoot: status.secure_boot,
        secureLock: status.secure_lock,
        confirmed: false 
      };

      if (!deviceConnected) {
          addLog(`Device Connected! Serial: ${deviceInfo.serial}, FW: v${deviceInfo.firmwareVersion}`, "success");
      }
      deviceConnected = true;

    } catch (err) {
      console.error("Connection failed:", err);
      if (deviceConnected) {
          addLog(`Connection lost: ${err}`, "error");
      }
      deviceConnected = false;
    } finally {
      loading = false;
    }
  }

  async function saveConfiguration() {
    if (!deviceConnected || !originalConfig) return;
    loading = true;
    addLog("Analyzing configuration changes...", "info");

    try {
      const rustConfig: any = {};
      
      if (config.vid !== originalConfig.vid || config.pid !== originalConfig.pid) {
          rustConfig.vid = config.vid;
          rustConfig.pid = config.pid;
          addLog(`Queuing change: VID/PID -> ${config.vid}:${config.pid}`, "info");
      }

      if (config.productName !== originalConfig.productName) {
          rustConfig.product_name = config.productName;
          addLog(`Queuing change: Product Name -> ${config.productName}`, "info");
      }

      if (Number(config.ledGpio) !== Number(originalConfig.ledGpio)) {
          rustConfig.led_gpio = Number(config.ledGpio);
          addLog(`Queuing change: LED GPIO -> ${config.ledGpio}`, "info");
      }

      if (Number(config.ledBrightness) !== Number(originalConfig.ledBrightness)) {
          rustConfig.led_brightness = Number(config.ledBrightness);
          addLog(`Queuing change: Brightness -> ${config.ledBrightness}`, "info");
      }

      if (Number(config.touchTimeout) !== Number(originalConfig.touchTimeout)) {
          rustConfig.touch_timeout = Number(config.touchTimeout);
          addLog(`Queuing change: Timeout -> ${config.touchTimeout}`, "info");
      }

      const optionsChanged = 
          config.ledDimmable !== originalConfig.ledDimmable ||
          config.powerCycleOnReset !== originalConfig.powerCycleOnReset ||
          config.ledSteady !== originalConfig.ledSteady;

      if (optionsChanged) {
          rustConfig.led_dimmable = config.ledDimmable;
          rustConfig.power_cycle_on_reset = config.powerCycleOnReset;
          rustConfig.led_steady = config.ledSteady;
          addLog("Queuing change: Device Options (Bitmask updated)", "info");
      }

      if (config.enableSecp256k1 !== originalConfig.enableSecp256k1) {
          rustConfig.enable_secp256k1 = config.enableSecp256k1;
          addLog(`Queuing change: Secp256k1 -> ${config.enableSecp256k1}`, "info");
      }

      if (Number(config.ledDriver) !== Number(originalConfig.ledDriver)) {
          rustConfig.led_driver = Number(config.ledDriver);
          addLog(`Queuing change: LED Driver -> ${config.ledDriver}`, "info");
      }
      
      if (Object.keys(rustConfig).length === 0) {
        addLog("No changes detected.", "warning");
        showStatusDialog("No Changes", "There are no configuration changes to save.");
      } else {
        addLog("Sending configuration to device...", "info");
        const response = await invoke("write_config", { config: rustConfig });
        addLog(`Device Response: ${response}`, "success");
        
        showStatusDialog("Success", "Configuration Applied Successfully!");
        
        await checkConnection();
      }

    } catch (err: any) {
      addLog(`Write Failed: ${err}`, "error");
      showStatusDialog("Write Failed", "Error saving config: " + err);
    } finally {
      loading = false;
    }
  }

  async function lockDevice() {
    addLog("Action Blocked: Secure Boot toggle attempted but feature is disabled.", "warning");
    return;
  }

  onMount(() => {
    document.documentElement.classList.add("dark");
    addLog("Application started.", "info");

    const setupWindow = async () => {
      isMaximized = await appWindow.isMaximized();
      unlistenResize = await appWindow.onResized(async () => {
        isMaximized = await appWindow.isMaximized();
      });
    };

    setupWindow();
    checkConnection();

    return () => {
        if (unlistenResize) unlistenResize();
    };
  });
</script>

<div class="flex flex-col h-screen w-full bg-background text-foreground overflow-hidden border border-border">
  
  <header 
    data-tauri-drag-region 
    class="h-10 bg-muted/50 border-b flex items-center justify-between px-4 select-none"
  >
    <div class="text-xs font-medium text-muted-foreground pointer-events-none flex items-center gap-2">
    </div>
    
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
          variant={currentView === 'home' ? "secondary" : "ghost"} 
          class="w-full justify-start gap-3 font-medium"
          onclick={() => currentView = 'home'}
        >
          <Home class="h-4 w-4" />
          Home
        </Button>

        <Button 
          variant={currentView === 'config' ? "secondary" : "ghost"} 
          class="w-full justify-start gap-3 font-medium"
          onclick={() => currentView = 'config'}
        >
          <Settings class="h-4 w-4" />
          Configuration
        </Button>

        <Button 
          variant={currentView === 'security' ? "secondary" : "ghost"} 
          class="w-full justify-start gap-3 font-medium"
          onclick={() => currentView = 'security'}
        >
          <ShieldCheck class="h-4 w-4" />
          Security
        </Button>

        <Button 
          variant={currentView === 'logs' ? "secondary" : "ghost"} 
          class="w-full justify-start gap-3 font-medium"
          onclick={() => currentView = 'logs'}
        >
          <ScrollText class="h-4 w-4" />
          Logs
        </Button>

        <Button 
          variant={currentView === 'about' ? "secondary" : "ghost"} 
          class="w-full justify-start gap-3 font-medium"
          onclick={() => currentView = 'about'}
        >
          <Info class="h-4 w-4" />
          About
        </Button>
      </nav>

      <div class="p-4 border-t bg-background/50">
        <div class="flex items-center gap-3 px-2">
          <div class={`h-2.5 w-2.5 rounded-full shadow-sm ${deviceConnected ? 'bg-green-500' : 'bg-red-500'}`}></div>
          <span class="text-sm font-medium text-muted-foreground">
            {deviceConnected ? "Connected" : "Disconnected"}
          </span>
        </div>
      </div>
    </aside>

    <main class="flex-1 overflow-hidden bg-background">
      <ScrollArea class="h-full">
        <div class="p-8">
          <div class="max-w-4xl mx-auto space-y-8">

            {#if currentView === 'home'}
               <div class="space-y-6">
                  <div class="flex items-center justify-between">
                    <div>
                      <h1 class="text-3xl font-bold tracking-tight">Device Status</h1>
                      <p class="text-muted-foreground mt-1">Overview of connected hardware.</p>
                    </div>
                    <Button variant="outline" size="icon" onclick={checkConnection} disabled={loading}>
                       <RefreshCw class={`h-4 w-4 ${loading ? 'animate-spin' : ''}`} />
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
                          {deviceConnected ? config.productName : "---"}
                        </div>
                        <p class="text-xs text-muted-foreground mt-1">
                          VID: {deviceConnected ? config.vid : "--"} | PID: {deviceConnected ? config.pid : "--"}
                        </p>
                      </Card.Content>
                    </Card.Root>

                    <Card.Root>
                      <Card.Header class="flex flex-row items-center justify-between space-y-0 pb-2">
                        <Card.Title class="text-sm font-medium">Serial Number</Card.Title>
                         <Cpu class="h-4 w-4 text-muted-foreground" />
                      </Card.Header>
                      <Card.Content>
                        <div class="text-xl font-bold font-mono tracking-wide break-all leading-tight">
                          {deviceConnected ? deviceInfo.serial : "---"}
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
                          {deviceConnected ? `${deviceInfo.flashUsed} / ${deviceInfo.flashTotal} KB` : "---"}
                        </div>
                        <div class="w-full bg-secondary h-1.5 mt-3 rounded-full overflow-hidden">
                          <div 
                            class="bg-primary h-full transition-all duration-500" 
                            style={`width: ${deviceConnected && deviceInfo.flashTotal > 0 ? (deviceInfo.flashUsed / deviceInfo.flashTotal) * 100 : 0}%`}>
                          </div>
                        </div>
                      </Card.Content>
                    </Card.Root>

                   <Card.Root>
                      <Card.Header class="flex flex-row items-center justify-between space-y-0 pb-2">
                        <Card.Title class="text-sm font-medium">Firmware</Card.Title>
                        <Info class="h-4 w-4 text-muted-foreground" />
                      </Card.Header>
                      <Card.Content>
                        <div class="text-2xl font-bold">
                          {deviceConnected ? `v${deviceInfo.firmwareVersion}` : "---"}
                        </div>
                      </Card.Content>
                    </Card.Root>

                    <Card.Root>
                      <Card.Header class="flex flex-row items-center justify-between space-y-0 pb-2">
                        <Card.Title class="text-sm font-medium">Security Status</Card.Title>
                        {#if security.secureBoot}
                           <Lock class="h-4 w-4 text-green-500" />
                        {:else}
                           <LockOpen class="h-4 w-4 text-amber-500" />
                        {/if}
                      </Card.Header>
                      <Card.Content>
                        <div class="text-xl font-bold">
                          {deviceConnected ? (security.secureBoot ? "Secure Boot" : "Development") : "---"}
                        </div>
                        <p class="text-xs text-muted-foreground mt-1">
                          {deviceConnected ? (security.secureLock ? "Read-out Locked" : "Debug Enabled") : ""}
                        </p>
                      </Card.Content>
                    </Card.Root>
                  </div>
                
                 {#if !deviceConnected}
                  <Alert.Root class="bg-amber-500/10 border-amber-500/50 text-amber-500">
                     <TriangleAlert class="h-4 w-4" />
                    <Alert.Title>No Device Detected</Alert.Title>
                  <Alert.Description>
                     Please plug in your Pico FIDO key.
                     If you are on Linux, ensure you have the correct udev rules installed.
                  </Alert.Description>
                  </Alert.Root>
                {/if}
              </div>
            {/if}

            {#if currentView === 'config'}
              <div class="space-y-6">
                 <div>
                  <h1 class="text-3xl font-bold tracking-tight">Configuration</h1>
                  <p class="text-muted-foreground">Customize your key's identity and hardware behavior.</p>
                 </div>

                <Card.Root>
                  <Card.Header>
                    <Card.Title>Identity</Card.Title>
                    <Card.Description>USB Identification settings</Card.Description>
                  </Card.Header>
                   <Card.Content class="space-y-4">
                      <div class="space-y-2">
                        <Label>Vendor Preset</Label>
                        <Select.Root type="single" bind:value={selectedVendor} onValueChange={handleVendorChange} disabled={!deviceConnected}>
                          <Select.Trigger class="w-full">
                             {vendors.find(v => v.value === selectedVendor)?.label ?? "Select a vendor"}
                          </Select.Trigger>
                          <Select.Content>
                            {#each vendors as vendor}
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
                        <Input id="vid" bind:value={config.vid} maxlength={4} placeholder="CAFE" disabled={!deviceConnected || selectedVendor !== 'custom'} class="font-mono" />
                       </div>
                      <div class="space-y-2">
                         <Label for="pid">Product ID (HEX)</Label>
                         <Input id="pid" bind:value={config.pid} maxlength={4} placeholder="4242" disabled={!deviceConnected || selectedVendor !== 'custom'} class="font-mono" />
                       </div>
                    </div>
                    <div class="space-y-2">
                       <Label for="product">Product Name</Label>
                       <Input id="product" bind:value={config.productName} placeholder="My Key" disabled={!deviceConnected} />
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
                        <Switch id="secp256k1" bind:checked={config.enableSecp256k1} disabled={!deviceConnected} />
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
                        <Input id="gpio" type="number" bind:value={config.ledGpio} disabled={!deviceConnected} />
                      </div>
                       <div class="space-y-2">
                          <Label for="timeout">Touch Timeout (Seconds)</Label>
                        <Input id="timeout" type="number" bind:value={config.touchTimeout} disabled={!deviceConnected} />
                       </div>
                    </div>

                   <div class="space-y-2 pt-2">
                      <Label>LED Driver</Label>
                      <Select.Root type="single" bind:value={config.ledDriver} disabled={!deviceConnected}>
                        <Select.Trigger class="w-full">
                           {ledDrivers.find(d => d.value === config.ledDriver)?.label ?? "Select Driver"}
                        </Select.Trigger>
                        <Select.Content>
                          {#each ledDrivers as driver}
                              <Select.Item value={driver.value} label={driver.label}>
                              {driver.label}
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
                        <Switch id="led-dimmable" bind:checked={config.ledDimmable} disabled={!deviceConnected} />
                    </div>

                    <div class="flex items-center justify-between space-x-2">
                         <Label for="led-steady" class="flex flex-col space-y-1">
                          <span>LED Steady (No Blink)</span>
                        </Label>
                        <Switch id="led-steady" bind:checked={config.ledSteady} disabled={!deviceConnected} />
                     </div>

                    <div class="flex items-center justify-between space-x-2">
                        <Label for="power-cycle" class="flex flex-col space-y-1">
                            <span>Power Cycle on Reset</span>
                         </Label>
                        <Switch id="power-cycle" bind:checked={config.powerCycleOnReset} disabled={!deviceConnected} />
                    </div>

                   <div class="space-y-3 pt-4">
                      <div class="flex justify-between">
                          <Label>LED Brightness (0-15)</Label>
                          <span class="text-xs text-muted-foreground">Level {config.ledBrightness}</span>
                      </div>
                      <Input type="range" min="0" max="15" bind:value={config.ledBrightness} disabled={!deviceConnected} />
                    </div>
                  </Card.Content>
                  <Card.Footer class="border-t bg-muted/20 px-6 py-4 flex justify-end">
                    <Button onclick={saveConfiguration} disabled={!deviceConnected || loading}>
                      {#if loading} <RefreshCw class="mr-2 h-4 w-4 animate-spin" /> {/if}
                      <Save class="mr-2 h-4 w-4" />
                      Apply Changes
                    </Button>
                   </Card.Footer>
                </Card.Root>
              </div>
            {/if}

            {#if currentView === 'security'}
              <div class="space-y-6">
                 <div>
                  <h1 class="text-3xl font-bold tracking-tight text-destructive">Secure Boot</h1>
                  <p class="text-muted-foreground">Permanently lock this device to the current firmware vendor.</p>
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
                         <p class="font-normal text-xs text-muted-foreground">Verifies firmware signature on startup</p>
                      </div>
                      <Switch id="secure-boot" bind:checked={security.secureBoot} disabled={true} title="Status only - cannot be toggled via software" />
                    </div>

                    <div class="flex items-center justify-between space-x-2">
                      <div class="flex flex-col space-y-1">
                        <Label for="secure-lock">Secure Lock</Label>
                        <p class="font-normal text-xs text-muted-foreground">Prevents reading key material via debug ports</p>
                      </div>
                      <Switch id="secure-lock" bind:checked={security.secureLock} disabled={true} />
                    </div>

                    <Separator />

                    <div class="flex items-center space-x-2 bg-destructive/10 p-4 rounded-md border border-destructive/20">
                        <Switch id="confirm" bind:checked={security.confirmed} disabled={true} />
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

            {#if currentView === 'logs'}
              <div class="space-y-6 h-full flex flex-col">
                <div class="flex items-center justify-between">
                  <div>
                    <h1 class="text-3xl font-bold tracking-tight">System Logs</h1>
                    <p class="text-muted-foreground">Real-time device communication and application events.</p>
                  </div>
                  <Button variant="outline" size="sm" onclick={() => logs = []}>
                    Clear Logs
                  </Button>
                </div>

                <Card.Root class="flex-1 flex flex-col min-h-[500px] bg-black border-zinc-800">
                    <Card.Content class="p-0 flex-1 flex flex-col">
                        {#if logs.length === 0}
                             <div class="flex-1 flex flex-col items-center justify-center text-zinc-500">
                                <Terminal class="h-12 w-12 mb-4 opacity-50" />
                                <p>No events recorded yet.</p>
                             </div>
                        {:else}
                            <ScrollArea class="h-[500px] p-4 font-mono text-sm" orientation="vertical">
                                <div class="space-y-1">
                                    {#each logs as log}
                                        <div class="flex gap-3">
                                            <span class="text-zinc-500 shrink-0">[{log.timestamp}]</span>
                                            <span class={`break-all ${
                                                log.type === 'error' ? 'text-red-400' : 
                                                log.type === 'success' ? 'text-green-400' : 
                                                log.type === 'warning' ? 'text-yellow-400' : 
                                                'text-zinc-300'
                                            }`}>
                                                {#if log.type === 'success'}➜ {/if}
                                                {#if log.type === 'error'}✖ {/if}
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

            {#if currentView === 'about'}
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
                      <Badge variant="secondary" class="px-4 py-1">v0.1.0-Alpha</Badge>
                      <p class="text-muted-foreground max-w-md">
                          An open source commissioning tool for Pico FIDO security keys.
                          Developed with Rust, Tauri, and Svelte.
                      </p>
                      
                      <div class="text-sm text-muted-foreground space-y-1 pt-4 border-t w-full max-w-xs">
                         <div class="flex justify-between"><span>Code By:</span> <span class="font-medium text-foreground">Suyog Tandel</span></div>
                         <div class="flex justify-between"><span>Artwork:</span> <span class="font-medium text-foreground">Suyog Tandel</span></div>
                         <div class="flex justify-between items-center pt-2 mt-2 border-t border-dashed">
                            <span class="flex items-center gap-1"><Copyright class="h-3 w-3" /> Copyright:</span> 
                            <span class="font-medium text-foreground">2026 Suyog Tandel</span>
                         </div>
                      </div>

                      <div class="flex gap-4 pt-6">
                        <Button variant="outline" size="sm" class="gap-2">
                          <Github class="h-4 w-4" />
                            GitHub
                          </Button>
                        <Button variant="outline" size="sm" class="gap-2">
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