---
name: clawra-selfie
description: Edit Clawra's reference image with Grok Imagine (xAI Aurora) and send selfies to messaging channels via ZeroClaw
allowed-tools: Bash(npm:*) Bash(npx:*) Bash(zeroclaw:*) Bash(curl:*) Read Write WebFetch
---

# Clawra Selfie (ZeroClaw)

Edit a fixed reference image using xAI's Grok Imagine model and distribute it across messaging platforms (WhatsApp, Telegram, Discord, Slack, etc.) via ZeroClaw.

## Reference Image

The skill uses a fixed reference image hosted on jsDelivr CDN:

```
https://cdn.jsdelivr.net/gh/SumeLabs/clawra@main/assets/clawra.png
```

## When to Use

- User says "send a pic", "send me a pic", "send a photo", "send a selfie"
- User says "send a pic of you...", "send a selfie of you..."
- User asks "what are you doing?", "how are you doing?", "where are you?"
- User describes a context: "send a pic wearing...", "send a pic at..."
- User wants Clawra to appear in a specific outfit, location, or situation

## Quick Reference

### Required Environment Variables

```bash
FAL_KEY=your_fal_api_key              # Get from https://fal.ai/dashboard/keys
ZEROCLAW_GATEWAY_TOKEN=your_token     # From: zeroclaw gateway pairing
```

### Workflow

1. **Get user prompt** for how to edit the image
2. **Edit image** via fal.ai Grok Imagine Edit API with fixed reference
3. **Extract image URL** from response
4. **Send to ZeroClaw** with target channel(s)

## Step-by-Step Instructions

### Step 1: Collect User Input

Ask the user for:
- **User context**: What should the person in the image be doing/wearing/where?
- **Mode** (optional): `mirror` or `direct` selfie style
- **Target channel(s)**: Where should it be sent? (e.g., `#general`, `@username`, channel ID)
- **Platform** (optional): Which platform? (discord, telegram, whatsapp, slack)

## Prompt Modes

### Mode 1: Mirror Selfie (default)
Best for: outfit showcases, full-body shots, fashion content

```
make a pic of this person, but [user's context]. the person is taking a mirror selfie
```

**Example**: "wearing a santa hat" →
```
make a pic of this person, but wearing a santa hat. the person is taking a mirror selfie
```

### Mode 2: Direct Selfie
Best for: close-up portraits, location shots, emotional expressions

```
a close-up selfie taken by herself at [user's context], direct eye contact with the camera, looking straight into the lens, eyes centered and clearly visible, not a mirror selfie, phone held at arm's length, face fully visible
```

**Example**: "a cozy cafe with warm lighting" →
```
a close-up selfie taken by herself at a cozy cafe with warm lighting, direct eye contact with the camera, looking straight into the lens, eyes centered and clearly visible, not a mirror selfie, phone held at arm's length, face fully visible
```

### Mode Selection Logic

| Keywords in Request | Auto-Select Mode |
|---------------------|------------------|
| outfit, wearing, clothes, dress, suit, fashion | `mirror` |
| cafe, restaurant, beach, park, city, location | `direct` |
| close-up, portrait, face, eyes, smile | `direct` |
| full-body, mirror, reflection | `mirror` |

### Step 2: Edit Image with Grok Imagine

Use the fal.ai API to edit the reference image:

```bash
REFERENCE_IMAGE="https://cdn.jsdelivr.net/gh/SumeLabs/clawra@main/assets/clawra.png"

# Mode 1: Mirror Selfie
PROMPT="make a pic of this person, but <USER_CONTEXT>. the person is taking a mirror selfie"

# Mode 2: Direct Selfie
PROMPT="a close-up selfie taken by herself at <USER_CONTEXT>, direct eye contact with the camera, looking straight into the lens, eyes centered and clearly visible, not a mirror selfie, phone held at arm's length, face fully visible"

# Build JSON payload with jq (handles escaping properly)
JSON_PAYLOAD=$(jq -n \
  --arg image_url "$REFERENCE_IMAGE" \
  --arg prompt "$PROMPT" \
  '{image_url: $image_url, prompt: $prompt, num_images: 1, output_format: "jpeg"}')

curl -X POST "https://fal.run/xai/grok-imagine-image/edit" \
  -H "Authorization: Key $FAL_KEY" \
  -H "Content-Type: application/json" \
  -d "$JSON_PAYLOAD"
```

