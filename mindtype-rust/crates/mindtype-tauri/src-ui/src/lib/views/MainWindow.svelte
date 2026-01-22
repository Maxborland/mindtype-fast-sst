<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import Window from "../components/Window.svelte";
  import TabBar from "../components/TabBar.svelte";
  import Button from "../components/Button.svelte";
  import FilesTab from "../components/FilesTab.svelte";
  import LlmSettings from "../components/LlmSettings.svelte";
  import LicenseSettings from "../components/LicenseSettings.svelte";
  import { t, locale, setLocale, SUPPORTED_LOCALES, LOCALE_NAMES, type Locale } from "../i18n";

  interface Settings {
    language: string;
    model_id: string;
    hotkey: string;
    setup_completed: boolean;
  }

  interface Transcription {
    id: string;
    text: string;
    language: string;
    duration_ms: number;
    timestamp: string;
  }

  interface Props {
    recordingState: "idle" | "recording" | "transcribing" | "inserting";
    settings: Settings;
  }

  let { recordingState, settings }: Props = $props();

  // Tabs use translated labels
  const tabs = $derived([
    { id: "basic", label: $t("basic") },
    { id: "files", label: $t("files_tab") },
    { id: "settings", label: $t("settings") },
  ]);

  let activeTab = $state("basic");
  let transcriptions: Transcription[] = $state([]);
  let isRecording = $derived(recordingState === "recording");

  onMount(async () => {
    await loadTranscriptions();
  });

  async function loadTranscriptions() {
    try {
      transcriptions = await invoke<Transcription[]>("get_recent_transcriptions", { limit: 20 });
    } catch (error) {
      console.error("Failed to load transcriptions:", error);
    }
  }

  async function handleManualRecord() {
    if (recordingState === "idle") {
      await invoke("start_recording");
    } else if (recordingState === "recording") {
      const audioData = await invoke<number[]>("stop_recording");
      await invoke("transcribe", { audioData });
      await loadTranscriptions();
    }
  }

  function formatTime(timestamp: string): string {
    const date = new Date(timestamp);
    return date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  }

  function truncateText(text: string, maxLength: number): string {
    if (text.length <= maxLength) return text;
    return text.slice(0, maxLength) + "...";
  }
</script>

<Window title="MindType" dark>
  <div class="main">
    <TabBar {tabs} {activeTab} onTabChange={(id) => activeTab = id} />

    <div class="main__content panel">
      {#if activeTab === "basic"}
        <div class="basic-tab">
          <div class="history">
            <h3>{$t("journal")}</h3>
            {#if transcriptions.length === 0}
              <p class="history__empty">{$t("no_transcriptions")}</p>
            {:else}
              <div class="list">
                {#each transcriptions as t}
                  <div class="list__item">
                    <span class="history__time">{formatTime(t.timestamp)}</span>
                    <span class="history__text">{truncateText(t.text, 50)}</span>
                  </div>
                {/each}
              </div>
            {/if}
          </div>

          <div class="basic-tab__footer">
            <button
              class="btn btn--record"
              class:recording={isRecording}
              onclick={handleManualRecord}
              aria-label={isRecording ? "Stop recording" : "Start recording"}
            >
              {#if isRecording}
                ■
              {:else}
                ●
              {/if}
            </button>
            <span class="shortcut-hint">
              or press <strong>{settings.hotkey}</strong>
            </span>
          </div>
        </div>

      {:else if activeTab === "files"}
        <FilesTab />

      {:else if activeTab === "settings"}
        <div class="settings-tab">
          <div class="settings-section">
            <h3>{$t("general")}</h3>
            <div class="setting">
              <label class="setting__label">{$t("ui_language")}</label>
              <select class="setting__select" value={$locale} onchange={(e) => setLocale(e.currentTarget.value as Locale)}>
                {#each SUPPORTED_LOCALES as loc}
                  <option value={loc}>{LOCALE_NAMES[loc]}</option>
                {/each}
              </select>
            </div>
            <div class="setting">
              <label class="setting__label">{$t("hotkey")}</label>
              <span class="setting__value">{settings.hotkey}</span>
            </div>
            <div class="setting">
              <label class="setting__label">{$t("model")}</label>
              <span class="setting__value">{settings.model_id}</span>
            </div>
            <div class="setting">
              <label class="setting__label">{$t("transcription_language")}</label>
              <span class="setting__value">{settings.language.toUpperCase()}</span>
            </div>
          </div>

          <div class="settings-divider"></div>

          <LicenseSettings />

          <div class="settings-divider"></div>

          <LlmSettings />
        </div>
      {/if}
    </div>

    <div class="main__status">
      <span class="status__indicator" class:recording={isRecording}></span>
      <span class="status__text">
        {#if recordingState === "idle"}
          {$t("ready")}
        {:else if recordingState === "recording"}
          {$t("recording")}
        {:else if recordingState === "transcribing"}
          {$t("transcribing")}
        {:else}
          {$t("success")}
        {/if}
      </span>
    </div>
  </div>
</Window>

<style>
  .main {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .main__content {
    flex: 1;
    margin: 0 8px;
    overflow: auto;
  }

  .main__status {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    background: var(--main-bg);
    border-top: 1px solid var(--frame-outer);
  }

  .status__indicator {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #00AA00;
  }

  .status__indicator.recording {
    background: var(--recording-red);
    animation: pulse 1s infinite;
  }

  .status__text {
    font-size: 11px;
  }

  /* Basic Tab */
  .basic-tab {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .history {
    flex: 1;
    overflow: auto;
  }

  .history h3 {
    font-size: 12px;
    margin-bottom: 8px;
  }

  .history__empty {
    font-size: 11px;
    color: var(--text-secondary);
    text-align: center;
    padding: 32px;
  }

  .history__time {
    font-size: 10px;
    color: var(--text-secondary);
    min-width: 50px;
  }

  .history__text {
    flex: 1;
  }

  .basic-tab__footer {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 16px;
    padding: 16px;
    border-top: 1px solid var(--frame-inner-dark);
  }

  .btn--record.recording {
    animation: pulse 1s infinite;
  }

  .shortcut-hint {
    font-size: 11px;
    color: var(--text-secondary);
  }

  /* Settings Tab */
  .settings-tab {
    display: flex;
    flex-direction: column;
    gap: 8px;
    overflow-y: auto;
    padding-right: 4px;
  }

  .settings-section h3 {
    font-size: 12px;
    margin: 0 0 8px 0;
    font-weight: bold;
  }

  .settings-divider {
    height: 1px;
    background: var(--frame-inner-dark);
    margin: 8px 0;
  }

  .setting {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 6px 0;
    border-bottom: 1px solid var(--frame-inner-dark);
  }

  .setting__label {
    font-weight: bold;
    font-size: 11px;
  }

  .setting__value {
    font-size: 11px;
    color: var(--text-secondary);
  }

  .setting__select {
    font-family: var(--font-system);
    font-size: 11px;
    padding: 2px 4px;
    background: var(--main-bg);
    border: 1px solid var(--frame-outer);
    color: var(--text-primary);
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.5; }
  }
</style>
