# Functionality

## Features

### 1. Interactive Installer (`clawra install`)

7-step guided setup that:
1. Checks ZeroClaw CLI is installed
2. Prompts for OpenRouter API key (opens browser)
3. Installs skill files to `~/.zeroclaw/skills/clawra-selfie/`
4. Merges config into `~/.zeroclaw/config.toml`
5. Creates `IDENTITY.md` for agent identity
6. Injects selfie persona into `SOUL.md`
7. Prints summary with usage examples

**Trigger**: `clawra install`
**Key files**: `src/install.rs`, `src/config.rs`

### 2. CLI Selfie Generation (`clawra selfie`)

Standalone image generation from the command line.

**Trigger**: `clawra selfie "<context>" "<channel>" [--mode auto|mirror|direct] [--caption text] [--format jpeg|png|webp]`
**Key files**: `src/selfie.rs`, `src/main.rs`

### 3. Skill-Based Selfie (via ZeroClaw Agent)

The ZeroClaw agent invokes the skill when users send trigger messages.

**Photo triggers**: selfie, foto, pic, photo, picture, "manda una foto", "send a pic", "what are you doing", "where are you"
**Video triggers**: video, reel, clip, "manda un video", "video selfie"
**Voice triggers**: voice, audio, "manda un audio", "nota de voz", "habla"

**Key files**: `SKILL.md`, `scripts/clawra-selfie.sh`

### 4. Selfie Mode Detection

Automatically selects between mirror and direct selfie modes based on keywords in the user's message.

| Mode | Description | Keywords |
|------|-------------|----------|
| Mirror | Full-body, outfit showcase | wearing, outfit, clothes, dress, suit, fashion, full-body, mirror |
| Direct | Close-up, location shot | cafe, restaurant, beach, park, city, close-up, portrait, face, eyes, smile |

**Key files**: `src/selfie.rs` (`detect_mode` function)

### 5. Base64 Image Processing

Decodes base64 data URIs from the OpenRouter API response without external dependencies. Supports PNG, WebP, and JPEG formats.

**Key files**: `src/selfie.rs` (`save_base64_image`, `base64_decode` functions)

### 6. Telegram Direct Upload

Sends generated images directly via Telegram Bot API multipart upload, bypassing ZeroClaw's message system for reliable media delivery.

**Key files**: `src/selfie.rs` (curl-based `sendPhoto`), `scripts/clawra-selfie.sh`

### 7. Config Management

Reads and writes TOML configuration for ZeroClaw integration. Merges API keys, skill entries, and extra directory paths.

**Key files**: `src/config.rs` (`merge_skill_config`)

## Supported Image Models

| Model ID | Provider | Cost | Notes |
|----------|----------|------|-------|
| `google/gemini-2.5-flash-image` | Google | ~$0.04/selfie | Default |
| `google/gemini-3-pro-image-preview` | Google | ~$0.08/selfie | Higher quality |
| `openai/gpt-5-image-mini` | OpenAI | ~$0.10/selfie | Alternative |
| `openai/gpt-5-image` | OpenAI | ~$0.40/selfie | Best quality |
