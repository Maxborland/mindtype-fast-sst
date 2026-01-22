//! MindType Text-to-Speech
//!
//! Provides text-to-speech functionality using Microsoft Edge TTS.

mod edge_tts;
mod error;
mod player;

pub use edge_tts::{EdgeTts, Voice};
pub use error::TtsError;
pub use player::AudioPlayer;

/// Default TTS engine using Edge TTS
#[derive(Clone)]
pub struct TtsEngine {
    edge_tts: EdgeTts,
    player: AudioPlayer,
}

impl TtsEngine {
    /// Create a new TTS engine
    pub fn new() -> Self {
        Self {
            edge_tts: EdgeTts::new(),
            player: AudioPlayer::new(),
        }
    }

    /// Set speech rate (-50 to +100, default 0)
    pub fn set_rate(&mut self, rate: i32) {
        self.edge_tts.set_rate(rate);
    }

    /// Set speech pitch (-50 to +50, default 0)
    pub fn set_pitch(&mut self, pitch: i32) {
        self.edge_tts.set_pitch(pitch);
    }

    /// Set default voice
    pub fn set_default_voice(&mut self, voice: &str) {
        self.edge_tts.set_default_voice(voice);
    }

    /// Speak text aloud
    pub async fn speak(&self, text: &str, voice: Option<&str>) -> Result<(), TtsError> {
        let audio_data = self.edge_tts.synthesize(text, voice).await?;
        self.player.play_mp3(&audio_data)?;
        Ok(())
    }

    /// Stop current playback
    pub fn stop(&self) {
        self.player.stop();
    }

    /// Check if currently speaking
    pub fn is_speaking(&self) -> bool {
        self.player.is_playing()
    }

    /// List available voices for a language
    pub async fn list_voices(&self, language: Option<&str>) -> Result<Vec<Voice>, TtsError> {
        self.edge_tts.list_voices(language).await
    }
}

impl Default for TtsEngine {
    fn default() -> Self {
        Self::new()
    }
}

// Common voice constants
pub mod voices {
    // English voices
    pub const EN_US_ARIA: &str = "en-US-AriaNeural";
    pub const EN_US_GUY: &str = "en-US-GuyNeural";
    pub const EN_US_JENNY: &str = "en-US-JennyNeural";
    pub const EN_GB_LIBBY: &str = "en-GB-LibbyNeural";
    pub const EN_GB_RYAN: &str = "en-GB-RyanNeural";

    // Russian voices
    pub const RU_RU_SVETLANA: &str = "ru-RU-SvetlanaNeural";
    pub const RU_RU_DMITRY: &str = "ru-RU-DmitryNeural";

    // Spanish voices
    pub const ES_ES_ELVIRA: &str = "es-ES-ElviraNeural";
    pub const ES_MX_DALIA: &str = "es-MX-DaliaNeural";

    // German voices
    pub const DE_DE_KATJA: &str = "de-DE-KatjaNeural";
    pub const DE_DE_CONRAD: &str = "de-DE-ConradNeural";

    // French voices
    pub const FR_FR_DENISE: &str = "fr-FR-DeniseNeural";
    pub const FR_FR_HENRI: &str = "fr-FR-HenriNeural";

    // Chinese voices
    pub const ZH_CN_XIAOXIAO: &str = "zh-CN-XiaoxiaoNeural";
    pub const ZH_CN_YUNYANG: &str = "zh-CN-YunyangNeural";
}
