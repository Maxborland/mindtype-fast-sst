<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onMount } from "svelte";
  import Window from "../components/Window.svelte";
  import Button from "../components/Button.svelte";
  import RadioButton from "../components/RadioButton.svelte";
  import ProgressBar from "../components/ProgressBar.svelte";
  import TextField from "../components/TextField.svelte";
  import { t, setLocale, type Locale } from "../i18n";

  interface Props {
    initialLanguage?: string;
    onComplete?: () => void;
  }

  let { initialLanguage = "en", onComplete }: Props = $props();

  type Step = "language" | "provider" | "apikey" | "download" | "license" | "complete";

  let currentStep: Step = $state("language");
  let selectedLanguage = $state(initialLanguage);
  let selectedProvider = $state("local");
  let selectedModel = $state("small");
  let selectedLlmProvider = $state("openai");
  let apiKey = $state("");
  let licenseKey = $state("");
  let downloadProgress = $state(0);
  let downloadStatus = $state("");
  let isDownloading = $state(false);
  let isValidatingLicense = $state(false);
  let licenseError = $state("");

  const languages = [
    { code: "en", name: "English", flag: "🇺🇸" },
    { code: "ru", name: "Русский", flag: "🇷🇺" },
    { code: "es", name: "Español", flag: "🇪🇸" },
    { code: "de", name: "Deutsch", flag: "🇩🇪" },
    { code: "fr", name: "Français", flag: "🇫🇷" },
    { code: "zh", name: "中文", flag: "🇨🇳" },
  ];

  const models = [
    { id: "tiny", name: "Tiny", size: "75 MB", speed: "Fastest" },
    { id: "base", name: "Base", size: "150 MB", speed: "Fast" },
    { id: "small", name: "Small", size: "500 MB", speed: "Balanced" },
    { id: "medium", name: "Medium", size: "1.5 GB", speed: "Accurate" },
  ];

  const llmProviders = [
    { id: "openai", name: "OpenAI", placeholder: "sk-...", description: "Easy signup, pay-as-you-go" },
    { id: "anthropic", name: "Anthropic (Claude)", placeholder: "sk-ant-...", description: "Great for long documents" },
    { id: "gemini", name: "Google Gemini", placeholder: "AIza...", description: "Fast, good multilingual" },
    { id: "openrouter", name: "OpenRouter", placeholder: "sk-or-...", description: "Access many models" },
    { id: "ollama", name: "Ollama (Local)", placeholder: "", description: "Run models locally, no API key" },
  ];

  onMount(async () => {
    await listen<{ progress: number; status: string }>("download-progress", (event) => {
      downloadProgress = event.payload.progress * 100;
      downloadStatus = event.payload.status;
    });
  });

  async function handleLanguageNext() {
    await invoke("set_language", { lang: selectedLanguage });
    // Also update the UI locale
    setLocale(selectedLanguage as Locale);
    currentStep = "provider";
  }

  async function handleProviderNext() {
    if (selectedProvider === "local") {
      currentStep = "download";
      await startDownload();
    } else {
      // Cloud provider - need to configure API key
      currentStep = "apikey";
    }
  }

  async function handleApiKeyNext() {
    // Save LLM configuration
    const config = selectedLlmProvider === "ollama"
      ? { type: "ollama", base_url: null, model: null }
      : { type: selectedLlmProvider, api_key: apiKey, model: null };

    try {
      await invoke("set_llm_config", { config });
      // Skip model download for cloud-only users, go to license step
      currentStep = "license";
    } catch (error) {
      console.error("Failed to save LLM config:", error);
    }
  }

  async function handleLicenseNext() {
    if (licenseKey.trim()) {
      isValidatingLicense = true;
      licenseError = "";
      try {
        const isValid = await invoke<boolean>("validate_license", { licenseKey: licenseKey.trim() });
        if (isValid) {
          currentStep = "complete";
        } else {
          licenseError = "Invalid license key";
        }
      } catch (error) {
        licenseError = String(error);
      } finally {
        isValidatingLicense = false;
      }
    } else {
      // Skip license validation - continue as trial
      currentStep = "complete";
    }
  }

  async function startDownload() {
    isDownloading = true;
    try {
      await invoke("download_model", { modelId: selectedModel });
      // Go to license step after download
      currentStep = "license";
    } catch (error) {
      console.error("Download failed:", error);
      downloadStatus = `Error: ${error}`;
    } finally {
      isDownloading = false;
    }
  }

  async function handleComplete() {
    await invoke("complete_setup");
    onComplete?.();
  }
