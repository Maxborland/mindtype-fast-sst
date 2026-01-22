//! Assistant state machine

use serde::{Deserialize, Serialize};

/// Assistant state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AssistantState {
    /// Idle, waiting for user input
    Idle,
    /// Listening to user speech
    Listening,
    /// Transcribing audio
    Transcribing,
    /// Processing with LLM
    Processing,
    /// Speaking response
    Speaking,
    /// Error state
    Error,
}

impl Default for AssistantState {
    fn default() -> Self {
        Self::Idle
    }
}

impl std::fmt::Display for AssistantState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AssistantState::Idle => write!(f, "Idle"),
            AssistantState::Listening => write!(f, "Listening"),
            AssistantState::Transcribing => write!(f, "Transcribing"),
            AssistantState::Processing => write!(f, "Processing"),
            AssistantState::Speaking => write!(f, "Speaking"),
            AssistantState::Error => write!(f, "Error"),
        }
    }
}

/// State change event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateEvent {
    pub from: AssistantState,
    pub to: AssistantState,
    pub message: Option<String>,
}

impl StateEvent {
    pub fn new(from: AssistantState, to: AssistantState) -> Self {
        Self {
            from,
            to,
            message: None,
        }
    }

    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }
}
