<script lang="ts">
  import { onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";

  import {
    Home,
    Info,
    KeyRound,
    Maximize,
    Minimize,
    Minus,
    RefreshCw,
    ScrollText,
    Settings,
    ShieldCheck,
    X,
  } from "@lucide/svelte";
  import type { Component } from "svelte";

  import { Button } from "$lib/components/ui/button";
  import { Badge } from "$lib/components/ui/badge";
  import * as Sidebar from "$lib/components/ui/sidebar/index.js";

  import { device } from "$lib/device/manager.svelte";

  type View = "home" | "passkeys" | "config" | "security" | "logs" | "about";

  interface Props {
    currentView: View;
    onViewChange: (view: View) => void;
    children: import("svelte").Snippet;
  }

  let { currentView, onViewChange, children }: Props = $props();

  let isMaximized = $state(false);

  const appWindow = getCurrentWindow();

  const menuItems: Array<{ view: View; icon: Component; label: string }> = [
    { view: "home", icon: Home, label: "Home" },
    { view: "passkeys", icon: KeyRound, label: "Passkeys" },
    { view: "config", icon: Settings, label: "Configuration" },
    { view: "security", icon: ShieldCheck, label: "Security" },
    { view: "logs", icon: ScrollText, label: "Logs" },
    { view: "about", icon: Info, label: "About" },
  ];

  function minimize() {
    appWindow.minimize();
  }

  async function toggleMaximize() {
    await appWindow.toggleMaximize();
  }

  function closeApp() {
    appWindow.close();
  }

  onMount(() => {
    const handleResize = () => {
      appWindow.isMaximized().then((maximized) => {
        isMaximized = maximized;
      });
    };

    handleResize();
    window.addEventListener("resize", handleResize);

    return () => {
      window.removeEventListener("resize", handleResize);
    };
  });
</script>

<Sidebar.Provider>
  <Sidebar.Root collapsible="icon">
    <Sidebar.Header>
      <div class="flex items-center gap-3 p-2">
        <img
          src="/in.suyogtandel.picoforge.svg"
          alt="PicoForge Logo"
          class="h-12 w-12 shadow-sm"
        />
        <span
          class="font-bold text-xl tracking-tight group-data-[collapsible=icon]:hidden"
          >PicoForge</span
        >
      </div>
    </Sidebar.Header>

    <Sidebar.Content>
      <Sidebar.Group>
        <Sidebar.GroupLabel>Menu</Sidebar.GroupLabel>
        <Sidebar.GroupContent>
          <Sidebar.Menu>
            {#each menuItems as item}
              <Sidebar.MenuItem>
                <Sidebar.MenuButton
                  isActive={currentView === item.view}
                  onclick={() => onViewChange(item.view)}
                >
                  <item.icon />
                  <span>{item.label}</span>
                </Sidebar.MenuButton>
              </Sidebar.MenuItem>
            {/each}
          </Sidebar.Menu>
        </Sidebar.GroupContent>
      </Sidebar.Group>
    </Sidebar.Content>

    <Sidebar.Footer
      class="border-t bg-background/50 p-2 group-data-[collapsible=icon]:p-2"
    >
      <div class="p-2 space-y-3 group-data-[collapsible=icon]:hidden">
        <div class="flex items-center justify-between">
          <span class="text-xs font-medium text-muted-foreground"
            >Device Status</span
          >
          {#if device.connected}
            {#if device.method === "FIDO"}
              <Badge
                variant="default"
                class="bg-amber-500 hover:bg-amber-500 text-[10px] px-1.5 h-5"
                >Online - Fido</Badge
              >
            {:else}
              <Badge
                variant="default"
                class="bg-green-600 hover:bg-green-600 text-[10px] px-1.5 h-5"
                >Online</Badge
              >
            {/if}
          {:else if device.error}
            <Badge
              variant="destructive"
              class="bg-amber-600 hover:bg-amber-600 text-[10px] px-1.5 h-5"
              >Error</Badge
            >
          {:else}
            <Badge variant="destructive" class="text-[10px] px-1.5 h-5"
              >Offline</Badge
            >
          {/if}
        </div>

        <Button
          variant="outline"
          size="sm"
          class="w-full gap-2"
          disabled={device.loading}
          onclick={() => device.refresh()}
        >
          {#if device.loading}
            <RefreshCw class="h-3.5 w-3.5 animate-spin" />
          {:else}
            <RefreshCw class="h-3.5 w-3.5" />
          {/if}
          Refresh
        </Button>
      </div>
      <div
        class="hidden group-data-[collapsible=icon]:flex flex-col items-center justify-center p-2 gap-2"
      >
        <Button
          variant="ghost"
          size="icon"
          disabled={device.loading}
          onclick={() => device.refresh()}
        >
          <RefreshCw class="h-4 w-4 {device.loading ? 'animate-spin' : ''}" />
        </Button>
        <div
          class={`h-2 w-2 rounded-full ${device.connected ? (device.method === "FIDO" ? "bg-amber-500" : "bg-green-500") : device.error ? "bg-amber-500" : "bg-red-500"}`}
        ></div>
      </div>
    </Sidebar.Footer>
  </Sidebar.Root>

  <Sidebar.Inset>
    <header
      data-tauri-drag-region
      class="h-10 bg-background border-b flex items-center justify-between px-2 select-none sticky top-0 z-10"
    >
      <div class="flex items-center gap-2">
        <Sidebar.Trigger class="h-8 w-8" />
        <div
          class="text-xs font-medium text-muted-foreground pointer-events-none flex items-center gap-2"
        ></div>
      </div>

      <div class="flex items-center gap-1">
        <Button
          variant="ghost"
          size="icon"
          class="h-8 w-8 hover:bg-muted"
          onclick={minimize}
        >
          <Minus class="h-4 w-4" />
        </Button>

        <Button
          variant="ghost"
          size="icon"
          class="h-8 w-8 hover:bg-muted"
          onclick={toggleMaximize}
        >
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

    <main class="flex-1 overflow-hidden h-[calc(100vh-2.5rem)]">
      {@render children()}
    </main>
  </Sidebar.Inset>
</Sidebar.Provider>
