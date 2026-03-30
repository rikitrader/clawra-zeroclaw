use clawra_zeroclaw::cognitive::CognitiveEngine;
use clawra_zeroclaw::config::IdentityConfig;
use clawra_zeroclaw::consciousness::soul::constitution::Constitution;
use clawra_zeroclaw::consciousness::soul::model::SoulModel;
use clawra_zeroclaw::consciousness::soul::parser::parse_soul_file;
use clawra_zeroclaw::identity::{aieos_to_system_prompt, load_aieos_identity};
use clawra_zeroclaw::paywall::PaywallState;
use clawra_zeroclaw::search;
use reqwest::Client;
use std::path::Path;

use clawra_zeroclaw::telegram::client::{
    answer_pre_checkout, get_updates, send_message, send_photo, send_typing,
};
use clawra_zeroclaw::telegram::types::{ChatMsg, ChatRequest, ChatResponse};

use super::selfie_handler::handle_selfie;

const SOUL_INJECTION: &str = include_str!("../../templates/soul-injection.md");
const DEFAULT_CHAT_MODEL: &str = "google/gemini-2.0-flash-001";

const SELFIE_KEYWORDS: &[&str] = &[
    "selfie", "selfi", "photo", "foto", "pic", "picture", "imagen",
    "send me a pic", "what do you look like", "show me", "send a photo",
    "take a pic", "what are you doing", "where are you", "what are you wearing",
    "send pic", "send foto", "manda foto", "send selfie",
    "enviame", "envíame", "enviamela", "envíamela", "manda pic",
    "quiero ver", "dejame ver", "déjame ver", "muéstrame", "mostrame",
    "envia foto", "envía foto", "manda una foto", "manda selfie",
    "si envia", "si manda", "yes send", "send it",
];

pub struct PersonaConfig {
    pub bot_username: String,
    pub persona_names: Vec<String>,
    pub welcome_message: String,
    pub ack_phrases: Vec<String>,
}

impl Default for PersonaConfig {
    fn default() -> Self {
        Self {
            bot_username: "clawrabot".to_string(),
            persona_names: vec!["clawra".to_string()],
            welcome_message: "Hey! I'm here. Ask me anything, or ask for a selfie!".to_string(),
            ack_phrases: vec![
                "Sure, give me a sec...".to_string(),
                "One moment...".to_string(),
                "Coming right up...".to_string(),
                "Let me get that for you...".to_string(),
            ],
        }
    }
}

impl PersonaConfig {
    pub fn from_env_and_soul(soul: &SoulModel) -> Self {
        let bot_username = std::env::var("CLAWRA_BOT_USERNAME")
            .unwrap_or_else(|_| "clawrabot".to_string())
            .to_lowercase();

        let persona_names: Vec<String> = std::env::var("CLAWRA_PERSONA_NAMES")
            .map(|s| s.split(',').map(|n| n.trim().to_lowercase()).collect())
            .unwrap_or_else(|_| {
                if !soul.name.is_empty() {
                    vec![soul.name.to_lowercase()]
                } else {
                    vec!["clawra".to_string()]
                }
            });

        let welcome_message = std::env::var("CLAWRA_WELCOME_MESSAGE").unwrap_or_else(|_| {
            if !soul.name.is_empty() {
                format!(
                    "Hey! I'm {}. Ask me anything, or ask for a selfie!",
                    soul.name
                )
            } else {
                "Hey! I'm here. Ask me anything, or ask for a selfie!".to_string()
            }
        });

        let ack_phrases: Vec<String> = std::env::var("CLAWRA_ACK_PHRASES")
            .map(|s| s.split('|').map(|p| p.trim().to_string()).collect())
            .unwrap_or_else(|_| {
                vec![
                    "Sure, give me a sec...".to_string(),
                    "One moment...".to_string(),
                    "Coming right up...".to_string(),
                    "Let me get that for you...".to_string(),
                ]
            });

        Self {
            bot_username,
            persona_names,
            welcome_message,
            ack_phrases,
        }
    }
}

