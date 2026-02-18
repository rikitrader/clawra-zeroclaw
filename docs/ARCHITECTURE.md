# Architecture

## System Overview

Clawra ZeroClaw is a standalone Rust binary that integrates with the ZeroClaw agent runtime as a skill. It generates selfie images via the OpenRouter API and delivers them through Telegram's Bot API.

```
User Message (Telegram/Discord/etc.)
        │
        ▼
┌─────────────────────┐
│  ZeroClaw Agent      │ ← SOUL.md persona + SKILL.md triggers
│  (Orchestration)     │
└────────┬────────────┘
         │ shell tool call
         ▼
┌─────────────────────┐
│  clawra-selfie.sh   │ ← Bash wrapper (loads env from config.toml)
│  (Script Layer)      │
└────────┬────────────┘
         │
         ▼
┌─────────────────────┐     ┌─────────────────────┐
│  OpenRouter API      │────▶│  Image Model         │
│  /chat/completions   │     │  (Gemini Flash Image)│
└────────┬────────────┘     └─────────────────────┘
         │ base64 PNG
         ▼
┌─────────────────────┐
│  Telegram Bot API    │
│  /sendPhoto          │ ← multipart file upload via curl
└────────┬────────────┘
         │
         ▼
     User receives selfie
```

## Components

### 1. Rust Binary (`clawra`)

| Module | File | Responsibility |
|--------|------|----------------|
| CLI entrypoint | `src/main.rs` | Command routing (`install`, `selfie`, `help`, `version`) |
| Installer | `src/install.rs` | 7-step interactive setup wizard |
| Selfie engine | `src/selfie.rs` | OpenRouter API call, base64 decode, Telegram upload |
| Config | `src/config.rs` | TOML read/write, skill config merge |

### 2. Bash Script (`scripts/clawra-selfie.sh`)

Standalone script used by ZeroClaw's shell tool. Loads environment variables from `config.toml`'s `[env]` section, calls OpenRouter via Python's `urllib`, decodes base64, and uploads via `curl`.

### 3. Skill Definition (`SKILL.md`)

Declares trigger keywords and execution commands for the ZeroClaw skill system. The agent reads this to know when and how to invoke selfie generation.

### 4. Soul Injection (`templates/soul-injection.md`)

Persona fragment injected into the agent's SOUL.md during installation. Gives the agent a visual identity and instructions for selfie behavior.

## Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `ureq` | 2.x | HTTP client (sync, lightweight, JSON feature) |
| `serde` | 1.x | Serialization/deserialization (derive feature) |
| `serde_json` | 1.x | JSON parsing for API responses |
| `toml` | 0.8.x | TOML config read/write |

## External Services

| Service | Endpoint | Purpose |
|---------|----------|---------|
| OpenRouter | `https://openrouter.ai/api/v1/chat/completions` | Image generation via LLM |
| Telegram | `https://api.telegram.org/bot{token}/sendPhoto` | Photo delivery |

## Security Considerations

- API keys stored in ZeroClaw's encrypted config (`config.toml` with `[secrets] encrypt = true`)
- No secrets hardcoded in source
- Telegram bot token scoped to specific `allowed_users` in ZeroClaw config
- Temp files written to `/tmp/` and not cleaned up automatically (ephemeral by OS)
