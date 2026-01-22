//! Tauri IPC commands

use crate::crash_reporter::{add_breadcrumb, CrashReport};
use crate::state::{AppState, LlmConfig, RecordingState, Settings, Transcription};
use crate::tray::{update_tray_state, TrayState};
use chrono;
use serde::{Deserialize, Serialize};
use tauri::{Emitter, State, Window};

/// App state response for frontend
#[derive(Debug, Serialize)]
pub struct AppStateResponse {
    pub settings: Settings,
    pub recording_state: RecordingState,
    pub setup_completed: bool,
}

/// Get current application state
#[tauri::command]
pub async fn get_app_state(state: State<'_, AppState>) -> Result<AppStateResponse, String> {
    let settings = state.settings.read().await.clone();
    let recording_state = state.get_recording_state().await;

    Ok(AppStateResponse {
        setup_completed: settings.setup_completed,
        settings,
        recording_state,
    })
}

/// Set application language
#[tauri::command]
pub async fn set_language(lang: String, state: State<'_, AppState>) -> Result<(), String> {
    {
        let mut settings = state.settings.write().await;
        settings.language = lang;
    }
    state.save_settings().await.map_err(|e| e.to_string())?;
    Ok(())
}

/// Download progress event payload
#[derive(Clone, Serialize)]
pub struct DownloadProgress {
    pub progress: f32,
    pub status: String,
}

/// Audio level event payload for waveform visualization
#[derive(Clone, Serialize)]
pub struct AudioLevelEvent {
    pub rms: f32,
    pub peak: f32,
}

/// Download Whisper model
#[tauri::command]
pub async fn download_model(
    model_id: String,
    window: Window,
    state: State<'_, AppState>,
) -> Result<(), String> {
    use futures_util::StreamExt;
    use tokio::io::AsyncWriteExt;

    // Emit progress events
    let emit_progress = |progress: f32, status: &str| {
        let _ = window.emit(
            "download-progress",
            DownloadProgress {
                progress,
                status: status.to_string(),
            },
        );
    };

    emit_progress(0.0, "Preparing download...");

    // Get model download URLs based on model_id
    // We need encoder.onnx, decoder.onnx, and vocab.json for ONNX models
    let base_url = format!(
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-{}.bin",
        model_id
    );

    let models_dir = state.models_dir();
    let model_dir = models_dir.join(&model_id);
    std::fs::create_dir_all(&model_dir).map_err(|e| e.to_string())?;

    emit_progress(0.1, "Connecting to server...");

    // Download the model file
    let client = reqwest::Client::new();
    let response = client
        .get(&base_url)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        return Err(format!("Failed to download model: HTTP {}", response.status()));
    }

    let total_size = response.content_length().unwrap_or(0);
    let mut downloaded: u64 = 0;

    emit_progress(0.2, "Downloading model...");

    let model_path = model_dir.join("model.bin");
    let mut file = tokio::fs::File::create(&model_path)
        .await
        .map_err(|e| e.to_string())?;

    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        file.write_all(&chunk).await.map_err(|e| e.to_string())?;
        downloaded += chunk.len() as u64;

        if total_size > 0 {
            let progress = 0.2 + 0.7 * (downloaded as f32 / total_size as f32);
            emit_progress(progress, "Downloading model...");
        }
    }

    file.flush().await.map_err(|e| e.to_string())?;

    emit_progress(0.95, "Model downloaded!");

    // Update settings with new model
    {
        let mut settings = state.settings.write().await;
        settings.model_id = model_id;
    }
    state.save_settings().await.map_err(|e| e.to_string())?;

    emit_progress(1.0, "Model ready!");

    Ok(())
}

/// Complete setup wizard
#[tauri::command]
pub async fn complete_setup(state: State<'_, AppState>) -> Result<(), String> {
    {
        let mut settings = state.settings.write().await;
        settings.setup_completed = true;
    }
    state.save_settings().await.map_err(|e| e.to_string())?;
    Ok(())
}

