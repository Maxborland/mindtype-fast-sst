<script lang="ts">
  interface Tab {
    id: string;
    label: string;
  }

  interface Props {
    tabs: Tab[];
    activeTab: string;
    onTabChange?: (tabId: string) => void;
  }

  let { tabs, activeTab, onTabChange }: Props = $props();

  function handleClick(tabId: string) {
    onTabChange?.(tabId);
  }
</script>

<div class="tabbar">
  {#each tabs as tab}
    <button
      class="tabbar__tab"
      class:tabbar__tab--active={activeTab === tab.id}
      onclick={() => handleClick(tab.id)}
    >
      {tab.label}
    </button>
  {/each}
</div>
