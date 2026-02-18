# Changelog

All notable changes to Clawra ZeroClaw are documented in this file.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [1.0.0] - 2026-02-18

### Added
- Full Rust rewrite of Clawra selfie skill (previously Node.js)
- OpenRouter API integration for image generation (Gemini Flash Image default)
- Interactive `clawra install` wizard with 7-step guided setup
- CLI `clawra selfie` command for standalone image generation
- Telegram Bot API direct photo upload (multipart/form-data via curl)
- Automatic selfie mode detection (mirror vs direct) from keywords
- TOML config read/write for ZeroClaw integration
- SOUL.md persona injection during install
- IDENTITY.md agent identity template
- Standalone bash script (`clawra-selfie.sh`) for skill-based execution
- SKILL.md with trigger keywords for photos, videos, and voice notes
- Soul injection template (`templates/soul-injection.md`)
- Reference image support for consistent visual identity
- Support for multiple OpenRouter image models (Gemini, GPT-5 Image)
- ZeroClaw CLI fallback when Telegram env vars are not set
- Documentation: README, ARCHITECTURE, FUNCTIONALITY, RUNBOOK, CONFIGURATION

### Changed
- Migrated from Node.js (`npx`) to Rust (`cargo install`) installer
- Migrated from JSON config (`openclaw.json`) to TOML (`config.toml`)
- Migrated from fal.ai/Grok Imagine to OpenRouter/Gemini Flash Image
- Binary size reduced from ~28MB (Node.js) to 1.7MB (Rust)
- Startup time reduced from >500ms to <10ms

## [0.1.0] - 2026-02-17

### Added
- Initial port of Clawra selfie skill from OpenClaw to ZeroClaw
- Basic Node.js-based selfie generation (pre-Rust rewrite)
