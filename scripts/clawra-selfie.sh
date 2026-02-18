#!/bin/bash
# clawra-selfie.sh
# Generate selfie with Grok Imagine and send via ZeroClaw
#
# Usage: ./clawra-selfie.sh "<user_context>" "<channel>" ["<caption>"] ["<mode>"]
#
# Environment variables required:
#   FAL_KEY                - Your fal.ai API key
#   ZEROCLAW_GATEWAY_URL   - Gateway URL (default: http://localhost:8080)
#   ZEROCLAW_GATEWAY_TOKEN - Paired bearer token
#
# Example:
#   FAL_KEY=key ./clawra-selfie.sh "wearing a cowboy hat" "#general" "Howdy!" mirror

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

if [ -z "${FAL_KEY:-}" ]; then
    log_error "FAL_KEY environment variable not set"
    echo "Get your API key from: https://fal.ai/dashboard/keys"
    exit 1
fi

if ! command -v jq &> /dev/null; then
    log_error "jq is required but not installed"
    echo "Install with: brew install jq (macOS) or apt install jq (Linux)"
    exit 1
fi

# Check for zeroclaw CLI
USE_CLI=false
if command -v zeroclaw &> /dev/null; then
    USE_CLI=true
fi

# Fixed reference image (Clawra character)
REFERENCE_IMAGE="https://cdn.jsdelivr.net/gh/SumeLabs/clawra@main/assets/clawra.png"

USER_CONTEXT="${1:-}"
CHANNEL="${2:-}"
CAPTION="${3:-Edited with Grok Imagine}"
MODE="${4:-auto}"
OUTPUT_FORMAT="${5:-jpeg}"

if [ -z "$USER_CONTEXT" ] || [ -z "$CHANNEL" ]; then
    echo "Usage: $0 <user_context> <channel> [caption] [mode] [output_format]"
    echo ""
    echo "Arguments:"
    echo "  user_context  - What the person should be doing/wearing/where (required)"
    echo "  channel       - Target channel (required) e.g., #general, @user, chat_id"
    echo "  caption       - Message caption (default: 'Edited with Grok Imagine')"
    echo "  mode          - Selfie mode: mirror, direct, auto (default: auto)"
    echo "  output_format - Image format: jpeg, png, webp (default: jpeg)"
    echo ""
    echo "Examples:"
    echo "  $0 'wearing a cowboy hat' '#general' 'Howdy!' mirror"
    echo "  $0 'a cozy cafe' '#photography' '' direct"
    exit 1
fi

# Auto-detect mode based on keywords
if [ "$MODE" == "auto" ]; then
    if echo "$USER_CONTEXT" | grep -qiE "outfit|wearing|clothes|dress|suit|fashion|full-body|mirror"; then
        MODE="mirror"
    elif echo "$USER_CONTEXT" | grep -qiE "cafe|restaurant|beach|park|city|close-up|portrait|face|eyes|smile"; then
        MODE="direct"
    else
        MODE="mirror"
    fi
    log_info "Auto-detected mode: $MODE"
fi

# Construct prompt based on mode
if [ "$MODE" == "direct" ]; then
    EDIT_PROMPT="a close-up selfie taken by herself at $USER_CONTEXT, direct eye contact with the camera, looking straight into the lens, eyes centered and clearly visible, not a mirror selfie, phone held at arm's length, face fully visible"
else
    EDIT_PROMPT="make a pic of this person, but $USER_CONTEXT. the person is taking a mirror selfie"
fi

log_info "Mode: $MODE"
log_info "Editing reference image..."

# Build JSON payload with jq for proper escaping
JSON_PAYLOAD=$(jq -n \
    --arg image_url "$REFERENCE_IMAGE" \
    --arg prompt "$EDIT_PROMPT" \
    '{image_url: $image_url, prompt: $prompt, num_images: 1, output_format: "jpeg"}')

# Edit image via fal.ai Grok Imagine
RESPONSE=$(curl -s -X POST "https://fal.run/xai/grok-imagine-image/edit" \
    -H "Authorization: Key $FAL_KEY" \
    -H "Content-Type: application/json" \
    -d "$JSON_PAYLOAD")

# Check for errors
if echo "$RESPONSE" | jq -e '.error' > /dev/null 2>&1; then
    ERROR_MSG=$(echo "$RESPONSE" | jq -r '.error // .detail // "Unknown error"')
    log_error "Image edit failed: $ERROR_MSG"
    exit 1
fi

# Extract image URL
IMAGE_URL=$(echo "$RESPONSE" | jq -r '.images[0].url // empty')

if [ -z "$IMAGE_URL" ]; then
    log_error "Failed to extract image URL from response"
    echo "Response: $RESPONSE"
    exit 1
fi

log_info "Image edited: $IMAGE_URL"
log_info "Sending to channel: $CHANNEL"

# Send via ZeroClaw
SENT=false

if [ "$USE_CLI" = true ]; then
    # ZeroClaw supports [IMAGE:url] markers in agent messages
    if zeroclaw agent -m "[IMAGE:$IMAGE_URL] $CAPTION" 2>/dev/null; then
        SENT=true
    else
        log_warn "CLI send failed, trying direct API..."
    fi
fi

if [ "$SENT" = false ]; then
    # Direct API call to ZeroClaw gateway (pairing-based auth, port 8080)
    GATEWAY_URL="${ZEROCLAW_GATEWAY_URL:-http://localhost:8080}"
    GATEWAY_TOKEN="${ZEROCLAW_GATEWAY_TOKEN:-}"

    if [ -z "$GATEWAY_TOKEN" ]; then
        log_error "ZEROCLAW_GATEWAY_TOKEN not set."
        log_error "Pair with: curl -X POST $GATEWAY_URL/pair -d '{\"code\": \"<6-digit-code>\"}'"
        exit 1
    fi

    SEND_PAYLOAD=$(jq -n \
        --arg channel "$CHANNEL" \
        --arg message "$CAPTION" \
        --arg media "$IMAGE_URL" \
        '{action: "send", channel: $channel, message: $message, media: $media}')

    curl -s -X POST "$GATEWAY_URL/webhook" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $GATEWAY_TOKEN" \
        -d "$SEND_PAYLOAD"
fi

log_info "Done! Image sent to $CHANNEL"

echo ""
echo "--- Result ---"
jq -n \
    --arg url "$IMAGE_URL" \
    --arg channel "$CHANNEL" \
    --arg context "$USER_CONTEXT" \
    --arg mode "$MODE" \
    '{success: true, image_url: $url, channel: $channel, context: $context, mode: $mode}'
