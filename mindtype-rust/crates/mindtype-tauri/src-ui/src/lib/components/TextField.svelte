<script lang="ts">
  interface Props {
    value?: string;
    placeholder?: string;
    disabled?: boolean;
    type?: "text" | "password";
    id?: string;
    oninput?: (value: string) => void;
  }

  let {
    value = $bindable(""),
    placeholder = "",
    disabled = false,
    type = "text",
    id,
    oninput
  }: Props = $props();

  function handleInput(event: Event) {
    const target = event.target as HTMLInputElement;
    value = target.value;
    oninput?.(target.value);
  }
</script>

<input
  {type}
  {id}
  class="textfield"
  {value}
  {placeholder}
  {disabled}
  oninput={handleInput}
/>
