use std::time::Duration;

#[derive(Debug, Clone)]
pub enum ObserverEvent {
    AgentStart {
        provider: String,
        model: String,
    },
    LlmRequest {
        provider: String,
        model: String,
        messages_count: usize,
    },
    LlmResponse {
        provider: String,
        model: String,
        duration: Duration,
        success: bool,
        error_message: Option<String>,
    },
    AgentEnd {
        provider: String,
        model: String,
        duration: Duration,
        tokens_used: Option<u64>,
        cost_usd: Option<f64>,
    },
    ToolCallStart {
        tool: String,
    },
    ToolCall {
        tool: String,
        duration: Duration,
        success: bool,
    },
    TurnComplete,
    ChannelMessage {
        channel: String,
        direction: String,
    },
    HeartbeatTick,
    Error {
        component: String,
        message: String,
    },
    SurvivalTierChange {
        from: String,
        to: String,
        balance_cents: i64,
    },
    CosmicBrainTick {
        free_energy: f64,
        drift: f64,
        integration: f64,
    },
}

#[derive(Debug, Clone)]
pub enum ObserverMetric {
    RequestLatency(Duration),
    TokensUsed(u64),
    ActiveSessions(u64),
    QueueDepth(u64),
}

pub trait Observer: Send + Sync + 'static {
    fn record_event(&self, event: &ObserverEvent);
    fn record_metric(&self, metric: &ObserverMetric);
    fn flush(&self) {}
    fn name(&self) -> &str;
    fn as_any(&self) -> &dyn std::any::Any;
}
