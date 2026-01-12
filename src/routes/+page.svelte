<script lang="ts">
  import { onMount, tick } from "svelte";

  import { ScrollArea } from "$lib/components/ui/scroll-area";

  import { logger } from "$lib/services/log.svelte";
  import { device } from "$lib/device/manager.svelte";

  import SidebarMenu from "$lib/layout/sidebar.svelte";
  import HomeView from "$lib/views/homeView.svelte";
  import ConfigView from "$lib/views/configView.svelte";
  import SecurityView from "$lib/views/securityView.svelte";
  import LogsView from "$lib/views/logsView.svelte";
  import AboutView from "$lib/views/aboutView.svelte";

  import SetPinDialog from "$lib/components/dialogs/setPinDialog.svelte";
  import MinPinDialog from "$lib/components/dialogs/minPinDialog.svelte";
  import MessageDialog from "$lib/components/dialogs/messageDialog.svelte";

  type View = "home" | "config" | "security" | "logs" | "about";
  let currentView: View = $state("home");

  const viewMap = {
    home: HomeView,
    config: ConfigView,
    security: SecurityView,
    logs: LogsView,
    about: AboutView,
  };

  let ActiveView = $derived(viewMap[currentView]);

  $effect(() => {
    logger.logs.length;
    tick().then(() => {
      const viewport = document.querySelector("[data-radix-scroll-area-viewport]");
      if (viewport) {
        viewport.scrollTop = viewport.scrollHeight;
      }
    });
  });

  onMount(() => {
    document.documentElement.classList.add("dark");
    if (logger.logs.length === 0) logger.add("Application started.", "info");
    device.refresh();
  });
</script>

<SidebarMenu {currentView} onViewChange={(view) => (currentView = view)}>
  <ScrollArea class="h-full mr-1">
    <div class="container mx-auto py-8 px-8 max-w-6xl">
      <div class="space-y-8">
        <ActiveView />
      </div>
    </div>
  </ScrollArea>
</SidebarMenu>

<MessageDialog />
<SetPinDialog />
<MinPinDialog />
