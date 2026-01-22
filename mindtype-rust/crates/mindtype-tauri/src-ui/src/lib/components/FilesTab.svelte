<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { onMount, onDestroy } from "svelte";
  import ProgressBar from "./ProgressBar.svelte";
  import { t } from "../i18n";

  interface FileJob {
    id: string;
    filename: string;
    status: string;
    progress: number;
    transcription: string | null;
    summary: string | null;
  }

  interface FileJobUpdate {
    id: string;
    status: string;
    progress: number;
  }

  interface SummaryPreset {
    id: string;
    name: string;
    description: string;
  }

  let files: FileJob[] = $state([]);
  let isDragging = $state(false);
  let isProcessing = $state(false);
  let unlistenUpdate: UnlistenFn | null = null;
  let summaryPresets: SummaryPreset[] = $state([]);
  let summarizingIds: Set<string> = $state(new Set());

  onMount(async () => {
    // Load existing files
    await loadFiles();
    await loadPresets();

    // Listen for job updates
    unlistenUpdate = await listen<FileJobUpdate>("file-job-update", (event) => {
      const update = event.payload;
      files = files.map((f) =>
        f.id === update.id
          ? { ...f, status: update.status, progress: update.progress }
          : f
      );
    });
  });

  async function loadPresets() {
    try {
      summaryPresets = await invoke<SummaryPreset[]>("get_summary_presets");
    } catch (error) {
      console.error("Failed to load presets:", error);
    }
  }

  onDestroy(() => {
    if (unlistenUpdate) unlistenUpdate();
  });

  async function loadFiles() {
    try {
      files = await invoke<FileJob[]>("get_file_jobs");
    } catch (error) {
      console.error("Failed to load files:", error);
    }
  }

  function handleDragEnter(e: DragEvent) {
    e.preventDefault();
    isDragging = true;
  }

  function handleDragLeave(e: DragEvent) {
    e.preventDefault();
    isDragging = false;
  }

  function handleDragOver(e: DragEvent) {
    e.preventDefault();
  }

  async function handleDrop(e: DragEvent) {
    e.preventDefault();
    isDragging = false;

    const droppedFiles = e.dataTransfer?.files;
    if (!droppedFiles || droppedFiles.length === 0) return;

    const paths: string[] = [];
    for (let i = 0; i < droppedFiles.length; i++) {
      const file = droppedFiles[i];
      // Get the path from the file - this works in Tauri
      const path = (file as any).path || file.name;
      if (path) {
        paths.push(path);
      }
    }

    if (paths.length > 0) {
      try {
        await invoke("add_files_to_queue", { paths });
        await loadFiles();
      } catch (error) {
        console.error("Failed to add files:", error);
      }
    }
  }

  async function handleFileSelect() {
    try {
      // Open file dialog
      const selected = await invoke<string[] | null>("open_file_dialog");
      if (selected && selected.length > 0) {
        await invoke("add_files_to_queue", { paths: selected });
        await loadFiles();
      }
    } catch (error) {
      console.error("Failed to select files:", error);
    }
  }

  async function startProcessing() {
    if (isProcessing) return;
    isProcessing = true;
    try {
      await invoke("start_file_processing");
    } catch (error) {
      console.error("Failed to start processing:", error);
    } finally {
      isProcessing = false;
      await loadFiles();
    }
  }

  async function removeFile(id: string) {
    try {
      await invoke("remove_file_job", { id });
      files = files.filter((f) => f.id !== id);
    } catch (error) {
      console.error("Failed to remove file:", error);
    }
  }

  async function clearCompleted() {
    try {
      await invoke("clear_completed_jobs");
      await loadFiles();
    } catch (error) {
      console.error("Failed to clear completed:", error);
    }
  }

  async function exportTranscript(id: string, format: string) {
    try {
      await invoke("export_transcript", { id, format });
    } catch (error) {
      console.error("Failed to export transcript:", error);
    }
  }

  async function summarizeFile(id: string, presetId: string) {
    if (summarizingIds.has(id)) return;

    summarizingIds = new Set([...summarizingIds, id]);
    try {
      const preset = presetId === "meeting" ? { meeting: null }
        : presetId === "student" ? { student: null }
        : presetId === "project_manager" ? { project_manager: null }
        : null;

      const summary = await invoke<string>("summarize_file_job", {
        id,
        preset
      });

      // Update local state with the new summary
      files = files.map((f) =>
        f.id === id ? { ...f, summary } : f
      );
    } catch (error) {
      console.error("Failed to summarize:", error);
      alert(`Summarization failed: ${error}`);
    } finally {
      summarizingIds = new Set([...summarizingIds].filter((i) => i !== id));
    }
  }

  function getStatusIcon(status: string): string {
    switch (status) {
      case "Pending":
        return "...";
      case "ExtractingAudio":
        return "~";
      case "Transcribing":
        return "*";
      case "Summarizing":
        return "+";
      case "Completed":
        return "OK";
      default:
        return status.startsWith("Failed") ? "X" : "?";
    }
  }

  function getStatusClass(status: string): string {
    if (status === "Completed") return "status--success";
    if (status.startsWith("Failed")) return "status--error";
    if (status === "Pending") return "status--pending";
    return "status--processing";
  }

  let pendingCount = $derived(
    files.filter((f) => f.status === "Pending").length
  );
  let hasCompleted = $derived(
    files.some((f) => f.status === "Completed")
  );
