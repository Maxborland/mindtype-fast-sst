<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import TitleBar from "./TitleBar.svelte";

  interface Props {
    title: string;
    dark?: boolean;
    children?: import("svelte").Snippet;
  }

  let { title, dark = false, children }: Props = $props();

  async function handleClose() {
    const win = getCurrentWindow();
    await win.close();
  }
</script>

<div class="window" class:window--dark={dark}>
  <TitleBar {title} onClose={handleClose} />
  <div class="window__content">
    {#if children}
      {@render children()}
    {/if}
  </div>
</div>

<style>
  .window {
    width: 100%;
    height: 100%;
    overflow: hidden;
  }

  .window__content {
    flex: 1;
    overflow: auto;
  }
</style>
