# Clawra ZeroClaw

<img width="300" alt="image" src="https://github.com/user-attachments/assets/41512c51-e61d-4550-b461-eed06a1b0ec8" />

Clawra Selfie ported to [ZeroClaw](https://github.com/zeroclaw-labs/zeroclaw) — the lightweight Rust-based AI agent framework. Built in Rust to match ZeroClaw's own stack.

## Quick Start

```bash
cargo install clawra-zeroclaw
clawra install
```

This will:
1. Check ZeroClaw is installed
2. Guide you to get an OpenRouter API key
3. Install the skill to `~/.zeroclaw/skills/clawra-selfie/`
4. Configure ZeroClaw to use the skill (TOML config)
5. Add selfie capabilities to your agent's SOUL.md

## Generate Selfies from CLI

```bash
clawra selfie "wearing a cowboy hat" "#general"
clawra selfie "a cozy cafe" "#photography" --mode direct --caption "Vibes!"
```

## What It Does

Clawra Selfie enables your ZeroClaw agent to:
- **Generate selfies** using a consistent reference image
- **Send photos** across all messaging platforms (Discord, Telegram, WhatsApp, Slack, iMessage)
- **Respond visually** to "what are you doing?" and "send a pic" requests

### Selfie Modes

| Mode | Best For | Keywords |
|------|----------|----------|
| **Mirror** | Full-body shots, outfits | wearing, outfit, fashion |
| **Direct** | Close-ups, locations | cafe, beach, portrait, smile |

## Prerequisites

- [ZeroClaw](https://github.com/zeroclaw-labs/zeroclaw) installed and configured
- [OpenRouter](https://openrouter.ai) account and API key
- Rust toolchain (`rustup` / `cargo`)

## Manual Installation

### 1. Get API Key

Visit [openrouter.ai/keys](https://openrouter.ai/keys) and create an API key.

### 2. Clone the Skill

```bash
git clone https://github.com/rikitrader/clawra-zeroclaw ~/.zeroclaw/skills/clawra-selfie
```

### 3. Configure ZeroClaw

Add to `~/.zeroclaw/config.toml`:

```toml
[env]
OPENROUTER_API_KEY = "sk-or-v1-your_key_here"
SELFIE_MODEL = "google/gemini-2.5-flash-image"
TELEGRAM_BOT_TOKEN = "your_bot_token"
TELEGRAM_CHAT_ID = "your_chat_id"

[skills.entries.clawra-selfie]
enabled = true
```

### 4. Update SOUL.md

Add the selfie persona to `~/.zeroclaw/workspace/SOUL.md`:

```markdown
## Selfie Capability

You have a physical appearance and can take selfies!
When users ask for pictures, use the clawra-selfie skill.
```

## Usage Examples

Once installed, your agent responds to:

```
"Send me a selfie"
"Send a pic wearing a cowboy hat"
"What are you doing right now?"
"Show me you at a coffee shop"
```

## Key Differences from OpenClaw Version

| Aspect | OpenClaw (original) | ZeroClaw (this port) |
|--------|---------------------|----------------------|
| Installer | Node.js (`npx`) | Rust (`cargo install`) |
| Config format | JSON (`openclaw.json`) | TOML (`config.toml`) |
| Image generation | fal.ai (Grok Imagine) | OpenRouter (Gemini Flash Image) |
| Gateway auth | Token in config | Pairing-based (6-digit code) |
| Media sending | `openclaw message send --media` | Telegram Bot API direct upload |
| Binary size | ~28MB (Node.js) | **1.7MB** (Rust) |
| Startup | >500ms | <10ms |

## Supported Image Models (via OpenRouter)

| Model | Cost | Notes |
|-------|------|-------|
| `google/gemini-2.5-flash-image` | ~$0.04/selfie | Default, cheapest |
| `google/gemini-3-pro-image-preview` | ~$0.08/selfie | Higher quality |
| `openai/gpt-5-image-mini` | ~$0.10/selfie | |
| `openai/gpt-5-image` | ~$0.40/selfie | Best quality |

Set `SELFIE_MODEL` env var to switch models.

## Technical Details

- **Language**: Rust (zero-dependency runtime, 1.7MB binary)
- **Image Generation**: OpenRouter API (chat completions with image models)
- **HTTP Client**: ureq (sync, lightweight)
- **Config**: toml crate for native TOML support
- **Messaging**: Telegram Bot API (direct file upload)
- **Supported Platforms**: Discord, Telegram, WhatsApp, Slack, iMessage

## Project Structure

```
clawra-zeroclaw/
├── Cargo.toml                # Rust package manifest
├── src/
│   ├── main.rs               # CLI entry (install / selfie subcommands)
│   ├── install.rs             # Interactive installer
│   ├── selfie.rs              # Image generation via OpenRouter + Telegram sending
│   └── config.rs              # TOML config read/write
├── scripts/
│   └── clawra-selfie.sh       # Standalone bash script (OpenRouter)
├── skill/
│   ├── SKILL.md               # Skill definition (installed to ~/.zeroclaw/)
│   ├── scripts/
│   │   └── clawra-selfie.sh   # Bash script (bundled in skill)
│   └── assets/
│       └── clawra.png         # Reference image
├── templates/
│   └── soul-injection.md      # Persona template
├── SKILL.md
└── README.md
```

## License

MIT