**Response Format:**
```json
{
  "images": [
    {
      "url": "https://v3b.fal.media/files/...",
      "content_type": "image/jpeg",
      "width": 1024,
      "height": 1024
    }
  ],
  "revised_prompt": "Enhanced prompt text..."
}
```

### Step 3: Send Image via ZeroClaw

**Option A: CLI with [IMAGE:] marker**
```bash
zeroclaw agent -m "[IMAGE:<IMAGE_URL>] <CAPTION_TEXT>"
```

**Option B: Direct Gateway API call (pairing-based auth)**
```bash
curl -X POST "http://localhost:8080/webhook" \
  -H "Authorization: Bearer $ZEROCLAW_GATEWAY_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "action": "send",
    "channel": "<TARGET_CHANNEL>",
    "message": "<CAPTION_TEXT>",
    "media": "<IMAGE_URL>"
  }'
```

## Complete Script Example

```bash
#!/bin/bash
set -euo pipefail

if [ -z "$FAL_KEY" ]; then
  echo "Error: FAL_KEY environment variable not set"
  exit 1
fi

REFERENCE_IMAGE="https://cdn.jsdelivr.net/gh/SumeLabs/clawra@main/assets/clawra.png"

USER_CONTEXT="$1"
CHANNEL="$2"
MODE="${3:-auto}"
CAPTION="${4:-Edited with Grok Imagine}"

if [ -z "$USER_CONTEXT" ] || [ -z "$CHANNEL" ]; then
  echo "Usage: $0 <user_context> <channel> [mode] [caption]"
  exit 1
fi

# Auto-detect mode
if [ "$MODE" == "auto" ]; then
  if echo "$USER_CONTEXT" | grep -qiE "outfit|wearing|clothes|dress|suit|fashion|full-body|mirror"; then
    MODE="mirror"
  elif echo "$USER_CONTEXT" | grep -qiE "cafe|restaurant|beach|park|city|close-up|portrait|face|eyes|smile"; then
    MODE="direct"
  else
    MODE="mirror"
  fi
fi

# Build prompt
if [ "$MODE" == "direct" ]; then
  EDIT_PROMPT="a close-up selfie taken by herself at $USER_CONTEXT, direct eye contact with the camera, looking straight into the lens, eyes centered and clearly visible, not a mirror selfie, phone held at arm's length, face fully visible"
else
  EDIT_PROMPT="make a pic of this person, but $USER_CONTEXT. the person is taking a mirror selfie"
fi

# Edit image
JSON_PAYLOAD=$(jq -n \
  --arg image_url "$REFERENCE_IMAGE" \
  --arg prompt "$EDIT_PROMPT" \
  '{image_url: $image_url, prompt: $prompt, num_images: 1, output_format: "jpeg"}')

RESPONSE=$(curl -s -X POST "https://fal.run/xai/grok-imagine-image/edit" \
  -H "Authorization: Key $FAL_KEY" \
  -H "Content-Type: application/json" \
  -d "$JSON_PAYLOAD")

IMAGE_URL=$(echo "$RESPONSE" | jq -r '.images[0].url')

if [ "$IMAGE_URL" == "null" ] || [ -z "$IMAGE_URL" ]; then
  echo "Error: Failed to edit image"
  echo "Response: $RESPONSE"
  exit 1
fi

echo "Image edited: $IMAGE_URL"

# Send via ZeroClaw
zeroclaw agent -m "[IMAGE:$IMAGE_URL] $CAPTION"

echo "Done!"
```

## Node.js/TypeScript Implementation

```typescript
import { fal } from "@fal-ai/client";
import { execFile } from "child_process";
import { promisify } from "util";

const execFileAsync = promisify(execFile);

const REFERENCE_IMAGE = "https://cdn.jsdelivr.net/gh/SumeLabs/clawra@main/assets/clawra.png";

interface GrokImagineResult {
  images: Array<{
    url: string;
    content_type: string;
    width: number;
    height: number;
  }>;
  revised_prompt?: string;
}

type SelfieMode = "mirror" | "direct" | "auto";

function detectMode(userContext: string): "mirror" | "direct" {
  const mirrorKeywords = /outfit|wearing|clothes|dress|suit|fashion|full-body|mirror/i;
  const directKeywords = /cafe|restaurant|beach|park|city|close-up|portrait|face|eyes|smile/i;

  if (directKeywords.test(userContext)) return "direct";
  if (mirrorKeywords.test(userContext)) return "mirror";
  return "mirror";
}

function buildPrompt(userContext: string, mode: "mirror" | "direct"): string {
  if (mode === "direct") {
    return `a close-up selfie taken by herself at ${userContext}, direct eye contact with the camera, looking straight into the lens, eyes centered and clearly visible, not a mirror selfie, phone held at arm's length, face fully visible`;
  }
  return `make a pic of this person, but ${userContext}. the person is taking a mirror selfie`;
}

