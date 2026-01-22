<script lang="ts">
  interface Props {
    name: string;
    value: string;
    checked?: boolean;
    onchange?: (value: string) => void;
    children?: import("svelte").Snippet;
  }

  let { name, value, checked = false, onchange, children }: Props = $props();

  function handleChange() {
    onchange?.(value);
  }
</script>

<label class="radio">
  <input
    type="radio"
    class="radio__input"
    {name}
    {value}
    {checked}
    onchange={handleChange}
  />
  <span class="radio__mark"></span>
  <span class="radio__label">
    {#if children}
      {@render children()}
    {/if}
  </span>
</label>