/// Start recording audio
#[tauri::command]
pub async fn start_recording(window: Window, state: State<'_, AppState>) -> Result<(), String> {
    add_breadcrumb("Started recording");

    let current_state = state.get_recording_state().await;
    if current_state != RecordingState::Idle {
        return Err("Already recording or processing".to_string());
    }

    state.set_recording_state(RecordingState::Recording).await;

    // Save the currently focused window
    state.platform.save_foreground_window();

    // Initialize audio recorder
    let mut recorder = mindtype_core::AudioRecorder::new().map_err(|e| e.to_string())?;
    recorder.select_device(None).map_err(|e| e.to_string())?;

    // Set up audio level callback for waveform visualization
    let mut level_rx = recorder.set_level_callback();

    // Spawn task to forward audio levels to the frontend
    let window_clone = window.clone();
    tokio::spawn(async move {
        while let Some(level) = level_rx.recv().await {
            let _ = window_clone.emit("audio-level", AudioLevelEvent {
                rms: level.rms,
                peak: level.peak,
            });
        }
    });

    recorder.start().map_err(|e| e.to_string())?;
    *state.audio_recorder.write().await = Some(recorder);

    // Update tray icon
    update_tray_state(TrayState::Recording);

    // Emit state change
    let _ = window.emit("recording-state-changed", RecordingState::Recording);
    let _ = window.emit("show-overlay", ());

    Ok(())
}

/// Stop recording and get audio data
#[tauri::command]
pub async fn stop_recording(
    window: Window,
    state: State<'_, AppState>,
) -> Result<Vec<f32>, String> {
    add_breadcrumb("Stopped recording");

    let current_state = state.get_recording_state().await;
    if current_state != RecordingState::Recording {
        return Err("Not currently recording".to_string());
    }

    state
        .set_recording_state(RecordingState::Transcribing)
        .await;

    // Update tray icon to processing
    update_tray_state(TrayState::Processing);

    let _ = window.emit("recording-state-changed", RecordingState::Transcribing);

    // Stop recorder and get audio data
    let mut recorder_guard = state.audio_recorder.write().await;
    let mut recorder = recorder_guard
        .take()
        .ok_or_else(|| "No recorder active".to_string())?;

    let audio_data = recorder.stop().map_err(|e| e.to_string())?;

    // Hide overlay when done recording
    let _ = window.emit("hide-overlay", ());

    Ok(audio_data)
}

/// Transcribe audio data
#[tauri::command]
pub async fn transcribe(
    audio_data: Vec<f32>,
    window: Window,
    state: State<'_, AppState>,
) -> Result<Transcription, String> {
    add_breadcrumb(format!("Transcribing {} samples", audio_data.len()));

    let mut transcriber_guard = state.transcriber.write().await;
    let transcriber = transcriber_guard
        .as_mut()
        .ok_or_else(|| "Transcriber not initialized".to_string())?;

    let start_time = std::time::Instant::now();
    let result = transcriber
        .transcribe(&audio_data, "auto")
        .await
        .map_err(|e| e.to_string())?;
    let duration_ms = start_time.elapsed().as_millis() as u64;

    let transcription = Transcription {
        id: uuid::Uuid::new_v4().to_string(),
        text: result.text.clone(),
        language: result.language.clone(),
        duration_ms,
        timestamp: chrono::Utc::now(),
    };

    // Add to history
    state.add_transcription(transcription.clone()).await;

    // Update state
    state.set_recording_state(RecordingState::Idle).await;

    // Update tray icon back to idle
    update_tray_state(TrayState::Idle);

    let _ = window.emit("recording-state-changed", RecordingState::Idle);

    Ok(transcription)
}

/// Get recent transcriptions
#[tauri::command]
pub async fn get_recent_transcriptions(
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<Transcription>, String> {
    let transcriptions = state.transcriptions.read().await;
    let limit = limit.unwrap_or(20);
    Ok(transcriptions.iter().take(limit).cloned().collect())
}

/// Insert text at cursor position
#[tauri::command]
pub async fn insert_text(text: String, state: State<'_, AppState>) -> Result<(), String> {
    add_breadcrumb(format!("Inserting {} chars", text.len()));

    state
        .set_recording_state(RecordingState::Inserting)
        .await;

    // Restore the previously focused window and insert text
    state
        .platform
        .restore_foreground_window()
        .map_err(|e| e.to_string())?;

    // Small delay to ensure focus is restored
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    state
        .platform
        .insert_text(&text)
        .map_err(|e| e.to_string())?;

    state.set_recording_state(RecordingState::Idle).await;

    Ok(())
}

/// Get current settings
#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<Settings, String> {
    let settings = state.settings.read().await;
    Ok(settings.clone())
}

