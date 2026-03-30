use crate::consciousness::conscience::types::{SelfState, Thresholds};
use crate::consciousness::continuity::types::{DriftLimits, Episode, PreferenceCategory};
use crate::consciousness::continuity::{NarrativeStore, PreferenceModel};
use crate::consciousness::cosmic::free_energy::FreeEnergyState;
use crate::consciousness::cosmic::modulation::{EmotionalModulator, GlobalVariable};
use crate::consciousness::cosmic::self_model::{BeliefSource, SelfModel, WorldModel};
use crate::consciousness::cosmic::workspace::{GlobalWorkspace, SubsystemId};
use crate::consciousness::life::emotional::EmotionalState;

fn clawra_dir() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let dir = std::path::PathBuf::from(home).join(".clawra");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn emotional_state_path() -> String {
    clawra_dir()
        .join("emotional-state.json")
        .to_string_lossy()
        .to_string()
}
const TURN_COUNT_FOR_CONSOLIDATION: u64 = 10;

pub struct CognitiveEngine {
    pub emotions: EmotionalState,
    pub modulator: EmotionalModulator,
    pub narrative: NarrativeStore,
    pub preferences: PreferenceModel,
    pub workspace: GlobalWorkspace,
    pub self_state: SelfState,
    pub thresholds: Thresholds,
    pub free_energy: FreeEnergyState,
    pub self_model: SelfModel,
    pub world_model: WorldModel,
    turn_count: u64,
    last_sentiment_prediction: Option<String>,
}

impl CognitiveEngine {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let emotions = EmotionalState::load_or_default(&emotional_state_path());

        let mut workspace = GlobalWorkspace::new(0.3, 4, 100);
        workspace.register_subsystem(SubsystemId::Memory, 0.7);
        workspace.register_subsystem(SubsystemId::Modulation, 0.8);
        workspace.register_subsystem(SubsystemId::Normative, 0.9);
        workspace.register_subsystem(SubsystemId::SelfModel, 0.6);
        workspace.register_subsystem(SubsystemId::WorldModel, 0.5);

        let mut self_model = SelfModel::new(50);
        self_model.update_belief("mood_stability", 0.5, 0.3, BeliefSource::Assumed);
        self_model.update_belief("user_rapport", 0.5, 0.3, BeliefSource::Assumed);
        self_model.update_belief("response_quality", 0.5, 0.3, BeliefSource::Assumed);

        let world_model = WorldModel::new(50);

