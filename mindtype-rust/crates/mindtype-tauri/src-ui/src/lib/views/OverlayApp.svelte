<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";

  type RecordingState = "idle" | "recording" | "transcribing" | "inserting";

  interface AudioLevelEvent {
    rms: number;
    peak: number;
  }

  let recordingState: RecordingState = $state("idle");
  let elapsedTime = $state(0);
  let waveformBars: number[] = $state(Array(12).fill(4));
  let audioLevel = $state({ rms: 0, peak: 0 });

  let timer: ReturnType<typeof setInterval> | null = null;
  let waveformTimer: ReturnType<typeof setInterval> | null = null;
  let unlistenState: UnlistenFn | null = null;
  let unlistenAudio: UnlistenFn | null = null;
  let unlistenShow: UnlistenFn | null = null;
  let unlistenHide: UnlistenFn | null = null;

  let isRecording = $derived(recordingState === "recording");
  let isTranscribing = $derived(recordingState === "transcribing");
  let isInserting = $derived(recordingState === "inserting");
  let isActive = $derived(recordingState !== "idle");

  const appWindow = getCurrentWindow();

  onMount(async () => {
    // Listen for recording state changes
    unlistenState = await listen<RecordingState>("recording-state-changed", (event) => {
      recordingState = event.payload;

      // Reset timer when starting to record
      if (event.payload === "recording") {
        elapsedTime = 0;
      }
    });

    // Listen for audio level events
    unlistenAudio = await listen<AudioLevelEvent>("audio-level", (event) => {
      audioLevel = event.payload;
    });

    // Listen for show/hide commands
    unlistenShow = await listen("show-overlay", async () => {
      await appWindow.show();
      await appWindow.setFocus();
    });

    unlistenHide = await listen("hide-overlay", async () => {
      await appWindow.hide();
    });

    // Timer for elapsed time
    timer = setInterval(() => {
      if (isRecording) {
        elapsedTime += 0.1;
      }
    }, 100);

    // Waveform animation based on audio levels
    waveformTimer = setInterval(() => {
      if (isRecording) {
        const baseHeight = Math.max(4, audioLevel.rms * 50);
        const peakBoost = audioLevel.peak * 30;
        waveformBars = waveformBars.map(() => {
          const variation = (Math.random() - 0.5) * 6;
          return Math.min(28, Math.max(4, baseHeight + variation + peakBoost * Math.random()));
        });
      } else if (isTranscribing) {
        // Gentle wave animation during processing
        const time = Date.now() / 200;
        waveformBars = waveformBars.map((_, i) => {
          return 8 + Math.sin(time + i * 0.5) * 6;
        });
      } else {
        waveformBars = waveformBars.map(() => 4);
      }
    }, 50);
  });

  onDestroy(() => {
    if (timer) clearInterval(timer);
    if (waveformTimer) clearInterval(waveformTimer);
    if (unlistenState) unlistenState();
    if (unlistenAudio) unlistenAudio();
    if (unlistenShow) unlistenShow();
    if (unlistenHide) unlistenHide();
  });

  function formatElapsedTime(seconds: number): string {
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    const tenths = Math.floor((seconds % 1) * 10);
    return `${mins}:${secs.toString().padStart(2, "0")}.${tenths}`;
  }
</script>

<div class="overlay" class:active={isActive}>
  <!-- Status indicator -->
  <div
    class="indicator"
    class:recording={isRecording}
    class:processing={isTranscribing}
    class:inserting={isInserting}
  ></div>

  <!-- Waveform visualization -->
  <div class="waveform">
    {#each waveformBars as height}
      <div class="bar" style="height: {height}px"></div>
    {/each}
  </div>

  <!-- Time/Status display -->
  <div class="status">
    {#if isRecording}
      <span class="time">{formatElapsedTime(elapsedTime)}</span>
    {:else if isTranscribing}
      <span class="processing-text">Processing...</span>
    {:else if isInserting}
      <span class="inserting-text">Inserting...</span>
    {:else}
      <span class="ready-text">Ready</span>
    {/if}
  </div>
</div>

<style>
  .overlay {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 16px;
    background: rgba(30, 30, 30, 0.95);
    border: 2px solid #444;
    border-radius: 10px;
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.5);
    opacity: 0;
    transition: opacity 0.2s ease;
  }

  .overlay.active {
    opacity: 1;
  }

  .indicator {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: #666;
    flex-shrink: 0;
  }

  .indicator.recording {
    background: var(--recording-red);
    animation: pulse 1s infinite;
  }

  .indicator.processing {
    background: var(--processing-yellow);
    animation: pulse 0.5s infinite;
  }

  .indicator.inserting {
    background: var(--success-green);
  }

  .waveform {
    display: flex;
    align-items: center;
    gap: 2px;
    height: 28px;
    min-width: 60px;
  }

  .bar {
    width: 3px;
    background: #00CC00;
    border-radius: 1px;
    transition: height 0.05s ease;
  }

  .status {
    font-family: "Monaco", "Consolas", "Courier New", monospace;
    font-size: 14px;
    color: white;
    min-width: 60px;
    text-align: right;
  }

  .time {
    color: white;
  }

  .processing-text {
    font-size: 11px;
    color: var(--processing-yellow);
  }

  .inserting-text {
    font-size: 11px;
    color: var(--success-green);
  }

  .ready-text {
    font-size: 11px;
    color: #888;
  }
</style>
