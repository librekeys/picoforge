<script lang="ts">
  import { Button } from "$lib/components/ui/button";
  import * as Card from "$lib/components/ui/card";
  import { ScrollArea } from "$lib/components/ui/scroll-area";

  import { logger } from "$lib/services/log.svelte";

  import { Terminal } from "@lucide/svelte";
</script>

<div class="space-y-4 h-full flex flex-col">
  <div class="flex items-center justify-between">
    <div>
      <h1 class="text-3xl font-bold tracking-tight">System Logs</h1>
      <p class="text-muted-foreground">
        Real-time device communication and application events.
      </p>
    </div>
    <Button variant="outline" size="sm" onclick={() => (logger.logs = [])}
      >Clear Logs</Button
    >
  </div>

  <Card.Root
    class="flex-1 flex flex-col min-h-[500px] bg-black border-zinc-800"
  >
    <Card.Content class="p-0 flex-1 flex flex-col">
      {#if logger.logs.length === 0}
        <div
          class="flex-1 flex flex-col items-center justify-center text-zinc-500"
        >
          <Terminal class="h-12 w-12 mb-4 opacity-50" />
          <p>No events recorded yet.</p>
        </div>
      {:else}
        <ScrollArea
          class="h-[500px] p-4 font-mono text-sm"
          orientation="vertical"
        >
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
