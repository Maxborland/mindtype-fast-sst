<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import Window from "./lib/components/Window.svelte";
  import Wizard from "./lib/views/Wizard.svelte";
  import MainWindow from "./lib/views/MainWindow.svelte";
  import ErrorBoundary from "./lib/components/ErrorBoundary.svelte";
  import { setupGlobalErrorHandlers, captureError } from "./lib/utils/crashReporter";

  interface AppState {
    settings: {
      language: string;
      model_id: string;
      hotkey: string;
      setup_completed: boolean;
    };
    recording_state: "idle" | "recording" | "transcribing" | "inserting";
    setup_completed: boolean;
  }

  let appState: AppState | null = $state(null);
  let loading = $state(true);

  onMount(async () => {
    // Set up global error handlers for crash reporting
    setupGlobalErrorHandlers();

    try {
      // Get initial app state
      appState = await invoke<AppState>("get_app_state");
      loading = false;

      // Listen for state changes
      await listen("recording-state-changed", (event) => {
        if (appState) {
          appState.recording_state = event.payload as AppState["recording_state"];
        }
      });

      // Listen for tray menu events
      await listen("tray-start-recording", async () => {
        if (appState?.recording_state === "idle") {
          await invoke("start_recording");
        }
      });

      await listen("tray-open-settings", () => {
        // Switch to settings tab - handled by MainWindow
      });

      // Listen for hotkey events
      await listen("hotkey-pressed", async () => {
        await invoke("start_recording");
      });

      await listen("hotkey-released", async () => {
        const audioData = await invoke<number[]>("stop_recording");
        await invoke("transcribe", { audioData });
      });
    } catch (error) {
      console.error("Failed to initialize app:", error);
      if (error instanceof Error) {
        captureError(error, "Failed to initialize app");
      }
      loading = false;
    }
  });

  function handleSetupComplete() {
    if (appState) {
      appState.setup_completed = true;
      appState.settings.setup_completed = true;
    }
  }
</script>

<ErrorBoundary>
  {#snippet children()}
    {#if loading}
      <Window title="MindType">
        <div class="flex items-center justify-center h-full">
          <p>Loading...</p>
        </div>
      </Window>
    {:else if appState && !appState.setup_completed}
      <Wizard
        initialLanguage={appState.settings.language}
        onComplete={handleSetupComplete}
      />
    {:else if appState}
      <MainWindow
        recordingState={appState.recording_state}
        settings={appState.settings}
      />
    {/if}
  {/snippet}
</ErrorBoundary>
