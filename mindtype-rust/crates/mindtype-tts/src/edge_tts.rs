//! Microsoft Edge TTS implementation
//!
//! Uses the Edge browser's TTS WebSocket API to synthesize speech.

use crate::error::TtsError;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error};

const VOICES_URL: &str = "https://speech.platform.bing.com/consumer/speech/synthesize/readaloud/voices/list?trustedclienttoken=6A5AA1D4EAFF4E9FB37E23D68491D6F4";
const WSS_URL: &str = "wss://speech.platform.bing.com/consumer/speech/synthesize/readaloud/edge/v1?TrustedClientToken=6A5AA1D4EAFF4E9FB37E23D68491D6F4";

/// Voice information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Voice {
    pub name: String,
    pub short_name: String,
    pub gender: String,
    pub locale: String,
    #[serde(default)]
    pub suggested_codec: String,
    #[serde(default)]
    pub friendly_name: String,
    #[serde(default)]
    pub status: String,
}

impl Voice {
    /// Get the language code (e.g., "en" from "en-US")
    pub fn language(&self) -> &str {
        self.locale.split('-').next().unwrap_or(&self.locale)
    }
}

/// Edge TTS synthesizer
#[derive(Clone)]
pub struct EdgeTts {
    rate: i32,
    pitch: i32,
    default_voice: String,
}

impl EdgeTts {
    /// Create a new Edge TTS instance
    pub fn new() -> Self {
        Self {
            rate: 0,
            pitch: 0,
            default_voice: "en-US-AriaNeural".to_string(),
        }
    }

    /// Set speech rate (-50 to +100)
    pub fn set_rate(&mut self, rate: i32) {
        self.rate = rate.clamp(-50, 100);
    }

    /// Set speech pitch (-50 to +50)
    pub fn set_pitch(&mut self, pitch: i32) {
        self.pitch = pitch.clamp(-50, 50);
    }

    /// Set default voice
    pub fn set_default_voice(&mut self, voice: &str) {
        self.default_voice = voice.to_string();
    }

    /// List available voices
    pub async fn list_voices(&self, language: Option<&str>) -> Result<Vec<Voice>, TtsError> {
        let client = reqwest::Client::new();
        let response = client
            .get(VOICES_URL)
            .header("Accept", "application/json")
            .send()
            .await?;

        let voices: Vec<Voice> = response.json().await?;

        // Filter by language if specified
        if let Some(lang) = language {
            let lang_lower = lang.to_lowercase();
            Ok(voices
                .into_iter()
                .filter(|v| {
                    v.locale.to_lowercase().starts_with(&lang_lower)
                        || v.language().to_lowercase() == lang_lower
                })
                .collect())
        } else {
            Ok(voices)
        }
    }

    /// Synthesize text to MP3 audio data
    pub async fn synthesize(
        &self,
        text: &str,
        voice: Option<&str>,
    ) -> Result<Vec<u8>, TtsError> {
        let voice = voice.unwrap_or(&self.default_voice);

        debug!("Synthesizing with voice: {}", voice);

        // Connect to WebSocket
        let (ws_stream, _) = connect_async(WSS_URL)
            .await
            .map_err(|e| TtsError::WebSocketError(e.to_string()))?;

        let (mut write, mut read) = ws_stream.split();

        // Generate unique request ID
        let request_id = uuid::Uuid::new_v4().to_string().replace("-", "");

        // Send configuration message
        let config_msg = format!(
            "X-Timestamp:{}\r\nContent-Type:application/json; charset=utf-8\r\nPath:speech.config\r\n\r\n{{\"context\":{{\"synthesis\":{{\"audio\":{{\"metadataoptions\":{{\"sentenceBoundaryEnabled\":false,\"wordBoundaryEnabled\":false}},\"outputFormat\":\"audio-24khz-48kbitrate-mono-mp3\"}}}}}}}}",
            Self::timestamp()
        );
        write
            .send(Message::Text(config_msg))
            .await
            .map_err(|e| TtsError::WebSocketError(e.to_string()))?;

        // Build SSML
        let ssml = self.build_ssml(text, voice);

        // Send SSML message
        let ssml_msg = format!(
            "X-RequestId:{}\r\nContent-Type:application/ssml+xml\r\nX-Timestamp:{}\r\nPath:ssml\r\n\r\n{}",
            request_id,
            Self::timestamp(),
            ssml
        );
        write
            .send(Message::Text(ssml_msg))
            .await
            .map_err(|e| TtsError::WebSocketError(e.to_string()))?;

        // Collect audio data
        let mut audio_data = Vec::new();

        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Binary(data)) => {
                    // Binary messages contain audio data with header
                    // Header format: "X-RequestId:...\r\nX-Timestamp:...\r\nPath:audio\r\n"
                    // followed by the actual audio bytes after "Path:audio\r\n"
                    if let Some(audio_start) = Self::find_audio_start(&data) {
                        audio_data.extend_from_slice(&data[audio_start..]);
                    }
                }
                Ok(Message::Text(text)) => {
                    // Text messages are metadata/status
                    if text.contains("Path:turn.end") {
                        debug!("TTS synthesis complete");
                        break;
                    }
                }
                Ok(Message::Close(_)) => {
                    break;
                }
                Err(e) => {
                    error!("WebSocket error: {}", e);
                    return Err(TtsError::WebSocketError(e.to_string()));
                }
                _ => {}
            }
        }

        if audio_data.is_empty() {
            return Err(TtsError::NoAudioData);
        }

        Ok(audio_data)
    }

    fn build_ssml(&self, text: &str, voice: &str) -> String {
        let rate_str = if self.rate >= 0 {
            format!("+{}%", self.rate)
        } else {
            format!("{}%", self.rate)
        };

        let pitch_str = if self.pitch >= 0 {
            format!("+{}Hz", self.pitch)
        } else {
            format!("{}Hz", self.pitch)
        };

        // Escape XML special characters
        let escaped_text = text
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&apos;");

        format!(
            r#"<speak version="1.0" xmlns="http://www.w3.org/2001/10/synthesis" xmlns:mstts="https://www.w3.org/2001/mstts" xml:lang="en-US">
<voice name="{}">
<prosody rate="{}" pitch="{}">
{}
</prosody>
</voice>
</speak>"#,
            voice, rate_str, pitch_str, escaped_text
        )
    }

    fn timestamp() -> String {
        chrono::Utc::now().format("%a %b %d %Y %H:%M:%S GMT+0000 (Coordinated Universal Time)").to_string()
    }

    fn find_audio_start(data: &[u8]) -> Option<usize> {
        // Look for "Path:audio\r\n" in the header
        let pattern = b"Path:audio\r\n";
        data.windows(pattern.len())
            .position(|window| window == pattern)
            .map(|pos| pos + pattern.len())
    }
}

impl Default for EdgeTts {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires network
    async fn test_list_voices() {
        let tts = EdgeTts::new();
        let voices = tts.list_voices(Some("en")).await.unwrap();
        assert!(!voices.is_empty());
        assert!(voices.iter().any(|v| v.locale.starts_with("en-")));
    }

    #[tokio::test]
    #[ignore] // Requires network
    async fn test_synthesize() {
        let tts = EdgeTts::new();
        let audio = tts
            .synthesize("Hello, world!", Some("en-US-AriaNeural"))
            .await
            .unwrap();
        assert!(!audio.is_empty());
        // Check MP3 magic bytes
        assert_eq!(&audio[..2], b"\xff\xfb".as_slice().get(..2).unwrap_or(&[0, 0]));
    }
}