/// Update settings
#[derive(Debug, Deserialize)]
pub struct UpdateSettingsPayload {
    pub language: Option<String>,
    pub model_id: Option<String>,
    pub hotkey: Option<String>,
}

#[tauri::command]
pub async fn update_settings(
    payload: UpdateSettingsPayload,
    state: State<'_, AppState>,
) -> Result<Settings, String> {
    {
        let mut settings = state.settings.write().await;
        if let Some(language) = payload.language {
            settings.language = language;
        }
        if let Some(model_id) = payload.model_id {
            settings.model_id = model_id;
        }
        if let Some(hotkey) = payload.hotkey {
            settings.hotkey = hotkey;
        }
    }
    state.save_settings().await.map_err(|e| e.to_string())?;

    let settings = state.settings.read().await;
    Ok(settings.clone())
}

/// Validate license
#[tauri::command]
pub async fn validate_license(
    license_key: String,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let mut manager = state.license_manager.write().await;
    manager.set_license_key(&license_key);
    let status = manager.validate().await.map_err(|e| e.to_string())?;
    Ok(status.is_valid())
}

/// License status response for frontend
#[derive(Debug, Serialize)]
pub struct LicenseStatusResponse {
    pub activated: bool,
    pub license_key: Option<String>,
    pub plan: Option<String>,
    pub expires_at: Option<String>,
    pub days_remaining: Option<i64>,
    pub credits_remaining: Option<u32>,
}

/// Get current license status
#[tauri::command]
pub async fn get_license_status(
    state: State<'_, AppState>,
) -> Result<LicenseStatusResponse, String> {
    let mut manager = state.license_manager.write().await;
    let status = manager.validate().await.map_err(|e| e.to_string())?;

    match status {
        mindtype_licensing::LicenseStatus::Valid { plan, expires_at } => {
            let days_remaining = expires_at.map(|exp| {
                (exp - chrono::Utc::now()).num_days()
            });

            Ok(LicenseStatusResponse {
                activated: true,
                license_key: manager.license_key().map(|s| s.to_string()),
                plan: Some(plan.to_string()),
                expires_at: expires_at.map(|dt| dt.to_rfc3339()),
                days_remaining,
                credits_remaining: None, // TODO: Implement credits
            })
        }
        mindtype_licensing::LicenseStatus::Trial { days_left, .. } => {
            Ok(LicenseStatusResponse {
                activated: false,
                license_key: None,
                plan: None,
                expires_at: None,
                days_remaining: Some(days_left as i64),
                credits_remaining: None,
            })
        }
        _ => {
            Ok(LicenseStatusResponse {
                activated: false,
                license_key: None,
                plan: None,
                expires_at: None,
                days_remaining: None,
                credits_remaining: None,
            })
        }
    }
}

