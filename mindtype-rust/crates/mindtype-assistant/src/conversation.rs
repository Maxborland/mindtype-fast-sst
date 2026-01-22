//! Conversation history management

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Message role in the conversation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
}

/// A single message in the conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Uuid,
    pub role: Role,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

impl Message {
    /// Create a new message
    pub fn new(role: Role, content: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            role,
            content: content.into(),
            timestamp: Utc::now(),
        }
    }
}

/// Conversation history
#[derive(Debug, Clone, Default)]
pub struct Conversation {
    messages: Vec<Message>,
    system_prompt: Option<String>,
}

impl Conversation {
    /// Create a new empty conversation
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the system prompt
    pub fn set_system_prompt(&mut self, prompt: &str) {
        self.system_prompt = Some(prompt.to_string());
    }

    /// Add a message to the conversation
    pub fn add_message(&mut self, role: Role, content: &str) {
        self.messages.push(Message::new(role, content));
    }

    /// Get all messages
    pub fn messages(&self) -> &Vec<Message> {
        &self.messages
    }

    /// Get message count (excluding system)
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    /// Check if conversation is empty
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    /// Clear all messages (keeps system prompt)
    pub fn clear(&mut self) {
        self.messages.clear();
    }

    /// Trim conversation to keep only the last N messages
    pub fn trim_to(&mut self, max_messages: usize) {
        if self.messages.len() > max_messages {
            let to_remove = self.messages.len() - max_messages;
            self.messages.drain(0..to_remove);
        }
    }

    /// Convert conversation to a prompt string for the LLM
    pub fn to_prompt(&self) -> String {
        let mut prompt = String::new();

        // Add system prompt if set
        if let Some(ref system) = self.system_prompt {
            prompt.push_str("System: ");
            prompt.push_str(system);
            prompt.push_str("\n\n");
        }

        // Add conversation history
        for message in &self.messages {
            let role_str = match message.role {
                Role::System => "System",
                Role::User => "User",
                Role::Assistant => "Assistant",
            };
            prompt.push_str(role_str);
            prompt.push_str(": ");
            prompt.push_str(&message.content);
            prompt.push_str("\n\n");
        }

        // Add prompt for assistant response
        prompt.push_str("Assistant: ");

        prompt
    }

    /// Get the last message
    pub fn last_message(&self) -> Option<&Message> {
        self.messages.last()
    }

    /// Get the last user message
    pub fn last_user_message(&self) -> Option<&Message> {
        self.messages.iter().rev().find(|m| m.role == Role::User)
    }

    /// Get the last assistant message
    pub fn last_assistant_message(&self) -> Option<&Message> {
        self.messages
            .iter()
            .rev()
            .find(|m| m.role == Role::Assistant)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conversation() {
        let mut conv = Conversation::new();
        conv.set_system_prompt("You are a helpful assistant.");
        conv.add_message(Role::User, "Hello!");
        conv.add_message(Role::Assistant, "Hi there! How can I help?");
        conv.add_message(Role::User, "What's the weather?");

        assert_eq!(conv.len(), 3);
        assert_eq!(conv.last_user_message().unwrap().content, "What's the weather?");

        let prompt = conv.to_prompt();
        assert!(prompt.contains("System: You are a helpful assistant."));
        assert!(prompt.contains("User: Hello!"));
        assert!(prompt.contains("Assistant: Hi there!"));
    }

    #[test]
    fn test_trim() {
        let mut conv = Conversation::new();
        for i in 0..10 {
            conv.add_message(Role::User, &format!("Message {}", i));
        }

        assert_eq!(conv.len(), 10);
        conv.trim_to(5);
        assert_eq!(conv.len(), 5);
        assert_eq!(conv.messages[0].content, "Message 5");
    }
}
