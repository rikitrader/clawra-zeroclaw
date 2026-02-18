use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::process::Command;

const REFERENCE_IMAGE: &str =
    "https://imgix.ranker.com/user_node_img/50149/1002963598/original/1002963598-photo-u220763866";

const DEFAULT_IMAGE_MODEL: &str = "google/gemini-2.5-flash-image";

// ── OpenRouter API types ─────────────────────────────────────────

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
}

#[derive(Serialize)]
struct Message {
    role: String,
    content: Vec<ContentPart>,
}

#[derive(Serialize)]
#[serde(tag = "type")]
enum ContentPart {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image_url")]
    ImageUrl { image_url: ImageUrlRef },
}

#[derive(Serialize)]
struct ImageUrlRef {
    url: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Deserialize)]
struct ResponseMessage {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    images: Vec<ResponseImage>,
}

#[derive(Deserialize)]
struct ResponseImage {
    image_url: ImageUrlData,
}

#[derive(Deserialize)]
struct ImageUrlData {
    url: String,
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
            "Edit this photo: create a close-up selfie of this exact same person at {context}. \
             Keep her exact face, hair, and features identical. \
             She is taking the selfie herself with her phone, direct eye contact with the camera, \
             looking straight into the lens, face fully visible. \
             Photorealistic, natural lighting."
        ),
        _ => format!(
            "Edit this photo: create a mirror selfie of this exact same person, but {context}. \
             Keep her exact face, hair, and features identical. \
             She is taking a mirror selfie with her phone visible in the reflection. \
             Photorealistic, natural lighting."
        ),
    }
}

/// Decode base64 data URI and save to a temp file, return the file path
fn save_base64_image(data_uri: &str) -> Result<String, String> {
    // data:image/png;base64,iVBOR...
    let base64_data = data_uri
        .split(",")
        .nth(1)
        .ok_or("Invalid data URI: no base64 payload")?;

    let ext = if data_uri.contains("image/png") {
        "png"
    } else if data_uri.contains("image/webp") {
        "webp"
    } else {
        "jpg"
    };

    use std::io::Write;
    let decoded = base64_decode(base64_data)?;
    let path = format!("/tmp/jenni-selfie-{}.{}", std::process::id(), ext);
    let mut file = fs::File::create(&path).map_err(|e| format!("Failed to create temp file: {e}"))?;
    file.write_all(&decoded)
        .map_err(|e| format!("Failed to write image: {e}"))?;

    Ok(path)
}

/// Simple base64 decoder (no external deps)
fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut lookup = [255u8; 256];
    for (i, &c) in TABLE.iter().enumerate() {
        lookup[c as usize] = i as u8;
    }

    let clean: Vec<u8> = input.bytes().filter(|b| !b.is_ascii_whitespace() && *b != b'=').collect();
    let mut output = Vec::with_capacity(clean.len() * 3 / 4);

    for chunk in clean.chunks(4) {
        let mut buf = [0u8; 4];
        for (i, &b) in chunk.iter().enumerate() {
            buf[i] = lookup[b as usize];
            if buf[i] == 255 {
                return Err(format!("Invalid base64 character: {}", b as char));
            }
        }
        output.push((buf[0] << 2) | (buf[1] >> 4));
        if chunk.len() > 2 {
            output.push((buf[1] << 4) | (buf[2] >> 2));
        }
        if chunk.len() > 3 {
            output.push((buf[2] << 6) | buf[3]);
        }
    }

    Ok(output)
}

