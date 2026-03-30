use reqwest::Client;

use clawra_zeroclaw::paywall::{blur_image, PaywallState};
use clawra_zeroclaw::telegram::client::{send_message, send_photo, send_star_invoice};

const DEFAULT_SELFIE_SCENES: &[&str] = &[
    "at home relaxing in a cozy outfit",
    "at a trendy coffee shop with a latte",
    "getting ready for the day looking cute",
    "chilling on the couch in casual clothes",
    "in front of a mirror showing off today's look",
    "at the gym after a workout",
    "at a rooftop with city views",
    "in a cute sundress at a park",
];

pub fn extract_selfie_context(text: &str) -> String {
    let lower = text.to_lowercase();
    let stop_words = [
        "send", "me", "a", "pic", "photo", "foto", "selfie", "manda",
        "una", "envia", "envía", "de", "you", "of", "y", "la", "si",
        "please", "por", "favor",
    ];
    let cleaned: String = lower
        .split_whitespace()
        .filter(|w| {
            let word = w.trim_matches(|c: char| !c.is_alphanumeric());
            !stop_words.contains(&word)
        })
        .collect::<Vec<_>>()
        .join(" ");
    let cleaned = cleaned.trim().to_string();

    if cleaned.len() < 5 {
        let idx = (text.len() * 7) % DEFAULT_SELFIE_SCENES.len();
        DEFAULT_SELFIE_SCENES[idx].to_string()
    } else {
        text.to_string()
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn handle_selfie(
    client: &Client,
    bot_token: &str,
    chat_id: i64,
    message_id: i64,
    text: &str,
    api_key: &str,
    ack_phrases: &[String],
    paywall: &mut PaywallState,
) {
    eprintln!("\x1b[33m[SELFIE]\x1b[0m Selfie request detected, generating...");

    let ack = &ack_phrases[message_id as usize % ack_phrases.len()];
    send_message(client, bot_token, chat_id, ack, message_id).await;

    let api_key_clone = api_key.to_string();
    let context = extract_selfie_context(text);
    let image_model = std::env::var("SELFIE_MODEL")
        .unwrap_or_else(|_| "google/gemini-2.5-flash-image".to_string());

    let selfie_future = tokio::task::spawn_blocking(move || {
        let mode = crate::selfie::detect_mode(&context);
        let prompt = crate::selfie::build_prompt(&context, mode);
        eprintln!("\x1b[33m[SELFIE]\x1b[0m Mode: {mode}, generating image...");
        let (data_uri, used_model, _text) =
            crate::selfie::generate_image(&api_key_clone, &prompt, &image_model)?;
        eprintln!("\x1b[33m[SELFIE]\x1b[0m Generated via {used_model}");
        let path = crate::selfie::save_base64_image(&data_uri)?;
        Ok::<String, String>(path)
    });

    let selfie_result = tokio::time::timeout(
        std::time::Duration::from_secs(240),
        selfie_future,
    )
    .await;

    match selfie_result {
        Ok(Ok(Ok(image_path))) => {
            if paywall.has_free_images(chat_id) {
                let remaining = paywall.free_remaining(chat_id) - 1;
                eprintln!("\x1b[33m[SELFIE]\x1b[0m FREE image ({remaining} remaining)");
                paywall.use_free_image(chat_id);
                send_photo(client, bot_token, chat_id, &image_path, "", message_id).await;
            } else {
                eprintln!("\x1b[35m[PAY]\x1b[0m No free images -- sending blurred + invoice");

                match blur_image(&image_path) {
                    Ok(blurred_path) => {
                        paywall.store_pending(chat_id, &image_path);
                        send_photo(
                            client,
                            bot_token,
                            chat_id,
                            &blurred_path,
                            "",
                            message_id,
                        )
                        .await;
                        send_star_invoice(
                            client,
                            bot_token,
                            chat_id,
                            PaywallState::star_price(),
                        )
                        .await;
                    }
                    Err(e) => {
                        eprintln!("\x1b[31m[ERR]\x1b[0m Blur failed: {e}");
                        send_message(
                            client,
                            bot_token,
                            chat_id,
                            "Oops, camera glitch! Try again!",
                            message_id,
                        )
                        .await;
                    }
                }
            }
        }
        Ok(Ok(Err(e))) => {
            eprintln!("\x1b[31m[ERR]\x1b[0m Selfie generation failed: {e}");
            send_message(
                client,
                bot_token,
                chat_id,
                "Ugh, my camera glitched! Try a different scene?",
                message_id,
            )
            .await;
        }
        Ok(Err(e)) => {
            eprintln!("\x1b[31m[ERR]\x1b[0m Selfie task panicked: {e}");
            send_message(
                client,
                bot_token,
                chat_id,
                "Camera crashed! Try again!",
                message_id,
            )
            .await;
        }
        Err(_) => {
            eprintln!("\x1b[31m[ERR]\x1b[0m Selfie generation timed out");
            send_message(
                client,
                bot_token,
                chat_id,
                "That took too long! Try something simpler?",
                message_id,
            )
            .await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_selfie_context_short_input_returns_default_scene() {
        let result = extract_selfie_context("send pic");
        assert!(
            DEFAULT_SELFIE_SCENES.contains(&result.as_str()),
            "Expected a default scene, got: {result}"
        );
    }

    #[test]
    fn extract_selfie_context_preserves_meaningful_content() {
        let result = extract_selfie_context("send me a pic wearing a red dress at the beach");
        assert_eq!(result, "send me a pic wearing a red dress at the beach");
    }

    #[test]
    fn extract_selfie_context_empty_after_stop_words() {
        let result = extract_selfie_context("send me a photo please");
        assert!(
            DEFAULT_SELFIE_SCENES.contains(&result.as_str()),
            "All stop words should yield default scene, got: {result}"
        );
    }

    #[test]
    fn extract_selfie_context_spanish_stop_words() {
        let result = extract_selfie_context("manda una foto por favor");
        assert!(
            DEFAULT_SELFIE_SCENES.contains(&result.as_str()),
            "Spanish stop words should yield default scene, got: {result}"
        );
    }

    #[test]
    fn extract_selfie_context_with_context_keywords() {
        let result = extract_selfie_context("selfie wearing leather jacket at night");
        assert!(result.contains("leather jacket"));
    }

    #[test]
    fn extract_selfie_context_deterministic_for_same_input() {
        let r1 = extract_selfie_context("pic");
        let r2 = extract_selfie_context("pic");
        assert_eq!(r1, r2, "Same input should produce same default scene");
    }
}
