<script lang="ts">
  interface Props {
    title: string;
    onClose?: () => void;
  }

  let { title, onClose }: Props = $props();
</script>

<div class="titlebar drag-region">
  <button class="titlebar__close no-drag" onclick={onClose} aria-label="Close">
  </button>
  <div class="titlebar__stripes"></div>
  <span class="titlebar__title">{title}</span>
  <div class="titlebar__stripes"></div>
</div>

<style>
  /* Component styles are in system7.css */
</style>