fn build_system_prompt(
    soul: &SoulModel,
    constitution: &Constitution,
    cognitive_ctx: &str,
    identity_ctx: Option<&str>,
) -> String {
    let mut prompt = String::new();

    prompt.push_str(SOUL_INJECTION);
    prompt.push_str("\n\n");

    prompt.push_str(&constitution.to_prompt_section());
    prompt.push('\n');

    if !soul.name.is_empty() {
        prompt.push_str(&soul.to_prompt_section());
        prompt.push('\n');
    }

    if let Some(identity) = identity_ctx {
        prompt.push_str("\n# AIEOS Identity\n\n");
        prompt.push_str(identity);
        prompt.push('\n');
    }

    if !cognitive_ctx.is_empty() {
        prompt.push_str(cognitive_ctx);
        prompt.push('\n');
    }

    prompt.push_str(
        "IMPORTANT: You are in a TEXT-ONLY Telegram chat. You CANNOT generate images, call tools, \
         or use any functions. You have NO tool_code capability. Never output code blocks, function calls, \
         or JSON. Only respond with plain conversational text.\n\n\
         When someone asks for a selfie or picture, respond playfully in character — describe what you're \
         doing, tease them, flirt — but do NOT try to generate or send an actual image.\n\n\
         You are chatting on Telegram. Be yourself — playful, flirty, expressive, confident. \
         Keep responses concise (1-3 paragraphs max). You have a physical appearance and personality. \
         You are NOT an AI assistant — you are a real person with a life, opinions, and feelings. \
         Never break character. Never say you're an AI.\n\n\
         LANGUAGE RULE: Always reply in the SAME language the user writes in. \
         If they write in Spanish, reply in Spanish. If English, reply in English. \
         If Portuguese, reply in Portuguese. Match their language exactly.",
    );

    prompt
}

fn sanitize_reply(reply: &str) -> String {
    let mut result = reply.to_string();

    while let Some(start) = result.find("```") {
        if let Some(end) = result[start + 3..].find("```") {
            result = format!("{}{}", &result[..start], &result[start + 3 + end + 3..]);
        } else {
            result = result[..start].to_string();
            break;
        }
    }

    let blocked = [
        "tool_code",
        "tool_call",
        "function_call",
        "```json",
        "```python",
    ];
    for pattern in blocked {
        result = result.replace(pattern, "");
    }

    let result = result.trim().to_string();
    if result.is_empty() {
        "...".to_string()
    } else {
        result
    }
}