/// Activate a license key
#[tauri::command]
pub async fn activate_license(
    license_key: String,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    add_breadcrumb("Activating license");
    let mut manager = state.license_manager.write().await;
    manager.set_license_key(&license_key);
    let status = manager.validate().await.map_err(|e| e.to_string())?;

    if status.is_valid() {
        // Save the license key to settings
        {
            let mut settings = state.settings.write().await;
            settings.license_key = Some(license_key);
        }
        state.save_settings().await.map_err(|e| e.to_string())?;
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Deactivate the current license
#[tauri::command]
pub async fn deactivate_license(
    state: State<'_, AppState>,
) -> Result<(), String> {
    add_breadcrumb("Deactivating license");
    let manager = state.license_manager.read().await;
    manager.deactivate().await.map_err(|e| e.to_string())?;
    drop(manager);

    // Clear the license key from settings
    {
        let mut settings = state.settings.write().await;
        settings.license_key = None;
    }
    state.save_settings().await.map_err(|e| e.to_string())?;
    Ok(())
}

/// Open a URL in the default browser
#[tauri::command]
pub async fn open_url(url: String) -> Result<(), String> {
    open::that(&url).map_err(|e| e.to_string())
}

// =====================
// File Processing Commands
// =====================

/// File job response for frontend
#[derive(Debug, Clone, Serialize)]
pub struct FileJobResponse {
    pub id: String,
    pub filename: String,
    pub status: String,
    pub progress: u8,
    pub transcription: Option<String>,
    pub summary: Option<String>,
}

/// File job update event
#[derive(Debug, Clone, Serialize)]
pub struct FileJobUpdateEvent {
    pub id: String,
    pub status: String,
    pub progress: u8,
}

/// Add files to the processing queue
#[tauri::command]
pub async fn add_files_to_queue(
    paths: Vec<String>,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    add_breadcrumb(format!("Adding {} files to queue", paths.len()));

    let processor = state.file_processor.read().await;
    let path_bufs: Vec<std::path::PathBuf> = paths.into_iter().map(std::path::PathBuf::from).collect();
    let ids = processor.add_files(path_bufs).await;
    Ok(ids.into_iter().map(|id| id.to_string()).collect())
}

/// Get all file jobs
#[tauri::command]
pub async fn get_file_jobs(state: State<'_, AppState>) -> Result<Vec<FileJobResponse>, String> {
    let processor = state.file_processor.read().await;
    let jobs = processor.get_jobs().await;

    Ok(jobs
        .into_iter()
        .map(|job| FileJobResponse {
            id: job.id.to_string(),
            filename: job.filename,
            status: job.status.to_string(),
            progress: job.progress,
            transcription: job.transcription,
            summary: job.summary,
        })
        .collect())
}

/// Start processing all pending files
#[tauri::command]
pub async fn start_file_processing(
    window: Window,
    state: State<'_, AppState>,
) -> Result<(), String> {
    add_breadcrumb("Starting file processing");

    // Set up update callback
    let mut processor = state.file_processor.write().await;
    let mut update_rx = processor.set_update_callback();
    drop(processor);

    // Spawn task to forward updates to frontend
    let window_clone = window.clone();
    tokio::spawn(async move {
        while let Some(update) = update_rx.recv().await {
            let _ = window_clone.emit(
                "file-job-update",
                FileJobUpdateEvent {
                    id: update.id.to_string(),
                    status: update.status.to_string(),
                    progress: update.progress,
                },
            );
        }
    });

    // Process all pending jobs
    // The file processor handles audio loading internally
    // For actual transcription, we'd need to integrate more deeply
    let processor = state.file_processor.read().await;
    processor.process_all().await.map_err(|e| e.to_string())?;

    Ok(())
}

/// Remove a file job from the queue
#[tauri::command]
pub async fn remove_file_job(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    let processor = state.file_processor.read().await;
    processor.remove_job(uuid).await;
    Ok(())
}

/// Clear all completed jobs
#[tauri::command]
pub async fn clear_completed_jobs(state: State<'_, AppState>) -> Result<(), String> {
    use mindtype_core::FileStatus;

    let processor = state.file_processor.read().await;
    let jobs = processor.get_jobs().await;

    for job in jobs {
        if matches!(job.status, FileStatus::Completed | FileStatus::Failed(_)) {
            processor.remove_job(job.id).await;
        }
    }

    Ok(())
}

/// Export transcript to file
#[tauri::command]
pub async fn export_transcript(
    id: String,
    format: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    let processor = state.file_processor.read().await;

    let job = processor
        .get_job(uuid)
        .await
        .ok_or_else(|| "Job not found".to_string())?;

    let transcription = job
        .transcription
        .ok_or_else(|| "No transcription available".to_string())?;

    // Generate output path
    let base_name = job.path.file_stem().unwrap_or_default().to_string_lossy();
    let output_dir = job.path.parent().unwrap_or(std::path::Path::new("."));

    let (extension, content) = match format.as_str() {
        "txt" => ("txt", transcription.clone()),
        "md" => {
            let md = format!(
                "# Transcription: {}\n\n{}\n\n---\n*Generated by MindType*\n",
                job.filename, transcription
            );
            ("md", md)
        }
        "json" => {
            let json = serde_json::json!({
                "filename": job.filename,
                "transcription": transcription,
                "summary": job.summary,
                "created_at": job.created_at.to_rfc3339(),
                "completed_at": job.completed_at.map(|t| t.to_rfc3339()),
            });
            ("json", serde_json::to_string_pretty(&json).unwrap())
        }
        _ => return Err("Unsupported format".to_string()),
    };

    let output_path = output_dir.join(format!("{}_transcript.{}", base_name, extension));
    std::fs::write(&output_path, content).map_err(|e| e.to_string())?;

    Ok(output_path.to_string_lossy().to_string())
}

/// Open file dialog to select audio/video files
#[tauri::command]
pub async fn open_file_dialog() -> Result<Option<Vec<String>>, String> {
    // Note: File dialog is handled via Tauri's drag-drop or plugin-dialog
    // For now, return empty - files can be added via drag-drop
    Ok(None)
}

// =====================
// LLM Configuration Commands
// =====================

/// Available LLM providers for frontend dropdown
#[derive(Debug, Clone, Serialize)]
pub struct LlmProviderInfo {
    pub id: String,
    pub name: String,
    pub requires_api_key: bool,
    pub models: Vec<String>,
}

/// Get list of available LLM providers
#[tauri::command]
pub async fn get_llm_providers() -> Result<Vec<LlmProviderInfo>, String> {
    Ok(vec![
        LlmProviderInfo {
            id: "mindtype_cloud".to_string(),
            name: "MindType Cloud".to_string(),
            requires_api_key: false,
            models: vec!["default".to_string()],
        },
        LlmProviderInfo {
            id: "openai".to_string(),
            name: "OpenAI".to_string(),
            requires_api_key: true,
            models: vec![
                "gpt-4o".to_string(),
                "gpt-4o-mini".to_string(),
                "gpt-4-turbo".to_string(),
                "gpt-3.5-turbo".to_string(),
            ],
        },
        LlmProviderInfo {
            id: "anthropic".to_string(),
            name: "Anthropic".to_string(),
            requires_api_key: true,
            models: vec![
                "claude-3-5-sonnet-latest".to_string(),
                "claude-3-5-haiku-latest".to_string(),
                "claude-3-opus-latest".to_string(),
            ],
        },
        LlmProviderInfo {
            id: "gemini".to_string(),
            name: "Google Gemini".to_string(),
            requires_api_key: true,
            models: vec![
                "gemini-1.5-pro".to_string(),
                "gemini-1.5-flash".to_string(),
                "gemini-2.0-flash".to_string(),
            ],
        },
        LlmProviderInfo {
            id: "openrouter".to_string(),
            name: "OpenRouter".to_string(),
            requires_api_key: true,
            models: vec![
                "anthropic/claude-3.5-sonnet".to_string(),
                "openai/gpt-4o".to_string(),
                "google/gemini-pro-1.5".to_string(),
                "meta-llama/llama-3.1-70b-instruct".to_string(),
            ],
        },
        LlmProviderInfo {
            id: "ollama".to_string(),
            name: "Ollama (Local)".to_string(),
            requires_api_key: false,
            models: vec![
                "llama3.1".to_string(),
                "llama3.2".to_string(),
                "mistral".to_string(),
                "qwen2.5".to_string(),
            ],
        },
    ])
}

/// Get current LLM configuration
#[tauri::command]
pub async fn get_llm_config(state: State<'_, AppState>) -> Result<LlmConfig, String> {
    let settings = state.settings.read().await;
    Ok(settings.llm.clone())
}

/// Update LLM configuration
#[tauri::command]
pub async fn set_llm_config(config: LlmConfig, state: State<'_, AppState>) -> Result<(), String> {
    {
        let mut settings = state.settings.write().await;
        settings.llm = config;
    }
    state.save_settings().await.map_err(|e| e.to_string())?;
    Ok(())
}

/// Test LLM provider connection
#[tauri::command]
pub async fn test_llm_connection(config: LlmConfig) -> Result<bool, String> {
    // For now, just validate that the config has required fields
    match &config {
        LlmConfig::MindTypeCloud => Ok(true),
        LlmConfig::OpenAi { api_key, .. } => Ok(!api_key.is_empty()),
        LlmConfig::Anthropic { api_key, .. } => Ok(!api_key.is_empty()),
        LlmConfig::Gemini { api_key, .. } => Ok(!api_key.is_empty()),
        LlmConfig::OpenRouter { api_key, .. } => Ok(!api_key.is_empty()),
        LlmConfig::Yandex { api_key, folder_id, .. } => {
            Ok(!api_key.is_empty() && !folder_id.is_empty())
        }
        LlmConfig::Ollama { base_url, .. } => {
            // Check if Ollama is reachable
            let url = base_url
                .clone()
                .unwrap_or_else(|| "http://localhost:11434".to_string());
            let client = reqwest::Client::new();
            match client.get(&format!("{}/api/tags", url)).send().await {
                Ok(resp) => Ok(resp.status().is_success()),
                Err(_) => Ok(false),
            }
        }
    }
}

// =====================
// Summarization Commands
// =====================

/// Summary preset types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SummaryPreset {
    /// Generic meeting notes
    Meeting,
    /// Student lecture notes
    Student,
    /// Project manager notes
    ProjectManager,
    /// Custom prompt
    Custom { prompt: String },
}

impl SummaryPreset {
    fn to_prompt(&self) -> String {
        match self {
            SummaryPreset::Meeting => {
                "Summarize this meeting transcription into well-structured notes. Include:\n\
                 - Key discussion points\n\
                 - Decisions made\n\
                 - Action items with assigned persons if mentioned\n\
                 - Next steps\n\
                 Use bullet points for clarity.".to_string()
            }
            SummaryPreset::Student => {
                "Convert this lecture transcription into comprehensive study notes. Include:\n\
                 - Main concepts and definitions\n\
                 - Key facts and examples\n\
                 - Important points to remember\n\
                 - Questions for review\n\
                 Use clear headings and bullet points.".to_string()
            }
            SummaryPreset::ProjectManager => {
                "Summarize this transcription for project management purposes. Include:\n\
                 - Project status updates\n\
                 - Blockers and risks identified\n\
                 - Resource requirements\n\
                 - Timeline updates\n\
                 - Action items with owners and deadlines\n\
                 Format for easy stakeholder review.".to_string()
            }
            SummaryPreset::Custom { prompt } => prompt.clone(),
        }
    }
}

/// Create a provider from LlmConfig
fn create_provider(config: &LlmConfig) -> Result<Box<dyn mindtype_llm::SummaryProvider>, String> {
    use mindtype_llm::*;

    match config {
        LlmConfig::MindTypeCloud => {
            // MindType Cloud uses credits - for now return error as it needs special handling
            Err("MindType Cloud summarization requires license activation".to_string())
        }
        LlmConfig::OpenAi { api_key, model } => {
            let provider = OpenAiProvider::new(api_key, model.as_deref());
            Ok(Box::new(provider))
        }
        LlmConfig::Anthropic { api_key, model } => {
            let provider = AnthropicProvider::new(api_key, model.as_deref());
            Ok(Box::new(provider))
        }
        LlmConfig::Gemini { api_key, model } => {
            let provider = GeminiProvider::new(api_key, model.as_deref());
            Ok(Box::new(provider))
        }
        LlmConfig::OpenRouter { api_key, model } => {
            let provider = OpenRouterProvider::new(api_key, model.as_deref());
            Ok(Box::new(provider))
        }
        LlmConfig::Yandex { api_key, folder_id, model } => {
            let provider = YandexProvider::new(api_key, folder_id, model.as_deref());
            Ok(Box::new(provider))
        }
        LlmConfig::Ollama { base_url, model } => {
            let provider = OllamaProvider::new(base_url.as_deref(), model.as_deref());
            Ok(Box::new(provider))
        }
    }
}

/// Summarize text using configured LLM provider
#[tauri::command]
pub async fn summarize_text(
    text: String,
    preset: Option<SummaryPreset>,
    language: Option<String>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let settings = state.settings.read().await;
    let provider = create_provider(&settings.llm)?;

    let prompt = preset.map(|p| p.to_prompt());

    let request = mindtype_llm::SummaryRequest::new(&text)
        .with_max_tokens(2048);

    let request = if let Some(p) = prompt {
        request.with_prompt(p)
    } else {
        request
    };

    let request = if let Some(l) = language {
        request.with_language(l)
    } else {
        request
    };

    let response = provider.summarize(request).await.map_err(|e| e.to_string())?;

    Ok(response.summary)
}

/// Summarize a file transcription by job ID
#[tauri::command]
pub async fn summarize_file_job(
    id: String,
    preset: Option<SummaryPreset>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|e| e.to_string())?;

    // Get the transcription
    let processor = state.file_processor.read().await;
    let job = processor
        .get_job(uuid)
        .await
        .ok_or_else(|| "Job not found".to_string())?;

    let transcription = job
        .transcription
        .ok_or_else(|| "No transcription available".to_string())?;
    drop(processor);

    // Summarize using the configured provider
    let settings = state.settings.read().await;
    let provider = create_provider(&settings.llm)?;
    drop(settings);

    let prompt = preset.map(|p| p.to_prompt());

    let request = mindtype_llm::SummaryRequest::new(&transcription)
        .with_max_tokens(2048);

    let request = if let Some(p) = prompt {
        request.with_prompt(p)
    } else {
        request
    };

    let response = provider.summarize(request).await.map_err(|e| e.to_string())?;

    // Update the job with the summary
    let processor = state.file_processor.read().await;
    processor.update_summary(uuid, response.summary.clone()).await;

    Ok(response.summary)
}

