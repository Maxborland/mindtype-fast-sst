//! MindType Voice Assistant
//!
//! A voice-enabled AI assistant with conversation history.

mod conversation;
mod error;
mod state;

pub use conversation::{Conversation, Message, Role};
pub use error::AssistantError;
pub use state::{AssistantState, StateEvent};

use mindtype_llm::{SummaryProvider, SummaryRequest};
use mindtype_tts::TtsEngine;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Voice assistant configuration
#[derive(Debug, Clone)]
pub struct AssistantConfig {
    /// System prompt for the assistant
    pub system_prompt: String,
    /// TTS voice to use
    pub voice: String,
    /// Enable TTS for responses
    pub enable_tts: bool,
    /// Maximum conversation history to keep
    pub max_history: usize,
}

impl Default for AssistantConfig {
    fn default() -> Self {
        Self {
            system_prompt: "You are a helpful voice assistant. Keep responses concise and conversational. Respond in the same language as the user's query.".to_string(),
            voice: mindtype_tts::voices::EN_US_ARIA.to_string(),
            enable_tts: true,
            max_history: 20,
        }
    }
}

/// Voice assistant that integrates LLM and TTS
pub struct VoiceAssistant {
    config: AssistantConfig,
    conversation: Arc<RwLock<Conversation>>,
    tts: TtsEngine,
    state: Arc<RwLock<AssistantState>>,
}

impl VoiceAssistant {
    /// Create a new voice assistant
    pub fn new(config: AssistantConfig) -> Self {
        let mut conversation = Conversation::new();
        conversation.set_system_prompt(&config.system_prompt);

        Self {
            config,
            conversation: Arc::new(RwLock::new(conversation)),
            tts: TtsEngine::new(),
            state: Arc::new(RwLock::new(AssistantState::Idle)),
        }
    }

    /// Create with default configuration
    pub fn default_assistant() -> Self {
        Self::new(AssistantConfig::default())
    }

    /// Get current state
    pub async fn state(&self) -> AssistantState {
        *self.state.read().await
    }

    /// Set TTS voice
    pub fn set_voice(&mut self, voice: &str) {
        self.config.voice = voice.to_string();
    }

    /// Enable or disable TTS
    pub fn set_tts_enabled(&mut self, enabled: bool) {
        self.config.enable_tts = enabled;
    }

    /// Process user input and generate response
    pub async fn process<P: SummaryProvider>(
        &self,
        user_input: &str,
        provider: &P,
    ) -> Result<String, AssistantError> {
        info!("Processing user input: {}", user_input);

        // Update state to processing
        *self.state.write().await = AssistantState::Processing;

        // Add user message to conversation
        {
            let mut conv = self.conversation.write().await;
            conv.add_message(Role::User, user_input);
        }

        // Build prompt with conversation history
        let prompt = {
            let conv = self.conversation.read().await;
            conv.to_prompt()
        };

        debug!("Sending prompt to LLM");

        // Get response from LLM
        let request = SummaryRequest::new(&prompt);
        let response = provider.summarize(request).await?;

        let assistant_response = response.summary;

        // Add assistant response to conversation
        {
            let mut conv = self.conversation.write().await;
            conv.add_message(Role::Assistant, &assistant_response);

            // Trim history if needed
            if conv.len() > self.config.max_history {
                conv.trim_to(self.config.max_history);
            }
        }

        // Update state to speaking
        *self.state.write().await = AssistantState::Speaking;

        // Speak response if TTS is enabled
        if self.config.enable_tts {
            debug!("Speaking response via TTS");
            if let Err(e) = self.tts.speak(&assistant_response, Some(&self.config.voice)).await {
                tracing::warn!("TTS failed: {}", e);
                // Don't fail the whole operation if TTS fails
            }
        }

        // Update state back to idle
        *self.state.write().await = AssistantState::Idle;

        Ok(assistant_response)
    }

    /// Stop any current TTS playback
    pub fn stop_speaking(&self) {
        self.tts.stop();
    }

    /// Check if assistant is currently speaking
    pub fn is_speaking(&self) -> bool {
        self.tts.is_speaking()
    }

    /// Get conversation history
    pub async fn get_history(&self) -> Vec<Message> {
        self.conversation.read().await.messages().clone()
    }

    /// Clear conversation history
    pub async fn clear_history(&self) {
        let mut conv = self.conversation.write().await;
        conv.clear();
        conv.set_system_prompt(&self.config.system_prompt);
    }

    /// Get the TTS engine for direct access
    pub fn tts(&self) -> &TtsEngine {
        &self.tts
    }
}