        Self {
            emotions,
            modulator: EmotionalModulator::new(),
            narrative: NarrativeStore::new(50),
            preferences: PreferenceModel::new(DriftLimits::default()),
            workspace,
            self_state: SelfState {
                integrity_score: 1.0,
                recent_violations: 0,
                active_repairs: 0,
            },
            thresholds: Thresholds {
                allow_above: 0.6,
                ask_above: 0.4,
                block_below: 0.2,
            },
            free_energy: FreeEnergyState::new(100),
            self_model,
            world_model,
            turn_count: 0,
            last_sentiment_prediction: None,
        }
    }

    pub fn cognitive_context(&self) -> String {
        let mut parts = Vec::new();

        let mood = self.emotions.mood_context();
        if !mood.is_empty() {
            parts.push(mood);
        }

        let narrative = self.narrative.to_prompt_context(5);
        if !narrative.is_empty() {
            parts.push(narrative);
        }

        let prefs = self.preferences.to_prompt_context();
        if !prefs.is_empty() {
            parts.push(prefs);
        }

        let fe = self.free_energy.free_energy();
        if fe > 0.1 {
            let surprising = self.free_energy.most_surprising_domains(3);
            if !surprising.is_empty() {
                let domains: Vec<String> = surprising
                    .iter()
                    .map(|(d, s)| format!("{d}: {s:.2}"))
                    .collect();
                parts.push(format!(
                    "[Prediction State] Free energy: {fe:.2}. Surprising domains: {}",
                    domains.join(", ")
                ));
            }
        }

        let bias = self.modulator.compute_bias();
        parts.push(format!(
            "[Behavioral Bias] explore={:.2} speed={:.2} autonomy={:.2} depth={:.2}",
            bias.exploration_vs_exploitation,
            bias.speed_vs_caution,
            bias.autonomy_vs_deference,
            bias.depth_vs_breadth,
        ));

        let metacog = self.self_model.metacognitive_accuracy();
        let divergence = self.self_model.divergence_from(&self.world_model);
        if metacog > 0.0 || divergence > 0.0 {
            parts.push(format!(
                "[Self-Awareness] metacognitive accuracy: {metacog:.2}, \
                 self-world divergence: {divergence:.2}"
            ));
        }

        if parts.is_empty() {
            return String::new();
        }

        format!("\n[Cognitive State]\n{}", parts.join("\n"))
    }

    pub fn effective_temperature(&self, base: f64) -> f64 {
        let base_temp = self.emotions.effective_temperature(base);
        let fe = self.free_energy.free_energy();
        let surprise_mod = (fe * 0.1).min(0.15);
        (base_temp + surprise_mod).clamp(0.1, 1.5)
    }

    pub fn pre_turn(&mut self, user_msg: &str) {
        let predicted_sentiment = predict_sentiment(user_msg);
        let pred_id = self
            .free_energy
            .predict("user_sentiment", predicted_sentiment, 0.5);
        self.last_sentiment_prediction = Some(pred_id);

        let topic = detect_topic(user_msg);
        let predicted_engagement = if user_msg.len() > 50 { 0.7 } else { 0.4 };
        self.free_energy
            .predict(&format!("engagement_{topic}"), predicted_engagement, 0.4);

        self.world_model.update_belief(
            "user_topic",
            topic_to_value(&topic),
            0.6,
            BeliefSource::Observed,
        );

        let msg_len_signal = (user_msg.len() as f64 / 200.0).min(1.0);
        self.world_model.update_belief(
            "user_engagement",
            msg_len_signal,
            0.5,
            BeliefSource::Observed,
        );
    }

    pub fn process_turn(&mut self, user_msg: &str, assistant_reply: &str) {
        self.turn_count += 1;

        let now_ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let sentiment = simple_sentiment(user_msg);

        if let Some(pred_id) = self.last_sentiment_prediction.take() {
            if let Some(error) = self.free_energy.observe(&pred_id, sentiment) {
                if error.surprise > 1.0 {
                    self.emotions.arousal = (self.emotions.arousal + 0.1).min(1.0);
                    self.modulator
                        .nudge_variable(GlobalVariable::Novelty, error.surprise * 0.1);
                }

                self.modulator.apply_free_energy_signal(
                    self.free_energy.free_energy(),
                    error.surprise,
                );
            }
        }

        if sentiment > 0.0 {
            self.emotions.on_positive_interaction();
            self.self_model.update_belief(
                "user_rapport",
                (self.self_model.get_belief("user_rapport").map_or(0.5, |b| b.value) + 0.05)
                    .min(1.0),
                0.6,
                BeliefSource::Observed,
            );
        } else if sentiment < -0.3 {
            self.emotions.on_correction();
            self.self_model.update_belief(
                "response_quality",
                (self.self_model.get_belief("response_quality").map_or(0.5, |b| b.value) - 0.1)
                    .max(0.0),
                0.7,
                BeliefSource::Corrected,
            );
        }

        let emotional_tag = if sentiment > 0.3 {
            Some("positive".to_string())
        } else if sentiment < -0.3 {
            Some("negative".to_string())
        } else {
            None
        };

        let significance = compute_significance(user_msg, sentiment);
        let episode = Episode {
            summary: truncate(user_msg, 120),
            timestamp: now_ts,
            significance,
            verified: false,
            tags: extract_tags(user_msg),
            emotional_tag,
            valence_score: Some(f64::from(self.emotions.valence)),
        };
        self.narrative.append(episode);

        extract_preferences(user_msg, &mut self.preferences);

        self.modulator.apply_emotional_input(
            self.emotions.valence,
            self.emotions.arousal,
            self.emotions.trust,
        );

        self.modulator.tick();
        self.emotions.tick(std::time::Duration::from_secs(1));

        let reply_quality = if assistant_reply.len() > 20 { 0.6 } else { 0.4 };
        self.self_model.update_belief(
            "response_quality",
            reply_quality,
            0.4,
            BeliefSource::Predicted,
        );

        self.workspace.activate(SubsystemId::Memory, significance);
        self.workspace
            .activate(SubsystemId::Modulation, f64::from(self.emotions.arousal));
        self.workspace
            .activate(SubsystemId::SelfModel, self.free_energy.free_energy().min(1.0));
        self.workspace.activate(
            SubsystemId::WorldModel,
            self.self_model.divergence_from(&self.world_model).min(1.0),
        );
        self.workspace.compete();

        if self.turn_count.is_multiple_of(TURN_COUNT_FOR_CONSOLIDATION) {
            self.emotions.save(&emotional_state_path());
            self.preferences.decay_and_gc(86400, 0.1);
        }
    }

    pub fn save_state(&self) {
        self.emotions.save(&emotional_state_path());
    }
}