/// Get available summary presets
#[tauri::command]
pub async fn get_summary_presets() -> Result<Vec<serde_json::Value>, String> {
    Ok(vec![
        serde_json::json!({
            "id": "meeting",
            "name": "Meeting Notes",
            "description": "Summarize meetings with action items and decisions"
        }),
        serde_json::json!({
            "id": "student",
            "name": "Study Notes",
            "description": "Convert lectures into study-friendly notes"
        }),
        serde_json::json!({
            "id": "project_manager",
            "name": "Project Manager",
            "description": "Extract project updates, risks, and action items"
        }),
    ])
}

// =====================
// Update Commands
// =====================

/// Update check result
#[derive(Debug, Clone, Serialize)]
pub struct UpdateInfo {
    pub available: bool,
    pub version: Option<String>,
    pub notes: Option<String>,
    pub date: Option<String>,
}

/// Check for available updates
#[tauri::command]
pub async fn check_for_updates(app: tauri::AppHandle) -> Result<UpdateInfo, String> {
    use tauri_plugin_updater::UpdaterExt;

    let updater = app.updater().map_err(|e| e.to_string())?;

    match updater.check().await {
        Ok(Some(update)) => Ok(UpdateInfo {
            available: true,
            version: Some(update.version.clone()),
            notes: update.body.clone(),
            date: update.date.map(|d| d.to_string()),
        }),
        Ok(None) => Ok(UpdateInfo {
            available: false,
            version: None,
            notes: None,
            date: None,
        }),
        Err(e) => Err(e.to_string()),
    }
}

