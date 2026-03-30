use reqwest::Client;

use super::types::{TelegramResponse, Update};

pub async fn get_updates(
    client: &Client,
    bot_token: &str,
    offset: i64,
) -> Result<Vec<Update>, String> {
    let url = format!(
        "https://api.telegram.org/bot{bot_token}/getUpdates?offset={offset}&timeout=30&allowed_updates=[\"message\",\"pre_checkout_query\"]"
    );

    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Telegram poll failed: {e}"))?;

    let body: TelegramResponse<Vec<Update>> = resp
        .json()
        .await
        .map_err(|e| format!("Parse error: {e}"))?;

    if !body.ok {
        return Err(format!(
            "Telegram API error: {}",
            body.description.unwrap_or_default()
        ));
    }

    Ok(body.result.unwrap_or_default())
}

pub async fn answer_pre_checkout(client: &Client, token: &str, query_id: &str) {
    let url = format!("https://api.telegram.org/bot{token}/answerPreCheckoutQuery");
    let body = serde_json::json!({
        "pre_checkout_query_id": query_id,
        "ok": true,
    });
    if let Err(e) = client.post(&url).json(&body).send().await {
        eprintln!("\x1b[31m[ERR]\x1b[0m Failed to answer pre-checkout: {e}");
    }
}

pub async fn send_star_invoice(client: &Client, token: &str, chat_id: i64, stars: u32) {
    let url = format!("https://api.telegram.org/bot{token}/sendInvoice");
    let body = serde_json::json!({
        "chat_id": chat_id,
        "title": "Unlock My Photo",
        "description": "Get the full unblurred photo just for you",
        "payload": format!("selfie_{chat_id}"),
        "provider_token": "",
        "currency": "XTR",
        "prices": [{"label": "Exclusive Photo", "amount": stars}],
    });

    match client.post(&url).json(&body).send().await {
        Ok(resp) => {
            let status = resp.status();
            if !status.is_success() {
                let text = resp.text().await.unwrap_or_default();
                eprintln!("\x1b[31m[ERR]\x1b[0m Invoice send failed ({status}): {text}");
            } else {
                eprintln!("\x1b[35m[PAY]\x1b[0m Invoice sent: {stars} Stars");
            }
        }
        Err(e) => {
            eprintln!("\x1b[31m[ERR]\x1b[0m Failed to send invoice: {e}");
        }
    }
}

pub async fn send_typing(client: &Client, token: &str, chat_id: i64) {
    let url = format!("https://api.telegram.org/bot{token}/sendChatAction");
    let body = serde_json::json!({ "chat_id": chat_id, "action": "typing" });
    let _ = client.post(&url).json(&body).send().await;
}

async fn human_delay(text: &str) {
    let words = text.split_whitespace().count();
    let base_ms = 800 + (words as u64 * 120).min(3000);
    let jitter = (text.len() as u64 * 7) % 500;
    tokio::time::sleep(std::time::Duration::from_millis(base_ms + jitter)).await;
}

pub async fn send_message(
    client: &Client,
    token: &str,
    chat_id: i64,
    text: &str,
    reply_to: i64,
) {
    send_typing(client, token, chat_id).await;
    human_delay(text).await;

    let url = format!("https://api.telegram.org/bot{token}/sendMessage");
    let body = serde_json::json!({
        "chat_id": chat_id,
        "text": text,
        "reply_to_message_id": reply_to,
        "parse_mode": "Markdown",
    });

    if let Err(e) = client.post(&url).json(&body).send().await {
        eprintln!("\x1b[31m[ERR]\x1b[0m Failed to send reply: {e}");
    }
}

pub async fn send_photo(
    client: &Client,
    token: &str,
    chat_id: i64,
    image_path: &str,
    caption: &str,
    _reply_to: i64,
) {
    let action_url = format!("https://api.telegram.org/bot{token}/sendChatAction");
    let action_body = serde_json::json!({ "chat_id": chat_id, "action": "upload_photo" });
    let _ = client.post(&action_url).json(&action_body).send().await;
    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;

    let url = format!("https://api.telegram.org/bot{token}/sendPhoto");
    let mut args = vec![
        "-s".to_string(),
        "-X".to_string(),
        "POST".to_string(),
        url,
        "-F".to_string(),
        format!("chat_id={chat_id}"),
        "-F".to_string(),
        format!("photo=@{image_path}"),
    ];
    if !caption.is_empty() {
        args.push("-F".to_string());
        args.push(format!("caption={caption}"));
    }
    let output = tokio::process::Command::new("curl")
        .args(&args)
        .output()
        .await;

    match output {
        Ok(o) if o.status.success() => {
            eprintln!("\x1b[32m[SELFIE]\x1b[0m Photo sent via Telegram");
        }
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr);
            eprintln!("\x1b[31m[ERR]\x1b[0m Photo send failed: {stderr}");
        }
        Err(e) => {
            eprintln!("\x1b[31m[ERR]\x1b[0m curl failed: {e}");
        }
    }
}