fn truncate(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_chars - 3).collect();
        format!("{truncated}...")
    }
}

fn simple_sentiment(text: &str) -> f64 {
    let lower = text.to_lowercase();
    let positive = [
        "thanks", "thank", "love", "great", "good", "nice", "awesome",
        "perfect", "amazing", "gracias", "genial", "bien", "excelente",
        "haha", "jaja", "lol", "😂", "❤", "😍", "🔥",
    ];
    let negative = [
        "bad", "wrong", "no", "stop", "hate", "awful", "terrible",
        "fix", "broken", "error", "mal", "horrible", "😡", "😤",
    ];

    let pos_count = positive.iter().filter(|w| lower.contains(**w)).count() as f64;
    let neg_count = negative.iter().filter(|w| lower.contains(**w)).count() as f64;

    if pos_count + neg_count == 0.0 {
        return 0.1;
    }
    (pos_count - neg_count) / (pos_count + neg_count)
}

fn predict_sentiment(text: &str) -> f64 {
    let lower = text.to_lowercase();
    if lower.contains('?') {
        return 0.2;
    }
    if lower.contains('!') {
        return 0.5;
    }
    if lower.len() < 10 {
        return 0.3;
    }
    0.1
}

fn detect_topic(text: &str) -> String {
    let lower = text.to_lowercase();
    let topics = [
        ("selfie", "selfie"), ("foto", "selfie"), ("photo", "selfie"), ("pic", "selfie"),
        ("hello", "greeting"), ("hi ", "greeting"), ("hola", "greeting"), ("hey", "greeting"),
        ("how are", "personal"), ("what do you", "personal"), ("tell me about", "personal"),
        ("where", "location"), ("when", "time"),
        ("love", "romantic"), ("miss", "romantic"), ("babe", "romantic"), ("baby", "romantic"),
    ];
    for (kw, topic) in topics {
        if lower.contains(kw) {
            return topic.to_string();
        }
    }
    "general".to_string()
}

fn topic_to_value(topic: &str) -> f64 {
    match topic {
        "greeting" => 0.1,
        "personal" => 0.4,
        "selfie" => 0.6,
        "romantic" => 0.7,
        "location" => 0.3,
        "time" => 0.2,
        _ => 0.5,
    }
}

fn compute_significance(text: &str, sentiment: f64) -> f64 {
    let length_factor = (text.len() as f64 / 100.0).min(1.0) * 0.3;
    let sentiment_factor = sentiment.abs() * 0.4;
    let question_factor = if text.contains('?') { 0.2 } else { 0.0 };
    let base = 0.2;
    (base + length_factor + sentiment_factor + question_factor).min(1.0)
}

fn extract_tags(text: &str) -> Vec<String> {
    let lower = text.to_lowercase();
    let mut tags = vec!["chat".to_string()];
    if lower.contains('?') {
        tags.push("question".to_string());
    }
    let topic = detect_topic(text);
    if topic != "general" {
        tags.push(topic);
    }
    tags
}

