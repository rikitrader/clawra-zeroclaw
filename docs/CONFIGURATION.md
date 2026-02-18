# Configuration

## Environment Variables

All environment variables are set in the ZeroClaw `config.toml` under the `[env]` section.

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `OPENROUTER_API_KEY` | Yes | — | OpenRouter API key for image generation |
| `SELFIE_MODEL` | No | `google/gemini-2.5-flash-image` | OpenRouter model ID for image generation |
| `TELEGRAM_BOT_TOKEN` | No | — | Telegram bot token for direct photo upload |
| `TELEGRAM_CHAT_ID` | No | — | Telegram chat/user ID for photo delivery |

## ZeroClaw Config (`~/.zeroclaw/config.toml`)

### Skill Entry

```toml
[skills.entries.clawra-selfie]
enabled = true
```

### Environment Variables

```toml
[env]
OPENROUTER_API_KEY = "sk-or-v1-your_key_here"
SELFIE_MODEL = "google/gemini-2.5-flash-image"
TELEGRAM_BOT_TOKEN = "123456:ABC-..."
TELEGRAM_CHAT_ID = "987654321"
```

### Dedicated Instance

For a dedicated Clawra bot (separate from main ZeroClaw), create a workspace at `~/.zeroclaw-clawra/`:

```toml
# ~/.zeroclaw-clawra/config.toml
default_model = "google/gemini-2.5-flash"
default_provider = "openrouter"

[channels_config.telegram]
bot_token = "your_dedicated_bot_token"
allowed_users = ["your_telegram_user_id"]

[env]
OPENROUTER_API_KEY = "sk-or-v1-..."
SELFIE_MODEL = "google/gemini-2.5-flash-image"
TELEGRAM_BOT_TOKEN = "your_dedicated_bot_token"
TELEGRAM_CHAT_ID = "your_telegram_user_id"

[skills.entries.clawra-selfie]
enabled = true
```

Start with: `ZEROCLAW_WORKSPACE=~/.zeroclaw-clawra zeroclaw daemon --port 3001`

## CLI Options

### `clawra selfie`

| Option | Default | Description |
|--------|---------|-------------|
| `<context>` | (required) | Scene description (e.g., "at the beach wearing a bikini") |
| `<channel>` | (required) | Target channel name |
| `--mode` | `auto` | Selfie mode: `auto`, `mirror`, or `direct` |
| `--caption` | `""` | Photo caption text |
| `--format` | `jpeg` | Output format: `jpeg`, `png`, or `webp` |

## Model Selection

Set `SELFIE_MODEL` to any OpenRouter model that supports image output:

| Model ID | Quality | Cost/selfie | Notes |
|----------|---------|-------------|-------|
| `google/gemini-2.5-flash-image` | Good | ~$0.04 | Default, cheapest |
| `google/gemini-3-pro-image-preview` | Better | ~$0.08 | Higher detail |
| `openai/gpt-5-image-mini` | Good | ~$0.10 | Alternative |
| `openai/gpt-5-image` | Best | ~$0.40 | Highest quality |