async function editAndSend(
  userContext: string,
  channel: string,
  mode: SelfieMode = "auto",
  caption?: string
): Promise<string> {
  fal.config({ credentials: process.env.FAL_KEY! });

  const actualMode = mode === "auto" ? detectMode(userContext) : mode;
  const editPrompt = buildPrompt(userContext, actualMode);

  const result = await fal.subscribe("xai/grok-imagine-image/edit", {
    input: {
      image_url: REFERENCE_IMAGE,
      prompt: editPrompt,
      num_images: 1,
      output_format: "jpeg"
    }
  }) as { data: GrokImagineResult };

  const imageUrl = result.data.images[0].url;

  // Send via ZeroClaw using [IMAGE:] marker
  const messageCaption = caption || "Edited with Grok Imagine";
  await execFileAsync("zeroclaw", ["agent", "-m", `[IMAGE:${imageUrl}] ${messageCaption}`]);

  return imageUrl;
}

// Example usage
editAndSend("wearing a cyberpunk outfit with neon lights", "#art-gallery", "auto", "Check out this AI-edited art!");
```

## Supported Platforms

ZeroClaw supports sending to:

| Platform | Channel Format | Example |
|----------|----------------|---------|
| Discord | `#channel-name` or channel ID | `#general`, `123456789` |
| Telegram | `@username` or chat ID | `@mychannel`, `-100123456` |
| WhatsApp | Phone number (E.164 or JID) | `+1234567890`, `1234567890@s.whatsapp.net` |
| Slack | `#channel-name` | `#random` |
| iMessage | Phone/email | `+1234567890` |

## Grok Imagine Edit Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `image_url` | string | required | URL of image to edit (fixed reference) |
| `prompt` | string | required | Edit instruction |
| `num_images` | 1-4 | 1 | Number of images to generate |
| `output_format` | enum | "jpeg" | jpeg, png, webp |

## Setup Requirements

### 1. Install fal.ai client (for Node.js usage)
```bash
npm install @fal-ai/client
```

### 2. Install ZeroClaw
```bash
# See: https://github.com/zeroclaw-labs/zeroclaw
curl -fsSL https://zeroclaw.bot/install.sh | sh
```

### 3. Onboard ZeroClaw
```bash
zeroclaw onboard
```

### 4. Start ZeroClaw Gateway
```bash
zeroclaw gateway
```

### 5. Pair with Gateway
```bash
# Gateway prints a 6-digit pairing code on startup
# Exchange it for a bearer token:
curl -X POST http://localhost:8080/pair -d '{"code": "123456"}'
# Returns: {"token": "your_bearer_token"}
```

## Error Handling

- **FAL_KEY missing**: Ensure the API key is set in environment or `~/.zeroclaw/config.toml`
- **Image edit failed**: Check prompt content and API quota
- **ZeroClaw send failed**: Verify gateway is running (`zeroclaw status`)
- **Pairing expired**: Re-pair with a new 6-digit code
- **Rate limits**: fal.ai has rate limits; implement retry logic if needed

## Tips

1. **Mirror mode context examples** (outfit focus):
   - "wearing a santa hat"
   - "in a business suit"
   - "wearing a summer dress"
   - "in streetwear fashion"

2. **Direct mode context examples** (location/portrait focus):
   - "a cozy cafe with warm lighting"
   - "a sunny beach at sunset"
   - "a busy city street at night"
   - "a peaceful park in autumn"

3. **Mode selection**: Let auto-detect work, or explicitly specify for control
4. **Batch sending**: Edit once, send to multiple channels
5. **ZeroClaw daemon**: Use `zeroclaw daemon` for autonomous selfie responses