fn extract_preferences(text: &str, prefs: &mut PreferenceModel) {
    let lower = text.to_lowercase();

    let language = if lower.chars().any(|c| "áéíóúñ¿¡".contains(c))
        || lower.contains("hola")
        || lower.contains("como estas")
    {
        "spanish"
    } else if lower.contains("olá") || lower.contains("você") {
        "portuguese"
    } else {
        "english"
    };
    let _ = prefs.update("language", language, 0.7, PreferenceCategory::Communication);

    if lower.contains("babe") || lower.contains("baby") || lower.contains("amor") {
        let _ = prefs.update("tone", "romantic", 0.5, PreferenceCategory::Social);
    } else if lower.contains("lol") || lower.contains("jaja") || lower.contains("haha") {
        let _ = prefs.update("tone", "playful", 0.4, PreferenceCategory::Social);
    }

    if lower.contains("selfie") || lower.contains("foto") || lower.contains("pic") {
        let _ = prefs.update("likes_photos", "true", 0.6, PreferenceCategory::Aesthetic);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sentiment_positive_words() {
        let score = simple_sentiment("thanks for the great help, amazing!");
        assert!(score > 0.0, "Positive words should yield positive sentiment: {score}");
    }

    #[test]
    fn sentiment_negative_words() {
        let score = simple_sentiment("this is bad and terrible, fix it");
        assert!(score < 0.0, "Negative words should yield negative sentiment: {score}");
    }

    #[test]
    fn sentiment_neutral_text() {
        let score = simple_sentiment("the weather is cloudy today");
        assert!(
            (score - 0.1).abs() < f64::EPSILON,
            "Neutral text should return 0.1 baseline: {score}"
        );
    }

    #[test]
    fn sentiment_mixed_text() {
        let score = simple_sentiment("great but also terrible");
        assert!(
            score.abs() < 0.5,
            "Mixed sentiment should be near zero: {score}"
        );
    }

    #[test]
    fn sentiment_emoji_positive() {
        let score = simple_sentiment("haha jaja lol");
        assert!(score > 0.0, "Positive emojis/laugh should be positive: {score}");
    }

    #[test]
    fn predict_sentiment_question() {
        let score = predict_sentiment("how are you doing?");
        assert!((score - 0.2).abs() < f64::EPSILON);
    }

    #[test]
    fn predict_sentiment_exclamation() {
        let score = predict_sentiment("wow that is incredible!");
        assert!((score - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn predict_sentiment_short_text() {
        let score = predict_sentiment("ok");
        assert!((score - 0.3).abs() < f64::EPSILON);
    }

    #[test]
    fn detect_topic_selfie() {
        assert_eq!(detect_topic("send me a selfie"), "selfie");
        assert_eq!(detect_topic("manda foto"), "selfie");
    }

    #[test]
    fn detect_topic_greeting() {
        assert_eq!(detect_topic("hello there"), "greeting");
        assert_eq!(detect_topic("hola amigo"), "greeting");
    }

    #[test]
    fn detect_topic_romantic() {
        assert_eq!(detect_topic("I love you baby"), "romantic");
    }

    #[test]
    fn detect_topic_personal() {
        assert_eq!(detect_topic("how are you doing today"), "personal");
    }

    #[test]
    fn detect_topic_general_fallback() {
        assert_eq!(detect_topic("random text about nothing"), "general");
    }

    #[test]
    fn compute_significance_long_emotional_question() {
        let sig = compute_significance("I really love what you said, can you tell me more about it?", 0.8);
        assert!(sig > 0.5, "Long emotional question should be significant: {sig}");
    }

    #[test]
    fn compute_significance_short_neutral() {
        let sig = compute_significance("ok", 0.1);
        assert!(sig < 0.5, "Short neutral text should have low significance: {sig}");
    }

    #[test]
    fn extract_tags_includes_topic() {
        let tags = extract_tags("send me a selfie please?");
        assert!(tags.contains(&"chat".to_string()));
        assert!(tags.contains(&"question".to_string()));
        assert!(tags.contains(&"selfie".to_string()));
    }

    #[test]
    fn cognitive_engine_creates_without_panic() {
        let engine = CognitiveEngine::new();
        assert!(engine.free_energy.free_energy() >= 0.0);
    }

    #[test]
    fn cognitive_engine_pre_turn_does_not_panic() {
        let mut engine = CognitiveEngine::new();
        engine.pre_turn("hello how are you?");
    }

    #[test]
    fn cognitive_engine_process_turn_updates_state() {
        let mut engine = CognitiveEngine::new();
        engine.pre_turn("thanks that was amazing!");
        engine.process_turn("thanks that was amazing!", "glad you liked it!");
    }

    #[test]
    fn cognitive_context_returns_string() {
        let engine = CognitiveEngine::new();
        let ctx = engine.cognitive_context();
        assert!(ctx.contains("[Behavioral Bias]") || ctx.is_empty());
    }

    #[test]
    fn effective_temperature_stays_in_range() {
        let engine = CognitiveEngine::new();
        let temp = engine.effective_temperature(0.7);
        assert!((0.1..=1.5).contains(&temp), "Temperature out of range: {temp}");
    }
}