</script>

<Window title="MindType Setup">
  <div class="wizard">
    {#if currentStep === "language"}
      <div class="wizard__step">
        <h2 class="wizard__title">{$t("setup_select_language")}</h2>
        <p class="wizard__description">{$t("setup_welcome")}</p>

        <div class="wizard__options">
          {#each languages as lang}
            <RadioButton
              name="language"
              value={lang.code}
              checked={selectedLanguage === lang.code}
              onchange={(v) => selectedLanguage = v}
            >
              <span class="language-option">
                <span class="language-flag">{lang.flag}</span>
                {lang.name}
              </span>
            </RadioButton>
          {/each}
        </div>

        <div class="wizard__actions">
          <Button primary onclick={handleLanguageNext}>{$t("next")}</Button>
        </div>
      </div>

    {:else if currentStep === "provider"}
      <div class="wizard__step">
        <h2 class="wizard__title">{$t("setup_ai_title")}</h2>
        <p class="wizard__description">{$t("setup_ai_description")}</p>

        <div class="wizard__options">
          <RadioButton
            name="provider"
            value="local"
            checked={selectedProvider === "local"}
            onchange={(v) => selectedProvider = v}
          >
            <div class="provider-option">
              <strong>{$t("local_whisper")}</strong>
              <span>Offline, private, requires download</span>
            </div>
          </RadioButton>

          <RadioButton
            name="provider"
            value="cloud"
            checked={selectedProvider === "cloud"}
            onchange={(v) => selectedProvider = v}
          >
            <div class="provider-option">
              <strong>{$t("cloud_api")}</strong>
              <span>Online, faster, requires API key</span>
            </div>
          </RadioButton>
        </div>

        {#if selectedProvider === "local"}
          <div class="model-select">
            <h3>{$t("select_model")}</h3>
            <div class="wizard__options">
              {#each models as model}
                <RadioButton
                  name="model"
                  value={model.id}
                  checked={selectedModel === model.id}
                  onchange={(v) => selectedModel = v}
                >
                  <div class="model-option">
                    <strong>{model.name}</strong>
                    <span>{model.size} - {model.speed}</span>
                  </div>
                </RadioButton>
              {/each}
            </div>
          </div>
        {/if}

        <div class="wizard__actions">
          <Button onclick={() => currentStep = "language"}>{$t("back")}</Button>
          <Button primary onclick={handleProviderNext}>{$t("next")}</Button>
        </div>
      </div>

    {:else if currentStep === "apikey"}
      <div class="wizard__step">
        <h2 class="wizard__title">{$t("ai_provider")}</h2>
        <p class="wizard__description">{$t("setup_ai_description")}</p>

        <div class="wizard__options">
          {#each llmProviders as provider}
            <RadioButton
              name="llmprovider"
              value={provider.id}
              checked={selectedLlmProvider === provider.id}
              onchange={(v) => selectedLlmProvider = v}
            >
              <div class="provider-option">
                <strong>{provider.name}</strong>
                <span>{provider.description}</span>
              </div>
            </RadioButton>
          {/each}
        </div>

        {#if selectedLlmProvider !== "ollama"}
          <div class="api-key-input">
            <label for="apikey">{$t("api_key")}:</label>
            <TextField
              id="apikey"
              type="password"
              placeholder={llmProviders.find(p => p.id === selectedLlmProvider)?.placeholder}
              bind:value={apiKey}
            />
          </div>
        {/if}

        <div class="wizard__actions">
          <Button onclick={() => currentStep = "provider"}>{$t("back")}</Button>
          <Button primary onclick={handleApiKeyNext}>{$t("next")}</Button>
        </div>
      </div>

    {:else if currentStep === "download"}
      <div class="wizard__step">
        <h2 class="wizard__title">{$t("download_model")}</h2>
        <p class="wizard__description">{downloadStatus}</p>

        <div class="wizard__progress">
          <ProgressBar value={downloadProgress} />
          <span class="progress-text">{Math.round(downloadProgress)}%</span>
        </div>

        <div class="wizard__actions">
          <Button disabled={isDownloading} onclick={() => currentStep = "provider"}>
            {$t("cancel")}
          </Button>
        </div>
      </div>

    {:else if currentStep === "license"}
      <div class="wizard__step">
        <h2 class="wizard__title">{$t("license_activation")}</h2>
        <p class="wizard__description">{$t("enter_license_key")}</p>

        <div class="license-input">
          <label for="license">{$t("license_key")}:</label>
          <TextField
            id="license"
            placeholder="MTXX-XXXX-XXXX-XXXX"
            bind:value={licenseKey}
          />
          {#if licenseError}
            <p class="error-text">{licenseError}</p>
          {/if}
        </div>

        <div class="license-info">
          <p><a href="https://mindtype.space" target="_blank">{$t("buy_license")}</a></p>
        </div>

        <div class="wizard__actions">
          <Button onclick={() => currentStep = selectedProvider === "local" ? "download" : "apikey"}>{$t("back")}</Button>
          <Button
            primary
            onclick={handleLicenseNext}
            disabled={isValidatingLicense}
          >
            {isValidatingLicense ? "..." : licenseKey.trim() ? $t("activate") : $t("skip")}
          </Button>
        </div>
      </div>

    {:else if currentStep === "complete"}
      <div class="wizard__step">
        <h2 class="wizard__title">{$t("setup_complete")}</h2>
        <p class="wizard__description">{$t("setup_instructions")}</p>

        <div class="wizard__icon">✓</div>

        <div class="wizard__actions">
          <Button primary onclick={handleComplete}>{$t("get_started")}</Button>
        </div>
      </div>
    {/if}
  </div>
</Window>

<style>
  .wizard {
    padding: 16px;
    height: 100%;
    display: flex;
    flex-direction: column;
  }

  .wizard__step {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .wizard__title {
    font-size: 16px;
    font-weight: bold;
    margin: 0;
  }

  .wizard__description {
    font-size: 12px;
    color: var(--text-secondary);
  }

  .wizard__options {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .wizard__actions {
    margin-top: auto;
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  .wizard__progress {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .wizard__icon {
    font-size: 48px;
    text-align: center;
    color: #00AA00;
  }

  .language-option {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .language-flag {
    font-size: 16px;
  }

  .provider-option,
  .model-option {
    display: flex;
    flex-direction: column;
  }

  .provider-option span,
  .model-option span {
    font-size: 10px;
    color: var(--text-secondary);
  }

  .model-select {
    margin-top: 8px;
  }

  .model-select h3 {
    font-size: 12px;
    margin-bottom: 8px;
  }

  .progress-text {
    text-align: center;
    font-size: 12px;
  }

  .api-key-input,
  .license-input {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-top: 16px;
  }

  .api-key-input label,
  .license-input label {
    font-size: 12px;
    font-weight: bold;
  }

  .error-text {
    color: #cc0000;
    font-size: 11px;
    margin: 4px 0 0;
  }

  .license-info {
    margin-top: 16px;
    font-size: 11px;
    color: var(--text-secondary);
  }

  .license-info a {
    color: #0066cc;
    text-decoration: none;
  }

  .license-info a:hover {
    text-decoration: underline;
  }
</style>
