use serde::{Deserialize, Serialize};
use std::env;
use std::process::Command;

const REFERENCE_IMAGE: &str =
    "https://cdn.jsdelivr.net/gh/SumeLabs/clawra@main/assets/clawra.png";

#[derive(Serialize)]
struct EditRequest {
    image_url: String,
    prompt: String,
    num_images: u8,
    output_format: String,
}

#[derive(Deserialize)]
struct EditResponse {
    images: Vec<ImageResult>,
    revised_prompt: Option<String>,
}

#[derive(Deserialize)]
struct ImageResult {
    url: String,
}

#[derive(Serialize)]
struct WebhookPayload {
    action: String,
    channel: String,
    message: String,
    media: String,
}

/// Detect selfie mode from keywords
fn detect_mode(context: &str) -> &'static str {
    let lower = context.to_lowercase();

    let direct_keywords = [
        "cafe",
        "restaurant",
        "beach",
        "park",
        "city",
        "close-up",
        "portrait",
        "face",
        "eyes",
        "smile",
    ];
    let mirror_keywords = [
        "outfit", "wearing", "clothes", "dress", "suit", "fashion", "full-body", "mirror",
    ];

    for kw in &direct_keywords {
        if lower.contains(kw) {
            return "direct";
        }
    }
    for kw in &mirror_keywords {
        if lower.contains(kw) {
            return "mirror";
        }
    }

    "mirror" // default
}

/// Build the edit prompt based on mode
fn build_prompt(context: &str, mode: &str) -> String {
    match mode {
        "direct" => format!(
            "a close-up selfie taken by herself at {context}, \
             direct eye contact with the camera, looking straight into the lens, \
             eyes centered and clearly visible, not a mirror selfie, \
             phone held at arm's length, face fully visible"
        ),
        _ => format!(
            "make a pic of this person, but {context}. \
             the person is taking a mirror selfie"
        ),
    }
}

/// Generate selfie and send via ZeroClaw
pub fn run(
    context: &str,
    channel: &str,
    mode: &str,
    caption: &str,
    format: &str,
) -> Result<(), String> {
    let fal_key = env::var("FAL_KEY")
        .map_err(|_| "FAL_KEY not set. Get your key from https://fal.ai/dashboard/keys")?;

    // Resolve mode
    let actual_mode = if mode == "auto" {
        detect_mode(context)
    } else {
        mode
    };

    println!("\x1b[32m[INFO]\x1b[0m Mode: {actual_mode}");

    let prompt = build_prompt(context, actual_mode);
    println!("\x1b[32m[INFO]\x1b[0m Editing reference image...");

    // Call fal.ai Grok Imagine Edit API
    let request = EditRequest {
        image_url: REFERENCE_IMAGE.to_string(),
        prompt,
        num_images: 1,
        output_format: format.to_string(),
    };

    let response = ureq::post("https://fal.run/xai/grok-imagine-image/edit")
        .set("Authorization", &format!("Key {fal_key}"))
        .set("Content-Type", "application/json")
        .send_json(serde_json::to_value(&request).unwrap())
        .map_err(|e| format!("fal.ai API error: {e}"))?;

    let result: EditResponse = response
        .into_json()
        .map_err(|e| format!("Failed to parse response: {e}"))?;

    let image_url = result
        .images
        .first()
        .map(|img| img.url.as_str())
        .ok_or("No image returned from API")?;

    println!("\x1b[32m[INFO]\x1b[0m Image edited: {image_url}");

    if let Some(revised) = &result.revised_prompt {
        println!("\x1b[32m[INFO]\x1b[0m Revised prompt: {revised}");
    }

    // Send via ZeroClaw
    println!("\x1b[32m[INFO]\x1b[0m Sending to channel: {channel}");

    // Try CLI first: zeroclaw agent -m "[IMAGE:url] caption"
    let marker = format!("[IMAGE:{image_url}] {caption}");
    let cli_result = Command::new("zeroclaw")
        .args(["agent", "-m", &marker])
        .output();

    match cli_result {
        Ok(output) if output.status.success() => {
            println!("\x1b[32m[INFO]\x1b[0m Sent via ZeroClaw CLI");
        }
        _ => {
            // Fallback to gateway API
            println!("\x1b[33m[WARN]\x1b[0m CLI unavailable, using gateway API...");

            let gateway_url = env::var("ZEROCLAW_GATEWAY_URL")
                .unwrap_or_else(|_| "http://localhost:8080".to_string());
            let gateway_token = env::var("ZEROCLAW_GATEWAY_TOKEN")
                .map_err(|_| "ZEROCLAW_GATEWAY_TOKEN not set. Pair with POST /pair on gateway")?;

            let payload = WebhookPayload {
                action: "send".to_string(),
                channel: channel.to_string(),
                message: caption.to_string(),
                media: image_url.to_string(),
            };

            ureq::post(&format!("{gateway_url}/webhook"))
                .set("Authorization", &format!("Bearer {gateway_token}"))
                .set("Content-Type", "application/json")
                .send_json(serde_json::to_value(&payload).unwrap())
                .map_err(|e| format!("Gateway send failed: {e}"))?;

            println!("\x1b[32m[INFO]\x1b[0m Sent via gateway API");
        }
    }

    // Print result JSON
    let result_json = serde_json::json!({
        "success": true,
        "image_url": image_url,
        "channel": channel,
        "context": context,
        "mode": actual_mode,
    });

    println!("\n--- Result ---");
    println!("{}", serde_json::to_string_pretty(&result_json).unwrap());

    Ok(())
}
