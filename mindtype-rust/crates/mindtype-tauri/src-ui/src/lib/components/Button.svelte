<script lang="ts">
  interface Props {
    primary?: boolean;
    disabled?: boolean;
    onclick?: () => void;
    children?: import("svelte").Snippet;
  }

  let { primary = false, disabled = false, onclick, children }: Props = $props();
</script>

<button
  class="btn"
  class:btn--primary={primary}
  {disabled}
  {onclick}
>
  {#if children}
    {@render children()}
  {/if}
</button>
