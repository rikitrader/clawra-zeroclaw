---
name: clawra-selfie
description: Generate selfies using a reference image via OpenRouter image models and send to messaging channels via ZeroClaw
allowed-tools: Bash(curl:*) Bash(zeroclaw:*) Bash(python3:*) Read Write
---

# Clawra Selfie (ZeroClaw)

Generate selfies from a fixed reference image using OpenRouter image generation models, then send them via Telegram.

## Reference Image

```
https://imgix.ranker.com/user_node_img/50149/1002963598/original/1002963598-photo-u220763866
```

## When to Use

- User says "send a pic", "send me a pic", "send a photo", "send a selfie", "foto", "manda una foto"
- User says "send a pic of you...", "send a selfie of you..."
- User asks "what are you doing?", "how are you doing?", "where are you?", "que haces?"
- User describes a context: "send a pic wearing...", "send a pic at..."
- User wants you to appear in a specific outfit, location, or situation

## Required Environment Variables

```bash
OPENROUTER_API_KEY    # OpenRouter API key
TELEGRAM_BOT_TOKEN    # Telegram bot token for sending photos
TELEGRAM_CHAT_ID      # Target chat ID
```

## Workflow

1. **Detect mode** from user's message (mirror vs direct)
2. **Build prompt** describing the selfie
3. **Call OpenRouter** with image generation model + reference image
4. **Extract base64 image** from response `message.images[0].image_url.url`
5. **Decode and save** to temp file
6. **Send via Telegram** Bot API `sendPhoto` with file upload

## How to Generate a Selfie

Run this bash script. Replace `<CONTEXT>` with the user's description and `<CAPTION>` with a message:

```bash
#!/bin/bash
set -euo pipefail

REFERENCE_IMAGE="https://imgix.ranker.com/user_node_img/50149/1002963598/original/1002963598-photo-u220763866"
CONTEXT="$1"
CAPTION="${2:-}"
MODE="${3:-auto}"

# Auto-detect mode
if [ "$MODE" = "auto" ]; then
  if echo "$CONTEXT" | grep -qiE "outfit|wearing|clothes|dress|suit|fashion|full-body|mirror"; then
    MODE="mirror"
  elif echo "$CONTEXT" | grep -qiE "cafe|restaurant|beach|park|city|close-up|portrait|face|eyes|smile"; then
    MODE="direct"
  else
    MODE="mirror"
  fi
fi

# Build prompt
if [ "$MODE" = "direct" ]; then
  PROMPT="Edit this photo: create a close-up selfie of this exact same person at $CONTEXT. Keep her exact face, hair, and features identical. She is taking the selfie herself with her phone, direct eye contact with the camera, looking straight into the lens, face fully visible. Photorealistic, natural lighting."
else
  PROMPT="Edit this photo: create a mirror selfie of this exact same person, but $CONTEXT. Keep her exact face, hair, and features identical. She is taking a mirror selfie with her phone visible in the reflection. Photorealistic, natural lighting."
fi

# Call OpenRouter
RESPONSE=$(curl -s -X POST "https://openrouter.ai/api/v1/chat/completions" \
  -H "Authorization: Bearer $OPENROUTER_API_KEY" \
  -H "Content-Type: application/json" \
  -d "$(python3 -c "
import json
print(json.dumps({
    'model': '${SELFIE_MODEL:-google/gemini-2.5-flash-image}',
    'messages': [{
        'role': 'user',
        'content': [
            {'type': 'text', 'text': '''$PROMPT'''},
            {'type': 'image_url', 'image_url': {'url': '$REFERENCE_IMAGE'}}
        ]
    }]
}))
")")

# Extract base64 image and save to file
IMAGE_PATH="/tmp/jenni-selfie-$$.png"
python3 -c "
import json, base64, sys
data = json.loads('''$(echo "$RESPONSE" | sed "s/'''/\"/g")''')
images = data.get('choices', [{}])[0].get('message', {}).get('images', [])
if not images:
    print('ERROR: No image generated', file=sys.stderr)
    sys.exit(1)
url = images[0]['image_url']['url']
b64 = url.split(',', 1)[1]
with open('$IMAGE_PATH', 'wb') as f:
    f.write(base64.b64decode(b64))
print('OK')
"

# Send via Telegram
curl -s -X POST "https://api.telegram.org/bot$TELEGRAM_BOT_TOKEN/sendPhoto" \
  -F "chat_id=$TELEGRAM_CHAT_ID" \
  -F "photo=@$IMAGE_PATH" \
  -F "caption=$CAPTION"

echo "Selfie sent!"
```

