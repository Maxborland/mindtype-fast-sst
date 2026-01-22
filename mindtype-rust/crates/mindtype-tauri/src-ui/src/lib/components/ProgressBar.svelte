<script lang="ts">
  interface Props {
    value: number; // 0-100
  }

  let { value }: Props = $props();

  let clampedValue = $derived(Math.max(0, Math.min(100, value)));
</script>

<div class="progress">
  <div class="progress__bar" style="width: {clampedValue}%"></div>
</div>
