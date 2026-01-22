<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";

  interface Props {
    recordingState: "idle" | "recording" | "transcribing" | "inserting";
  }

  interface AudioLevelEvent {
    rms: number;
    peak: number;
  }

  let { recordingState }: Props = $props();

  let elapsedTime = $state(0);
  let waveformBars: number[] = $state(Array(16).fill(4));
  let audioLevel = $state({ rms: 0, peak: 0 });
  let timer: ReturnType<typeof setInterval> | null = null;
  let waveformTimer: ReturnType<typeof setInterval> | null = null;
  let unlistenAudio: UnlistenFn | null = null;

  let isRecording = $derived(recordingState === "recording");
  let isTranscribing = $derived(recordingState === "transcribing");

  onMount(async () => {
    // Start timer when recording
    timer = setInterval(() => {
      if (isRecording) {
        elapsedTime += 0.1;
      }
    }, 100);

    // Listen to real audio level events
    unlistenAudio = await listen<AudioLevelEvent>("audio-level", (event) => {
      audioLevel = event.payload;
    });

    // Animate waveform based on real audio levels
    waveformTimer = setInterval(() => {
      if (isRecording) {
        // Use real RMS level with some variation for visual interest
        const baseHeight = Math.max(4, audioLevel.rms * 60);
        const peakBoost = audioLevel.peak * 40;
        waveformBars = waveformBars.map(() => {
          const variation = (Math.random() - 0.5) * 8;
          return Math.min(32, Math.max(4, baseHeight + variation + peakBoost * Math.random()));
        });
      } else {
        waveformBars = waveformBars.map(() => 4);
      }
    }, 50);
  });

  onDestroy(() => {
    if (timer) clearInterval(timer);
    if (waveformTimer) clearInterval(waveformTimer);
    if (unlistenAudio) unlistenAudio();
  });

  function formatElapsedTime(seconds: number): string {
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    const tenths = Math.floor((seconds % 1) * 10);
    return `${mins}:${secs.toString().padStart(2, "0")}.${tenths}`;
  }
</script>

<div class="overlay-container">
  <div class="overlay">
    <div class="overlay__indicator" class:recording={isRecording} class:processing={isTranscribing}></div>

    <div class="waveform">
      {#each waveformBars as height}
        <div class="waveform__bar" style="height: {height}px"></div>
      {/each}
    </div>

    <div class="overlay__time">
      {#if isTranscribing}
        <span class="processing-text">Processing...</span>
      {:else}
        {formatElapsedTime(elapsedTime)}
      {/if}
    </div>
  </div>
</div>

<style>
  .overlay-container {
    position: fixed;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    z-index: 9999;
  }

  .overlay {
    background: rgba(40, 40, 40, 0.95);
    border: 2px solid #555;
    border-radius: 12px;
    padding: 12px 20px;
    display: flex;
    align-items: center;
    gap: 16px;
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.5);
  }

  .overlay__indicator {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: #666;
    flex-shrink: 0;
  }

  .overlay__indicator.recording {
    background: var(--recording-red);
    animation: pulse 1s infinite;
  }

  .overlay__indicator.processing {
    background: #FFAA00;
    animation: pulse 0.5s infinite;
  }

  .waveform {
    display: flex;
    align-items: center;
    gap: 2px;
    height: 32px;
    min-width: 80px;
  }

  .waveform__bar {
    width: 3px;
    background: #00FF00;
    border-radius: 1px;
    transition: height 0.05s ease;
  }

  .overlay__time {
    font-family: "Monaco", "Consolas", "Courier New", monospace;
    font-size: 16px;
    color: white;
    min-width: 64px;
    text-align: right;
  }

  .processing-text {
    font-size: 12px;
    color: #FFAA00;
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.5; }
  }
</style>
