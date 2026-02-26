<div align="center">

# MindType (Rust Edition)

**Next-generation voice-to-text engine built in Rust for maximum performance.**

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

</div>

---

## Overview

Rust rewrite of [MindType](https://github.com/Maxborland/mindtype-app) — a desktop voice-to-text application with AI-powered summaries. Built as a Cargo workspace with modular crates for maximum reusability and performance.

## Architecture

```
crates/
├── mindtype-app          # Application orchestration
├── mindtype-core         # Core types and utilities
├── mindtype-platform     # Platform-specific code (Windows, macOS, Linux)
├── mindtype-licensing    # License validation
├── mindtype-llm          # LLM provider integrations
├── mindtype-whisper      # Whisper speech recognition
├── mindtype-tauri        # Tauri desktop UI
├── mindtype-text-processor # Text processing pipeline
├── mindtype-tts          # Text-to-speech engine
└── mindtype-assistant    # AI assistant integration
```

## Key Features

- **Whisper STT** — Local speech recognition, no cloud dependency
- **Multi-LLM** — OpenAI, Anthropic, Ollama, OpenRouter support
- **TTS** — Text-to-speech for accessibility
- **Tauri UI** — Native desktop app with web technologies
- **Cross-platform** — Windows, macOS, Linux

## Development

```bash
cargo build
cargo test
cargo run -p mindtype-tauri
```

## Related

- [mindtype-app](https://github.com/Maxborland/mindtype-app) — Python/PyQt6 version (current release)
- [mindtype-site](https://github.com/Maxborland/mindtype-site) — Website & license management

## License

MIT License — see [LICENSE](LICENSE) for details.