/// Generate selfie and send via ZeroClaw
pub fn run(
    context: &str,
    channel: &str,
    mode: &str,
    caption: &str,
    _format: &str,
) -> Result<(), String> {
    let api_key = env::var("OPENROUTER_API_KEY")
        .or_else(|_| env::var("OPENROUTER_KEY"))
        .map_err(|_| "OPENROUTER_API_KEY not set")?;

    let image_model =
        env::var("SELFIE_MODEL").unwrap_or_else(|_| DEFAULT_IMAGE_MODEL.to_string());

    // Resolve mode
    let actual_mode = if mode == "auto" {
        detect_mode(context)
    } else {
        mode
    };

    println!("\x1b[32m[INFO]\x1b[0m Mode: {actual_mode}");
    println!("\x1b[32m[INFO]\x1b[0m Model: {image_model}");

    let prompt = build_prompt(context, actual_mode);
    println!("\x1b[32m[INFO]\x1b[0m Generating selfie via OpenRouter...");

    // Call OpenRouter chat completions with image model
    let request = ChatRequest {
        model: image_model,
        messages: vec![Message {
            role: "user".to_string(),
            content: vec![
                ContentPart::Text { text: prompt },
                ContentPart::ImageUrl {
                    image_url: ImageUrlRef {
                        url: REFERENCE_IMAGE.to_string(),
                    },
                },
            ],
        }],
    };

    let response = ureq::post("https://openrouter.ai/api/v1/chat/completions")
        .set("Authorization", &format!("Bearer {api_key}"))
        .set("Content-Type", "application/json")
        .send_json(serde_json::to_value(&request).unwrap())
        .map_err(|e| format!("OpenRouter API error: {e}"))?;

    let result: ChatResponse = response
        .into_json()
        .map_err(|e| format!("Failed to parse response: {e}"))?;

    let choice = result
        .choices
        .first()
        .ok_or("No choices in API response")?;

    // Extract image from response.message.images[]
    let image = choice
        .message
        .images
        .first()
        .ok_or("No image generated. The model may not support image output.")?;

    let data_uri = &image.image_url.url;

    if !data_uri.starts_with("data:image") {
        return Err("Unexpected image format from API".to_string());
    }

    println!("\x1b[32m[INFO]\x1b[0m Image generated ({} bytes base64)", data_uri.len());

    if let Some(text) = &choice.message.content {
        if !text.is_empty() {
            println!("\x1b[32m[INFO]\x1b[0m Model said: {text}");
        }
    }

    // Save base64 image to temp file
    let image_path = save_base64_image(data_uri)?;
    println!("\x1b[32m[INFO]\x1b[0m Saved to: {image_path}");

    // Send via Telegram Bot API directly (base64 images need file upload)
    println!("\x1b[32m[INFO]\x1b[0m Sending to channel: {channel}");

    let bot_token = env::var("TELEGRAM_BOT_TOKEN").ok();
    let chat_id = env::var("TELEGRAM_CHAT_ID").ok();

    if let (Some(token), Some(cid)) = (bot_token, chat_id) {
        // Direct Telegram sendPhoto with file upload
        let url = format!("https://api.telegram.org/bot{token}/sendPhoto");
        // Use curl for multipart upload (ureq doesn't support multipart natively)
        let curl_result = Command::new("curl")
            .args([
                "-s",
                "-X", "POST",
                &url,
                "-F", &format!("chat_id={cid}"),
                "-F", &format!("photo=@{image_path}"),
                "-F", &format!("caption={caption}"),
            ])
            .output();

        match curl_result {
            Ok(output) if output.status.success() => {
                println!("\x1b[32m[INFO]\x1b[0m Sent via Telegram Bot API");
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                println!("\x1b[33m[WARN]\x1b[0m Telegram send issue: {stderr}");
            }
            Err(e) => {
                println!("\x1b[33m[WARN]\x1b[0m curl failed: {e}");
            }
        }
    } else {
        // Fallback: try ZeroClaw CLI with file path
        let marker = format!("[IMAGE:file://{image_path}] {caption}");
        let cli_result = Command::new("zeroclaw")
            .args(["agent", "-m", &marker])
            .output();

        match cli_result {
            Ok(output) if output.status.success() => {
                println!("\x1b[32m[INFO]\x1b[0m Sent via ZeroClaw CLI");
            }
            _ => {
                println!("\x1b[33m[WARN]\x1b[0m Could not send automatically.");
                println!("\x1b[33m[WARN]\x1b[0m Image saved at: {image_path}");
                println!("\x1b[33m[WARN]\x1b[0m Set TELEGRAM_BOT_TOKEN and TELEGRAM_CHAT_ID for direct sending.");
            }
        }
    }

    // Print result JSON
    let result_json = serde_json::json!({
        "success": true,
        "image_path": image_path,
        "channel": channel,
        "context": context,
        "mode": actual_mode,
    });

    println!("\n--- Result ---");
    println!("{}", serde_json::to_string_pretty(&result_json).unwrap());

    Ok(())
}
