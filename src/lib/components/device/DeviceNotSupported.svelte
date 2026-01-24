<script lang="ts">
  import * as Alert from "$lib/components/ui/alert";
  import { TriangleAlert } from "@lucide/svelte";
  import { device } from "$lib/device/manager.svelte";

  interface Props {
    message?: string;
  }

  let { message = "This device does implement this feature." }: Props =
    $props();
</script>

<Alert.Root variant={device.error ? "destructive" : "default"}>
  <TriangleAlert class="h-4 w-4" />
  <Alert.Title
    >{device.error ? "Connection Error" : "Feature not supported"}</Alert.Title
  >
  <Alert.Description>
    {device.error || message}
  </Alert.Description>
</Alert.Root>
