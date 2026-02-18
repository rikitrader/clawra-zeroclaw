/**
 * Grok Imagine to ZeroClaw Integration
 *
 * Generates selfie images using xAI's Grok Imagine model via fal.ai
 * and sends them to messaging channels via ZeroClaw.
 *
 * Usage:
 *   npx ts-node clawra-selfie.ts "<user_context>" "<channel>" ["<caption>"] ["<mode>"]
 *
 * Environment variables:
 *   FAL_KEY                - Your fal.ai API key
 *   ZEROCLAW_GATEWAY_URL   - ZeroClaw gateway URL (default: http://localhost:8080)
 *   ZEROCLAW_GATEWAY_TOKEN - Gateway paired bearer token
 */

import { execFile } from "child_process";
import { promisify } from "util";

const execFileAsync = promisify(execFile);

// Types
interface GrokImagineEditInput {
  image_url: string;
  prompt: string;
  num_images?: number;
  output_format?: OutputFormat;
}

interface GrokImagineImage {
  url: string;
  content_type: string;
  file_name?: string;
  width: number;
  height: number;
}

interface GrokImagineResponse {
  images: GrokImagineImage[];
  revised_prompt?: string;
}

interface ZeroClawMessage {
  action: "send";
  channel: string;
  message: string;
  media?: string;
}

type OutputFormat = "jpeg" | "png" | "webp";

type SelfieMode = "mirror" | "direct" | "auto";

interface EditAndSendOptions {
  userContext: string;
  channel: string;
  caption?: string;
  mode?: SelfieMode;
  outputFormat?: OutputFormat;
}

interface Result {
  success: boolean;
  imageUrl: string;
  channel: string;
  userContext: string;
  mode: string;
  revisedPrompt?: string;
}

// Fixed reference image (Clawra character)
const REFERENCE_IMAGE =
  "https://cdn.jsdelivr.net/gh/SumeLabs/clawra@main/assets/clawra.png";

// Check for fal.ai client
let falClient: any;
try {
  const { fal } = require("@fal-ai/client");
  falClient = fal;
} catch {
  falClient = null;
}

/**
 * Detect selfie mode from keywords in user context
 */
function detectMode(userContext: string): "mirror" | "direct" {
  const mirrorKeywords =
    /outfit|wearing|clothes|dress|suit|fashion|full-body|mirror/i;
  const directKeywords =
    /cafe|restaurant|beach|park|city|close-up|portrait|face|eyes|smile/i;

  if (directKeywords.test(userContext)) return "direct";
  if (mirrorKeywords.test(userContext)) return "mirror";
  return "mirror"; // default
}

/**
 * Build the edit prompt based on mode
 */
function buildPrompt(userContext: string, mode: "mirror" | "direct"): string {
  if (mode === "direct") {
    return `a close-up selfie taken by herself at ${userContext}, direct eye contact with the camera, looking straight into the lens, eyes centered and clearly visible, not a mirror selfie, phone held at arm's length, face fully visible`;
  }
  return `make a pic of this person, but ${userContext}. the person is taking a mirror selfie`;
}

/**
 * Edit image using Grok Imagine via fal.ai
 */
