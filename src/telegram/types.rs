use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct TelegramResponse<T> {
    pub ok: bool,
    pub result: Option<T>,
    pub description: Option<String>,
}

#[derive(Deserialize)]
pub struct Update {
    pub update_id: i64,
    pub message: Option<TgMessage>,
    pub pre_checkout_query: Option<PreCheckoutQuery>,
}

#[derive(Deserialize)]
pub struct TgMessage {
    pub message_id: i64,
    pub chat: TgChat,
    pub text: Option<String>,
    pub from: Option<TgUser>,
    pub successful_payment: Option<SuccessfulPayment>,
    #[serde(default)]
    pub reply_to_message: Option<Box<TgReplyMessage>>,
}

#[derive(Deserialize)]
pub struct TgReplyMessage {
    pub from: Option<TgUser>,
}

#[derive(Deserialize)]
pub struct TgChat {
    pub id: i64,
    #[serde(rename = "type")]
    pub chat_type: Option<String>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
pub struct TgUser {
    #[serde(default)]
    pub id: i64,
    pub first_name: String,
    #[serde(default)]
    pub is_bot: bool,
}

#[derive(Deserialize)]
pub struct PreCheckoutQuery {
    pub id: String,
}

#[derive(Deserialize)]
pub struct SuccessfulPayment {
    pub invoice_payload: String,
}

#[derive(Serialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMsg>,
    pub max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
}

#[derive(Serialize, Clone)]
pub struct ChatMsg {
    pub role: String,
    pub content: String,
}

#[derive(Deserialize)]
pub struct ChatResponse {
    pub choices: Option<Vec<ChatChoice>>,
    pub error: Option<ApiError>,
}

#[derive(Deserialize)]
pub struct ChatChoice {
    pub message: ChatChoiceMsg,
}

#[derive(Deserialize)]
pub struct ChatChoiceMsg {
    pub content: Option<String>,
}

#[derive(Deserialize)]
pub struct ApiError {
    pub message: String,
}
