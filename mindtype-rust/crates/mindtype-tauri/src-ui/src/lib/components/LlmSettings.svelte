<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import { t } from "../i18n";

  interface LlmProvider {
    id: string;
    name: string;
    requires_api_key: boolean;
    models: string[];
  }

  interface LlmConfig {
    type: string;
    api_key?: string;
    model?: string;
    base_url?: string;
    folder_id?: string;
  }

  let providers: LlmProvider[] = $state([]);
  let currentConfig: LlmConfig = $state({ type: "mindtype_cloud" });
  let selectedProvider = $state("mindtype_cloud");
  let apiKey = $state("");
  let selectedModel = $state("");
  let ollamaUrl = $state("http://localhost:11434");
  let yandexFolderId = $state("");
  let isTesting = $state(false);
  let testResult: { success: boolean; message: string } | null = $state(null);
  let isSaving = $state(false);

  const currentProvider = $derived(providers.find(p => p.id === selectedProvider));

  onMount(async () => {
    await loadProviders();
    await loadConfig();
  });

  async function loadProviders() {
    try {
      providers = await invoke<LlmProvider[]>("get_llm_providers");
    } catch (error) {
      console.error("Failed to load providers:", error);
    }
  }

  async function loadConfig() {
    try {
      currentConfig = await invoke<LlmConfig>("get_llm_config");
      // Map config to UI state
      selectedProvider = currentConfig.type;
      apiKey = currentConfig.api_key || "";
      selectedModel = currentConfig.model || "";
      if (currentConfig.type === "ollama") {
        ollamaUrl = currentConfig.base_url || "http://localhost:11434";
      }
      if (currentConfig.type === "yandex") {
        yandexFolderId = currentConfig.folder_id || "";
      }
    } catch (error) {
      console.error("Failed to load config:", error);
    }
  }

  function buildConfig(): LlmConfig {
    switch (selectedProvider) {
      case "mindtype_cloud":
        return { type: "mindtype_cloud" };
      case "openai":
        return { type: "open_ai", api_key: apiKey, model: selectedModel || undefined };
      case "anthropic":
        return { type: "anthropic", api_key: apiKey, model: selectedModel || undefined };
      case "gemini":
        return { type: "gemini", api_key: apiKey, model: selectedModel || undefined };
      case "openrouter":
        return { type: "open_router", api_key: apiKey, model: selectedModel || undefined };
      case "yandex":
        return { type: "yandex", api_key: apiKey, folder_id: yandexFolderId, model: selectedModel || undefined };
      case "ollama":
        return { type: "ollama", base_url: ollamaUrl || undefined, model: selectedModel || undefined };
      default:
        return { type: "mindtype_cloud" };
    }
  }

  async function saveConfig() {
    isSaving = true;
    testResult = null;
    try {
      const config = buildConfig();
      await invoke("set_llm_config", { config });
      testResult = { success: true, message: "Settings saved!" };
    } catch (error) {
      testResult = { success: false, message: `Failed to save: ${error}` };
    } finally {
      isSaving = false;
    }
  }

  async function testConnection() {
    isTesting = true;
    testResult = null;
    try {
      const config = buildConfig();
      const result = await invoke<boolean>("test_llm_connection", { config });
      testResult = result
        ? { success: true, message: "Connection successful!" }
        : { success: false, message: "Connection failed - check your settings" };
    } catch (error) {
      testResult = { success: false, message: `Test failed: ${error}` };
    } finally {
      isTesting = false;
    }
  }

  function handleProviderChange(e: Event) {
    const target = e.target as HTMLSelectElement;
    selectedProvider = target.value;
    // Reset fields when provider changes
    apiKey = "";
    selectedModel = "";
    testResult = null;
  }
</script>

