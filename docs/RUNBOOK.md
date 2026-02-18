# Runbook

## Prerequisites

- Rust toolchain (1.70+): `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- [ZeroClaw](https://github.com/zeroclaw-labs/zeroclaw) installed and configured
- [OpenRouter](https://openrouter.ai) account with API key
- curl (for Telegram media upload)

## Install

### Option A: Cargo Install (recommended)

```bash
cargo install clawra-zeroclaw
clawra install
```

### Option B: Build from Source

```bash
git clone https://github.com/rikitrader/clawra-zeroclaw
cd clawra-zeroclaw
cargo build --release
./target/release/clawra install
```

### Option C: Manual Skill Install

```bash
git clone https://github.com/rikitrader/clawra-zeroclaw ~/.zeroclaw/skills/clawra-selfie
```

Then manually add to `~/.zeroclaw/config.toml`:

```toml
[env]
OPENROUTER_API_KEY = "sk-or-v1-..."
SELFIE_MODEL = "google/gemini-2.5-flash-image"

[skills.entries.clawra-selfie]
enabled = true
```

## Build

```bash
cargo build                  # Debug build
cargo build --release        # Release build (optimized, 1.7MB)
cargo fmt --check            # Check formatting
cargo clippy -- -D warnings  # Lint
```

## Test

```bash
# CLI test
clawra selfie "at a coffee shop" "test" --mode direct --caption "Testing!"

# Verify Telegram delivery
export TELEGRAM_BOT_TOKEN="your_token"
export TELEGRAM_CHAT_ID="your_chat_id"
export OPENROUTER_API_KEY="your_key"
clawra selfie "relaxing at home" "telegram" --caption "Test selfie"

# Bash script test
bash scripts/clawra-selfie.sh "at the beach" "Test caption"
```

## Deploy

The skill is deployed by running `clawra install` on the target machine. Files are installed to:

| Path | Content |
|------|---------|
| `~/.zeroclaw/skills/clawra-selfie/SKILL.md` | Skill definition |
| `~/.zeroclaw/skills/clawra-selfie/scripts/clawra-selfie.sh` | Bash execution script |
| `~/.zeroclaw/config.toml` | Updated with API key and skill entry |
| `~/.zeroclaw/workspace/SOUL.md` | Updated with selfie persona |
| `~/.zeroclaw/workspace/IDENTITY.md` | Agent identity template |

## Troubleshoot

### Images not generating

1. Check API key: `curl -s https://openrouter.ai/api/v1/models -H "Authorization: Bearer $OPENROUTER_API_KEY" | head -c 200`
2. Check model availability: Ensure `SELFIE_MODEL` is a valid image model on OpenRouter
3. Check temp files: `ls -lt /tmp/jenni-selfie-*` (or `clawra-selfie-*`)
4. Test directly: `bash ~/.zeroclaw/skills/clawra-selfie/scripts/clawra-selfie.sh "test" "test"`

### Images generating but not sending

1. Check Telegram bot: `curl -s https://api.telegram.org/bot$TELEGRAM_BOT_TOKEN/getMe`
2. Check chat ID: `curl -s https://api.telegram.org/bot$TELEGRAM_BOT_TOKEN/sendMessage -d "chat_id=$TELEGRAM_CHAT_ID&text=test"`
3. Check ZeroClaw daemon: `zeroclaw doctor` (look for stale Telegram channel)
4. Restart daemon if channel is stale: `kill $(pgrep -f "zeroclaw daemon") && zeroclaw daemon &`

### Skill not triggering

1. Verify skill is loaded: `zeroclaw skills list`
2. Check SKILL.md exists: `cat ~/.zeroclaw/skills/clawra-selfie/SKILL.md`
3. Check config: `grep clawra-selfie ~/.zeroclaw/config.toml`
4. Verify SOUL.md has selfie persona: `grep "Selfie Capability" ~/.zeroclaw/workspace/SOUL.md`

### Common Errors

| Error | Cause | Fix |
|-------|-------|-----|
| `OPENROUTER_API_KEY not set` | Env var missing | Add to `config.toml` `[env]` section |
| `No image generated` | Model doesn't support image output | Switch to `google/gemini-2.5-flash-image` |
| `Telegram send issue` | Invalid bot token or chat ID | Verify with `getMe` and `sendMessage` |
| `curl failed` | curl not installed | Install curl (`brew install curl` / `apt install curl`) |