async function editImage(
  input: GrokImagineEditInput
): Promise<GrokImagineResponse> {
  const falKey = process.env.FAL_KEY;

  if (!falKey) {
    throw new Error(
      "FAL_KEY environment variable not set. Get your key from https://fal.ai/dashboard/keys"
    );
  }

  // Use fal client if available
  if (falClient) {
    falClient.config({ credentials: falKey });

    const result = await falClient.subscribe("xai/grok-imagine-image/edit", {
      input: {
        image_url: input.image_url,
        prompt: input.prompt,
        num_images: input.num_images || 1,
        output_format: input.output_format || "jpeg",
      },
    });

    return result.data as GrokImagineResponse;
  }

  // Fallback to fetch
  const response = await fetch("https://fal.run/xai/grok-imagine-image/edit", {
    method: "POST",
    headers: {
      Authorization: `Key ${falKey}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify({
      image_url: input.image_url,
      prompt: input.prompt,
      num_images: input.num_images || 1,
      output_format: input.output_format || "jpeg",
    }),
  });

  if (!response.ok) {
    const error = await response.text();
    throw new Error(`Image edit failed: ${error}`);
  }

  return response.json();
}

/**
 * Send image via ZeroClaw
 *
 * ZeroClaw supports two approaches:
 * 1. CLI: zeroclaw agent -m "[IMAGE:url] caption"
 * 2. Gateway API: POST /webhook with bearer token (pairing-based auth)
 */
async function sendViaZeroClaw(
  message: ZeroClawMessage,
  useCLI: boolean = true
): Promise<void> {
  if (useCLI) {
    try {
      // ZeroClaw uses [IMAGE:url] markers in agent messages
      const text = `[IMAGE:${message.media}] ${message.message}`;
      await execFileAsync("zeroclaw", ["agent", "-m", text]);
      return;
    } catch {
      console.log("[WARN] CLI send failed, falling back to API...");
    }
  }

  // Direct API call to ZeroClaw gateway (port 8080, pairing-based auth)
  const gatewayUrl =
    process.env.ZEROCLAW_GATEWAY_URL || "http://localhost:8080";
  const gatewayToken = process.env.ZEROCLAW_GATEWAY_TOKEN;

  if (!gatewayToken) {
    throw new Error(
      "ZEROCLAW_GATEWAY_TOKEN not set. Pair with: POST /pair on gateway"
    );
  }

  const headers: Record<string, string> = {
    "Content-Type": "application/json",
    Authorization: `Bearer ${gatewayToken}`,
  };

  const response = await fetch(`${gatewayUrl}/webhook`, {
    method: "POST",
    headers,
    body: JSON.stringify(message),
  });

  if (!response.ok) {
    const error = await response.text();
    throw new Error(`ZeroClaw send failed: ${error}`);
  }
}

/**
 * Main function: Edit reference image and send selfie
 */
async function editAndSend(options: EditAndSendOptions): Promise<Result> {
  const {
    userContext,
    channel,
    caption = "Edited with Grok Imagine",
    mode = "auto",
    outputFormat = "jpeg",
  } = options;

  // Determine mode
  const actualMode = mode === "auto" ? detectMode(userContext) : mode;
  console.log(`[INFO] Mode: ${actualMode}`);

  // Build prompt
  const editPrompt = buildPrompt(userContext, actualMode);
  console.log(`[INFO] Editing reference image...`);

  // Edit image
  const imageResult = await editImage({
    image_url: REFERENCE_IMAGE,
    prompt: editPrompt,
    num_images: 1,
    output_format: outputFormat,
  });

  const imageUrl = imageResult.images[0].url;
  console.log(`[INFO] Image edited: ${imageUrl}`);

  if (imageResult.revised_prompt) {
    console.log(`[INFO] Revised prompt: ${imageResult.revised_prompt}`);
  }

  // Send via ZeroClaw
  console.log(`[INFO] Sending to channel: ${channel}`);

  await sendViaZeroClaw({
    action: "send",
    channel,
    message: caption,
    media: imageUrl,
  });

  console.log(`[INFO] Done! Image sent to ${channel}`);

  return {
    success: true,
    imageUrl,
    channel,
    userContext,
    mode: actualMode,
    revisedPrompt: imageResult.revised_prompt,
  };
}

// CLI entry point
async function main() {
  const args = process.argv.slice(2);

  if (args.length < 2) {
    console.log(`
Usage: npx ts-node clawra-selfie.ts <user_context> <channel> [caption] [mode] [output_format]

Arguments:
  user_context  - What the person should be doing/wearing/where (required)
  channel       - Target channel (required) e.g., #general, @user
  caption       - Message caption (default: 'Edited with Grok Imagine')
  mode          - Selfie mode: mirror, direct, auto (default: auto)
  output_format - Image format: jpeg, png, webp (default: jpeg)

Environment:
  FAL_KEY                - Your fal.ai API key (required)
  ZEROCLAW_GATEWAY_TOKEN - ZeroClaw gateway paired token (for API mode)

Example:
  FAL_KEY=key npx ts-node clawra-selfie.ts "wearing a cowboy hat" "#general" "Howdy!" mirror
`);
    process.exit(1);
  }

  const [userContext, channel, caption, mode, outputFormat] = args;

  try {
    const result = await editAndSend({
      userContext,
      channel,
      caption,
      mode: (mode as SelfieMode) || "auto",
      outputFormat: (outputFormat as OutputFormat) || "jpeg",
    });

    console.log("\n--- Result ---");
    console.log(JSON.stringify(result, null, 2));
  } catch (error) {
    console.error(`[ERROR] ${(error as Error).message}`);
    process.exit(1);
  }
}

// Export for module use
export {
  editImage,
  sendViaZeroClaw,
  editAndSend,
  detectMode,
  buildPrompt,
  GrokImagineEditInput,
  GrokImagineResponse,
  ZeroClawMessage,
  EditAndSendOptions,
  Result,
};

// Run if executed directly
if (require.main === module) {
  main();
}