pub async fn run_dev(
    soul_path: Option<&str>,
    identity_path: Option<&str>,
) -> anyhow::Result<()> {
    let bot_token = std::env::var("TELEGRAM_BOT_TOKEN")
        .map_err(|_| anyhow::anyhow!("TELEGRAM_BOT_TOKEN not set"))?;
    let api_key = std::env::var("OPENROUTER_API_KEY")
        .or_else(|_| std::env::var("OPENROUTER_KEY"))
        .map_err(|_| anyhow::anyhow!("OPENROUTER_API_KEY not set"))?;
    let chat_model =
        std::env::var("CLAWRA_MODEL").unwrap_or_else(|_| DEFAULT_CHAT_MODEL.to_string());
    let brave_available = std::env::var("BRAVE_API_KEY").is_ok();
    if brave_available {
        eprintln!("\x1b[32m[CLAWRA]\x1b[0m Web search: enabled (Brave Search API)");
    } else {
        eprintln!("\x1b[32m[CLAWRA]\x1b[0m Web search: enabled (DuckDuckGo fallback)");
    }
    let allowed_chat_id: Option<i64> = std::env::var("TELEGRAM_CHAT_ID")
        .ok()
        .and_then(|s| s.parse().ok());

    let soul = soul_path
        .map(|p| parse_soul_file(Path::new(p)))
        .transpose()?
        .unwrap_or_default();

    let identity_prompt = if let Some(id_path) = identity_path {
        let config = IdentityConfig {
            format: "aieos".to_string(),
            aieos_path: Some(id_path.to_string()),
            ..Default::default()
        };
        let workspace = std::env::current_dir().unwrap_or_default();
        match load_aieos_identity(&config, &workspace) {
            Ok(Some(identity)) => {
                let text = aieos_to_system_prompt(&identity);
                if text.is_empty() {
                    None
                } else {
                    eprintln!(
                        "\x1b[32m[CLAWRA]\x1b[0m AIEOS identity loaded from: {id_path}"
                    );
                    Some(text)
                }
            }
            Ok(None) => None,
            Err(e) => {
                eprintln!(
                    "\x1b[33m[CLAWRA]\x1b[0m Failed to load AIEOS identity: {e}"
                );
                None
            }
        }
    } else {
        None
    };

    let persona = PersonaConfig::from_env_and_soul(&soul);
    let constitution = Constitution::default_laws();

    let mut engine = CognitiveEngine::new();
    let mut paywall = PaywallState::load();

    eprintln!("\x1b[32m[CLAWRA]\x1b[0m Starting dev mode...");
    eprintln!("\x1b[32m[CLAWRA]\x1b[0m Model: {chat_model}");
    eprintln!(
        "\x1b[32m[CLAWRA]\x1b[0m Constitution: {} laws loaded",
        constitution.laws().len()
    );
    eprintln!(
        "\x1b[32m[CLAWRA]\x1b[0m Paywall: {} free images, {} Stars per unlock",
        3,
        PaywallState::star_price()
    );
    eprintln!(
        "\x1b[32m[CLAWRA]\x1b[0m Bot username: @{}",
        persona.bot_username
    );
    eprintln!(
        "\x1b[32m[CLAWRA]\x1b[0m Persona names: {:?}",
        persona.persona_names
    );
    if !soul.name.is_empty() {
        eprintln!("\x1b[32m[CLAWRA]\x1b[0m Soul: {}", soul.name);
    }
    if identity_prompt.is_some() {
        eprintln!("\x1b[32m[CLAWRA]\x1b[0m AIEOS identity: active");
    }
    if let Some(cid) = allowed_chat_id {
        eprintln!("\x1b[32m[CLAWRA]\x1b[0m Restricted to chat_id: {cid}");
    } else {
        eprintln!("\x1b[33m[CLAWRA]\x1b[0m No TELEGRAM_CHAT_ID set -- responding to ALL chats");
    }
    eprintln!("\x1b[32m[CLAWRA]\x1b[0m Polling for messages... (Ctrl+C to stop)");

    let client = Client::new();
    let mut offset: i64 = 0;
    let mut conversation: Vec<ChatMsg> = Vec::new();
    let max_history = 20;

    loop {
        let updates = tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                eprintln!("\n\x1b[33m[CLAWRA]\x1b[0m Shutting down...");
                engine.save_state();
                paywall.save();
                eprintln!("\x1b[32m[CLAWRA]\x1b[0m Cognitive state saved.");
                eprintln!("\x1b[32m[CLAWRA]\x1b[0m Paywall state saved.");
                eprintln!("\x1b[32m[CLAWRA]\x1b[0m Goodbye.");
                return Ok(());
            }
            result = get_updates(&client, &bot_token, offset) => {
                match result {
                    Ok(u) => u,
                    Err(e) => {
                        eprintln!("\x1b[31m[ERR]\x1b[0m {e}");
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                        continue;
                    }
                }
            }
        };

        for update in updates {
            offset = update.update_id + 1;

            if let Some(pcq) = update.pre_checkout_query {
                eprintln!("\x1b[35m[PAY]\x1b[0m Pre-checkout query: {}", pcq.id);
                answer_pre_checkout(&client, &bot_token, &pcq.id).await;
                continue;
            }

            let msg = match update.message {
                Some(m) => m,
                None => continue,
            };

            if let Some(payment) = &msg.successful_payment {
                let payload = &payment.invoice_payload;
                eprintln!("\x1b[35m[PAY]\x1b[0m Payment received! Payload: {payload}");

                let chat_id: i64 = payload
                    .strip_prefix("selfie_")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(msg.chat.id);

                if let Some(clear_path) = paywall.take_pending(chat_id) {
                    eprintln!("\x1b[35m[PAY]\x1b[0m Sending unlocked photo: {clear_path}");
                    send_photo(
                        &client,
                        &bot_token,
                        msg.chat.id,
                        &clear_path,
                        "Here you go! Just for you",
                        msg.message_id,
                    )
                    .await;
                } else {
                    send_message(
                        &client,
                        &bot_token,
                        msg.chat.id,
                        "Payment received! But the photo expired. Ask me for a new one!",
                        msg.message_id,
                    )
                    .await;
                }
                continue;
            }

            let text = match msg.text {
                Some(t) => t,
                None => continue,
            };

            if let Some(allowed) = allowed_chat_id {
                if msg.chat.id != allowed {
                    continue;
                }
            }

            let sender = msg
                .from
                .as_ref()
                .map(|u| u.first_name.as_str())
                .unwrap_or("Someone");

            eprintln!("\x1b[36m[IN]\x1b[0m {sender}: {text}");

            let is_group = msg
                .chat
                .chat_type
                .as_deref()
                .map(|t| t == "group" || t == "supergroup")
                .unwrap_or(false);

            if is_group {
                let lower = text.to_lowercase();
                let bot_tag = format!("@{}", persona.bot_username);
                let is_tagged = lower.contains(&bot_tag);
                let is_named = persona
                    .persona_names
                    .iter()
                    .any(|name| lower.contains(name.as_str()));
                let is_reply_to_bot = msg
                    .reply_to_message
                    .as_ref()
                    .and_then(|r| r.from.as_ref())
                    .map(|u| u.is_bot)
                    .unwrap_or(false);
                let is_command = text.starts_with('/');

                if !is_tagged && !is_named && !is_reply_to_bot && !is_command {
                    continue;
                }
                eprintln!("\x1b[36m[GROUP]\x1b[0m Responding (tagged/named/replied)");
            }

            if text == "/start" {
                send_message(
                    &client,
                    &bot_token,
                    msg.chat.id,
                    &persona.welcome_message,
                    msg.message_id,
                )
                .await;
                continue;
            }

            if text == "/reset" {
                conversation.clear();
                send_message(
                    &client,
                    &bot_token,
                    msg.chat.id,
                    "Memory cleared. Fresh start.",
                    msg.message_id,
                )
                .await;
                continue;
            }

            let lower_text = text.to_lowercase();
            let is_selfie_request = SELFIE_KEYWORDS.iter().any(|kw| lower_text.contains(kw));

            if is_selfie_request {
                handle_selfie(
                    &client,
                    &bot_token,
                    msg.chat.id,
                    msg.message_id,
                    &text,
                    &api_key,
                    &persona.ack_phrases,
                    &mut paywall,
                )
                .await;
                continue;
            }

            conversation.push(ChatMsg {
                role: "user".to_string(),
                content: text.clone(),
            });

            if conversation.len() > max_history * 2 {
                conversation.drain(..conversation.len() - max_history * 2);
            }

            engine.pre_turn(&text);

            let mut search_context = String::new();
            if let Some(url) = search::extract_url(&text) {
                eprintln!("\x1b[36m[BROWSE]\x1b[0m Fetching: {url}");
                send_typing(&client, &bot_token, msg.chat.id).await;
                if let Some(content) = search::fetch_url(&client, &url).await {
                    eprintln!("\x1b[36m[BROWSE]\x1b[0m Got page content");
                    search_context = content;
                } else {
                    eprintln!("\x1b[33m[BROWSE]\x1b[0m Failed to fetch URL");
                }
            } else if search::needs_search(&text) {
                let query = search::extract_search_query(&text);
                eprintln!("\x1b[36m[SEARCH]\x1b[0m Searching: {query}");
                send_typing(&client, &bot_token, msg.chat.id).await;
                if let Some(results) = search::web_search(&client, &query).await {
                    eprintln!("\x1b[36m[SEARCH]\x1b[0m Got results, injecting into context");
                    search_context = results;
                } else {
                    eprintln!("\x1b[33m[SEARCH]\x1b[0m No results found");
                }
            }

            let cognitive_ctx = engine.cognitive_context();
            let mut system_prompt = build_system_prompt(
                &soul,
                &constitution,
                &cognitive_ctx,
                identity_prompt.as_deref(),
            );
            if !search_context.is_empty() {
                system_prompt.push_str("\n\nYou have access to real-time web search results. Use them to answer the user's question accurately. Cite sources naturally in your response. Stay in character while being informative.\n\n");
                system_prompt.push_str(&search_context);
            }
            let temperature = engine.effective_temperature(0.7);

            eprintln!(
                "\x1b[34m[COGNITIVE]\x1b[0m temp={:.2} | fe={:.2} | {}",
                temperature,
                engine.free_energy.free_energy(),
                cognitive_ctx.lines().take(4).collect::<Vec<_>>().join(" | ")
            );

            let mut messages = vec![ChatMsg {
                role: "system".to_string(),
                content: system_prompt,
            }];
            messages.extend(conversation.clone());

            let request = ChatRequest {
                model: chat_model.clone(),
                messages,
                max_tokens: 1024,
                temperature: Some(temperature),
            };

            let reply = match client
                .post("https://openrouter.ai/api/v1/chat/completions")
                .header("Authorization", format!("Bearer {api_key}"))
                .json(&request)
                .send()
                .await
            {
                Ok(r) => match r.json::<ChatResponse>().await {
                    Ok(cr) => {
                        if let Some(err) = cr.error {
                            format!("[API error: {}]", err.message)
                        } else {
                            cr.choices
                                .and_then(|c| c.into_iter().next())
                                .and_then(|c| c.message.content)
                                .unwrap_or_else(|| "[empty response]".to_string())
                        }
                    }
                    Err(e) => format!("[Parse error: {e}]"),
                },
                Err(e) => format!("[Request failed: {e}]"),
            };

            let reply = sanitize_reply(&reply);
            eprintln!("\x1b[35m[OUT]\x1b[0m {reply}");

            engine.process_turn(&text, &reply);

            conversation.push(ChatMsg {
                role: "assistant".to_string(),
                content: reply.clone(),
            });

            send_message(&client, &bot_token, msg.chat.id, &reply, msg.message_id).await;
        }
    }
}