</script>

<div class="files-tab">
  <div
    class="dropzone panel--sunken"
    class:dragging={isDragging}
    role="button"
    tabindex="0"
    ondragenter={handleDragEnter}
    ondragleave={handleDragLeave}
    ondragover={handleDragOver}
    ondrop={handleDrop}
    onclick={handleFileSelect}
    onkeydown={(e) => e.key === "Enter" && handleFileSelect()}
  >
    {#if isDragging}
      <p class="dropzone__text">{$t("drag_drop_files")}</p>
    {:else}
      <p class="dropzone__text">{$t("drag_drop_files")}</p>
      <p class="dropzone__hint">{$t("or_click_to_select")}</p>
    {/if}
    <p class="dropzone__formats">{$t("supported_formats")}</p>
  </div>

  <div class="file-list">
    {#if files.length === 0}
      <p class="files-tab__empty">{$t("no_files_in_queue")}</p>
    {:else}
      {#each files as file (file.id)}
        <div class="file-item">
          <div class="file-item__header">
            <span class="file-item__name" title={file.filename}>
              {file.filename.length > 30
                ? file.filename.slice(0, 27) + "..."
                : file.filename}
            </span>
            <span
              class="file-item__status {getStatusClass(file.status)}"
              title={file.status}
            >
              {getStatusIcon(file.status)}
            </span>
          </div>

          {#if file.status !== "Completed" && file.status !== "Pending" && !file.status.startsWith("Failed")}
            <ProgressBar value={file.progress} />
          {/if}

          {#if file.summary}
            <div class="file-item__summary">
              <strong>Summary:</strong>
              <p>{file.summary.length > 200 ? file.summary.slice(0, 200) + "..." : file.summary}</p>
            </div>
          {/if}

          <div class="file-item__actions">
            {#if file.status === "Completed"}
              <button
                class="btn btn--small"
                onclick={() => exportTranscript(file.id, "txt")}
              >
                TXT
              </button>
              <button
                class="btn btn--small"
                onclick={() => exportTranscript(file.id, "md")}
              >
                MD
              </button>
              <button
                class="btn btn--small"
                onclick={() => exportTranscript(file.id, "json")}
              >
                JSON
              </button>
              <span class="actions-divider"></span>
              {#if summarizingIds.has(file.id)}
                <span class="btn btn--small btn--disabled">Summarizing...</span>
              {:else if !file.summary}
                <select
                  class="btn btn--small preset-select"
                  onchange={(e) => {
                    const target = e.target as HTMLSelectElement;
                    if (target.value) {
                      summarizeFile(file.id, target.value);
                      target.value = "";
                    }
                  }}
                >
                  <option value="">Summarize...</option>
                  {#each summaryPresets as preset}
                    <option value={preset.id}>{preset.name}</option>
                  {/each}
                </select>
              {:else}
                <button
                  class="btn btn--small"
                  onclick={() => {
                    files = files.map((f) =>
                      f.id === file.id ? { ...f, summary: null } : f
                    );
                  }}
                >
                  Re-summarize
                </button>
              {/if}
            {/if}
            {#if file.status === "Pending" || file.status === "Completed" || file.status.startsWith("Failed")}
              <button
                class="btn btn--small btn--danger"
                onclick={() => removeFile(file.id)}
              >
                {$t("remove_from_queue")}
              </button>
            {/if}
          </div>
        </div>
      {/each}
    {/if}
  </div>

  <div class="files-tab__footer">
    {#if pendingCount > 0}
      <button
        class="btn btn--primary"
        onclick={startProcessing}
        disabled={isProcessing}
      >
        {isProcessing ? $t("status_transcribing") : $t("start_processing")}
      </button>
    {/if}
    {#if hasCompleted}
      <button class="btn" onclick={clearCompleted}>{$t("clear_queue")}</button>
    {/if}
  </div>
</div>

<style>
  .files-tab {
    display: flex;
    flex-direction: column;
    gap: 8px;
    height: 100%;
  }

  .dropzone {
    padding: 24px;
    text-align: center;
    border: 2px dashed var(--frame-inner-dark);
    background: white;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .dropzone:hover,
  .dropzone.dragging {
    border-color: var(--accent-blue);
    background: #f0f8ff;
  }

  .dropzone__text {
    font-size: 12px;
    margin-bottom: 4px;
  }

  .dropzone__hint {
    font-size: 10px;
    color: var(--text-secondary);
    margin-bottom: 8px;
  }

  .dropzone__formats {
    font-size: 9px;
    color: var(--text-tertiary);
  }

  .file-list {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .files-tab__empty {
    font-size: 11px;
    color: var(--text-secondary);
    text-align: center;
    padding: 16px;
  }

  .file-item {
    background: white;
    border: 1px solid var(--frame-inner-dark);
    padding: 8px;
    font-size: 11px;
  }

  .file-item__header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 4px;
  }

  .file-item__name {
    font-weight: bold;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
  }

  .file-item__status {
    font-size: 10px;
    padding: 2px 6px;
    border-radius: 2px;
    font-weight: bold;
    margin-left: 8px;
  }

  .status--pending {
    background: #ddd;
    color: #666;
  }

  .status--processing {
    background: #fff3cd;
    color: #856404;
  }

  .status--success {
    background: #d4edda;
    color: #155724;
  }

  .status--error {
    background: #f8d7da;
    color: #721c24;
  }

  .file-item__summary {
    margin-top: 6px;
    padding: 6px;
    background: #f8f9fa;
    border: 1px solid #e9ecef;
    font-size: 10px;
  }

  .file-item__summary strong {
    display: block;
    margin-bottom: 4px;
  }

  .file-item__summary p {
    margin: 0;
    color: #666;
    line-height: 1.4;
  }

  .file-item__actions {
    display: flex;
    gap: 4px;
    margin-top: 6px;
    flex-wrap: wrap;
    align-items: center;
  }

  .actions-divider {
    width: 1px;
    height: 16px;
    background: var(--frame-inner-dark);
    margin: 0 4px;
  }

  .btn--small {
    font-size: 9px;
    padding: 2px 6px;
  }

  .btn--disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .preset-select {
    min-width: 80px;
    cursor: pointer;
  }

  .btn--danger {
    color: #dc3545;
  }

  .btn--primary {
    background: var(--accent-blue);
    color: white;
  }

  .files-tab__footer {
    display: flex;
    gap: 8px;
    padding: 8px 0;
    border-top: 1px solid var(--frame-inner-dark);
  }
</style>