<div class="llm-settings">
  <h3>{$t("ai_provider")}</h3>
  <p class="description">{$t("setup_ai_description")}</p>

  <div class="field">
    <label for="provider">{$t("llm_provider")}</label>
    <select id="provider" value={selectedProvider} onchange={handleProviderChange}>
      {#each providers as provider}
        <option value={provider.id}>{provider.name}</option>
      {/each}
    </select>
  </div>

  {#if currentProvider?.requires_api_key}
    <div class="field">
      <label for="apiKey">{$t("api_key")}</label>
      <input
        id="apiKey"
        type="password"
        bind:value={apiKey}
        placeholder={$t("api_key")}
      />
    </div>
  {/if}

  {#if selectedProvider === "ollama"}
    <div class="field">
      <label for="ollamaUrl">Ollama URL</label>
      <input
        id="ollamaUrl"
        type="text"
        bind:value={ollamaUrl}
        placeholder="http://localhost:11434"
      />
    </div>
  {/if}

  {#if selectedProvider === "yandex"}
    <div class="field">
      <label for="folderId">Folder ID</label>
      <input
        id="folderId"
        type="text"
        bind:value={yandexFolderId}
        placeholder="Enter Yandex folder ID"
      />
    </div>
  {/if}

  {#if currentProvider && currentProvider.models.length > 0}
    <div class="field">
      <label for="model">{$t("model")}</label>
      <select id="model" bind:value={selectedModel}>
        <option value="">{$t("select_model")}</option>
        {#each currentProvider.models as model}
          <option value={model}>{model}</option>
        {/each}
      </select>
    </div>
  {/if}

  {#if testResult}
    <div class="result" class:success={testResult.success} class:error={!testResult.success}>
      {testResult.message}
    </div>
  {/if}

  <div class="actions">
    <button class="btn btn--secondary" onclick={testConnection} disabled={isTesting}>
      {isTesting ? "..." : "Test"}
    </button>
    <button class="btn btn--primary" onclick={saveConfig} disabled={isSaving}>
      {isSaving ? "..." : $t("save")}
    </button>
  </div>
</div>

<style>
  .llm-settings {
    padding: 8px 0;
  }

  h3 {
    font-size: 12px;
    margin: 0 0 4px 0;
    font-weight: bold;
  }

  .description {
    font-size: 10px;
    color: var(--text-secondary);
    margin: 0 0 12px 0;
  }

  .field {
    margin-bottom: 12px;
  }

  .field label {
    display: block;
    font-size: 11px;
    font-weight: bold;
    margin-bottom: 4px;
  }

  .field input,
  .field select {
    width: 100%;
    padding: 6px 8px;
    font-size: 11px;
    font-family: inherit;
    background: white;
    border: 2px solid;
    border-color: var(--frame-inner-dark) var(--frame-inner-light) var(--frame-inner-light) var(--frame-inner-dark);
    box-sizing: border-box;
  }

  .field input:focus,
  .field select:focus {
    outline: none;
    border-color: var(--accent-blue, #0066CC);
  }

  .result {
    padding: 8px;
    margin-bottom: 12px;
    font-size: 11px;
    border: 1px solid;
  }

  .result.success {
    background: #e6ffe6;
    border-color: #00aa00;
    color: #006600;
  }

  .result.error {
    background: #ffe6e6;
    border-color: #cc0000;
    color: #990000;
  }

  .actions {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
  }

  .btn {
    padding: 6px 16px;
    font-size: 11px;
    font-family: inherit;
    cursor: pointer;
    border: 2px solid;
    border-color: var(--frame-inner-light) var(--frame-inner-dark) var(--frame-inner-dark) var(--frame-inner-light);
    background: var(--button-face, #c0c0c0);
  }

  .btn:hover:not(:disabled) {
    background: var(--button-hover, #d0d0d0);
  }

  .btn:active:not(:disabled) {
    border-color: var(--frame-inner-dark) var(--frame-inner-light) var(--frame-inner-light) var(--frame-inner-dark);
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn--primary {
    background: var(--accent-blue, #0066CC);
    color: white;
    border-color: #004499 #0088ff #0088ff #004499;
  }

  .btn--primary:hover:not(:disabled) {
    background: #0077dd;
  }
</style>
