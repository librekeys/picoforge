<script lang="ts">
  import { Button } from "$lib/components/ui/button";
  import * as Card from "$lib/components/ui/card";
  import { Badge } from "$lib/components/ui/badge";

  import { Github } from "@lucide/svelte";

  import { open } from "@tauri-apps/plugin-shell";

  async function openGithub() {
    await open("https://github.com/librekeys/picoforge");
  }
</script>

<div class="space-y-4">
  <div>
    <h1 class="text-3xl font-bold tracking-tight">About</h1>
  </div>

  <Card.Root>
    <Card.Content class="pt-6">
      <div
        class="flex flex-col items-center justify-center space-y-4 text-center py-8"
      >
        <!-- <div class="bg-primary text-primary p-6 rounded-2xl"> -->
        <!-- <img src="/build-configure-symbolic.svg" alt="PicoForge Logo" class="h-12 w-12" /> -->
        <!-- </div> -->
        <img
          src="/in.suyogtandel.picoforge.svg"
          alt="PicoForge Logo"
          class="h-64 w-64 shadow-sm"
        />

        <h2 class="text-2xl font-bold">PicoForge</h2>
        <Badge variant="secondary" class="px-4 py-1">v0.3.0</Badge>
        <p class="text-muted-foreground max-w-md">
          An open source commissioning tool for Pico FIDO security keys.
          Developed with Rust, Tauri, and Svelte.
        </p>

        <div
          class="text-sm text-muted-foreground space-y-1 pt-4 border-t w-full max-w-xs"
        >
          <div class="flex justify-between">
            <span>Code By:</span>
            <span class="font-medium text-foreground">Suyog Tandel</span>
          </div>
          <div class="flex justify-between items-center pt-2 mt-2">
            <span class="flex items-center gap-1">Copyright:</span>
            <span class="font-medium text-foreground">© 2026 Suyog Tandel</span
            >
          </div>
        </div>

        <div class="flex gap-4 pt-6">
          <Button
            variant="outline"
            size="sm"
            class="gap-2"
            onclick={openGithub}
          >
            <Github class="h-4 w-4" />
            GitHub
          </Button>
        </div>
      </div>
    </Card.Content>
  </Card.Root>
</div>