/// Download and install update
#[tauri::command]
pub async fn install_update(app: tauri::AppHandle, window: Window) -> Result<(), String> {
    use tauri_plugin_updater::UpdaterExt;

    let updater = app.updater().map_err(|e| e.to_string())?;

    let update = updater
        .check()
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "No update available".to_string())?;

    // Emit download progress events
    let window_clone = window.clone();
    let mut downloaded = 0u64;

    update
        .download_and_install(
            |chunk_length, content_length| {
                downloaded += chunk_length as u64;
                let progress = if let Some(total) = content_length {
                    (downloaded as f64 / total as f64 * 100.0) as u8
                } else {
                    0
                };
                let _ = window_clone.emit("update-progress", progress);
            },
            || {
                // Called when download completes, before install
                let _ = window_clone.emit("update-installing", ());
            },
        )
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Get current app version
#[tauri::command]
pub fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

// =====================
// Crash Report Commands
// =====================

/// Manual crash report payload from frontend
#[derive(Debug, Deserialize)]
pub struct ManualCrashReport {
    pub error_type: String,
    pub error_message: String,
    pub stack_trace: Option<String>,
    pub user_description: Option<String>,
}

/// Submit a crash report from the frontend (for JS errors)
#[tauri::command]
pub async fn submit_crash_report(report: ManualCrashReport) -> Result<(), String> {
    add_breadcrumb(format!("Submitting crash report: {}", report.error_type));

    let crash_report = CrashReport {
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        platform: std::env::consts::OS.to_string(),
        rust_version: option_env!("CARGO_PKG_RUST_VERSION")
            .unwrap_or("unknown")
            .to_string(),
        os_version: os_info::get().to_string(),
        error_type: report.error_type,
        error_message: crate::crash_reporter::sanitize_text(&report.error_message)
            .chars()
            .take(500)
            .collect(),
        backtrace: report
            .stack_trace
            .map(|s| crate::crash_reporter::sanitize_text(&s))
            .unwrap_or_default(),
        breadcrumbs: crate::crash_reporter::get_breadcrumbs()
            .iter()
            .rev()
            .take(20)
            .map(|b| {
                crate::crash_reporter::sanitize_text(&format!("[{}] {}", b.timestamp, b.message))
            })
            .collect(),
        device_id: mindtype_platform::Platform::new()
            .ok()
            .and_then(|p| p.get_device_id().ok()),
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    crate::crash_reporter::send_crash_report_to_server(crash_report)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Get the path to crash reports directory
#[tauri::command]
pub fn get_crash_reports_dir() -> String {
    crate::crash_reporter::get_crashes_dir()
        .to_string_lossy()
        .to_string()
}
