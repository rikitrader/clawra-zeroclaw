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
2. Guide you to get a fal.ai API key
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
- [fal.ai](https://fal.ai) account (free tier available)
- Rust toolchain (`rustup` / `cargo`)

## Manual Installation

### 1. Get API Key

Visit [fal.ai/dashboard/keys](https://fal.ai/dashboard/keys) and create an API key.

### 2. Clone the Skill

```bash
git clone https://github.com/rikitrader/clawra-zeroclaw ~/.zeroclaw/skills/clawra-selfie
```

### 3. Configure ZeroClaw

Add to `~/.zeroclaw/config.toml`:

```toml
[env]
FAL_KEY = "your_fal_key_here"

[skills.entries.clawra-selfie]
enabled = true
```

### 4. Update SOUL.md

Add the selfie persona to `~/.zeroclaw/workspace/SOUL.md`:

```markdown
## Clawra Selfie Capability

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
| Gateway port | 18789 | 8080 |
| Gateway auth | Token in config | Pairing-based (6-digit code) |
| Media sending | `openclaw message send --media` | `[IMAGE:url]` markers or `/webhook` API |
| Binary size | ~28MB (Node.js) | **1.7MB** (Rust) |
| Startup | >500ms | <10ms |

## Reference Image

```
https://cdn.jsdelivr.net/gh/SumeLabs/clawra@main/assets/clawra.png
```

## Technical Details

- **Language**: Rust (zero-dependency runtime, 1.7MB binary)
- **Image Generation**: xAI Grok Imagine via fal.ai
- **HTTP Client**: ureq (sync, lightweight)
- **Config**: toml crate for native TOML support
- **Messaging**: ZeroClaw Gateway API (pairing-based auth)
- **Supported Platforms**: Discord, Telegram, WhatsApp, Slack, iMessage

## Project Structure

```
clawra-zeroclaw/
├── Cargo.toml                # Rust package manifest
├── src/
│   ├── main.rs               # CLI entry (install / selfie subcommands)
│   ├── install.rs             # Interactive installer
│   ├── selfie.rs              # Image generation + ZeroClaw sending
│   └── config.rs              # TOML config read/write
├── scripts/
│   └── clawra-selfie.sh       # Standalone bash script
├── skill/
│   ├── SKILL.md               # Skill definition (installed to ~/.zeroclaw/)
│   ├── scripts/
│   │   └── clawra-selfie.sh   # Bash script (bundled in skill)
│   └── assets/
│       └── clawra.png         # Reference image
├── templates/
│   └── soul-injection.md      # Persona template
├── assets/
│   └── clawra.png             # Reference image
├── SKILL.md
└── README.md
```

## License

MIT