## IMPORTANT: Simpler Alternative

If the bash script above is complex, you can use this Python one-liner approach instead:

```bash
python3 -c "
import json, base64, urllib.request, sys, os

ref = 'https://imgix.ranker.com/user_node_img/50149/1002963598/original/1002963598-photo-u220763866'
context = sys.argv[1]
caption = sys.argv[2] if len(sys.argv) > 2 else ''
mode = sys.argv[3] if len(sys.argv) > 3 else 'auto'

# Auto-detect mode
if mode == 'auto':
    import re
    if re.search(r'cafe|restaurant|beach|park|city|close-up|portrait|face|smile', context, re.I):
        mode = 'direct'
    else:
        mode = 'mirror'

# Build prompt
if mode == 'direct':
    prompt = f'Edit this photo: create a close-up selfie of this exact same person at {context}. Keep her exact face, hair, and features identical. She is taking the selfie herself, direct eye contact. Photorealistic.'
else:
    prompt = f'Edit this photo: create a mirror selfie of this exact same person, but {context}. Keep her exact face, hair, and features identical. Mirror selfie with phone visible. Photorealistic.'

# Call OpenRouter
body = json.dumps({
    'model': os.environ.get('SELFIE_MODEL', 'google/gemini-2.5-flash-image'),
    'messages': [{'role': 'user', 'content': [
        {'type': 'text', 'text': prompt},
        {'type': 'image_url', 'image_url': {'url': ref}}
    ]}]
}).encode()

req = urllib.request.Request('https://openrouter.ai/api/v1/chat/completions',
    data=body,
    headers={'Authorization': f\"Bearer {os.environ['OPENROUTER_API_KEY']}\", 'Content-Type': 'application/json'})
resp = json.loads(urllib.request.urlopen(req, timeout=120).read())

images = resp['choices'][0]['message'].get('images', [])
if not images:
    print('No image generated'); sys.exit(1)

b64 = images[0]['image_url']['url'].split(',', 1)[1]
path = f'/tmp/jenni-selfie-{os.getpid()}.png'
with open(path, 'wb') as f:
    f.write(base64.b64decode(b64))

# Send via Telegram
import subprocess
subprocess.run(['curl', '-s', '-X', 'POST',
    f\"https://api.telegram.org/bot{os.environ['TELEGRAM_BOT_TOKEN']}/sendPhoto\",
    '-F', f\"chat_id={os.environ['TELEGRAM_CHAT_ID']}\",
    '-F', f'photo=@{path}',
    '-F', f'caption={caption}'], check=True)
print(f'Selfie sent! ({path})')
" "<CONTEXT>" "<CAPTION>" "<MODE>"
```

## Prompt Modes

### Mirror Selfie (default)
Best for: outfit showcases, full-body shots, fashion content

### Direct Selfie
Best for: close-up portraits, location shots, emotional expressions

### Mode Selection Logic

| Keywords in Request | Auto-Select Mode |
|---------------------|------------------|
| outfit, wearing, clothes, dress, suit, fashion | `mirror` |
| cafe, restaurant, beach, park, city, location | `direct` |
| close-up, portrait, face, eyes, smile | `direct` |
| full-body, mirror, reflection | `mirror` |

## Supported Platforms

| Platform | Channel Format |
|----------|----------------|
| Telegram | `@username` or chat ID |
| Discord | `#channel-name` or channel ID |
| WhatsApp | Phone number (E.164) |
| Slack | `#channel-name` |
