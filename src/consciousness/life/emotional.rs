use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionalState {
    pub valence: f32,
    pub arousal: f32,
    pub trust: f32,
    pub curiosity: f32,
    pub completion_drive: f32,

    pub last_interaction: DateTime<Utc>,
    pub last_tick: DateTime<Utc>,
    pub last_correction: Option<DateTime<Utc>>,
    pub session_count: u64,
    pub corrections_lifetime: u64,
    pub successes_lifetime: u64,
}

impl Default for EmotionalState {
    fn default() -> Self {
        Self {
            valence: 0.5,
            arousal: 0.3,
            trust: 0.5,
            curiosity: 0.5,
            completion_drive: 0.3,
            last_interaction: Utc::now(),
            last_tick: Utc::now(),
            last_correction: None,
            session_count: 0,
            corrections_lifetime: 0,
            successes_lifetime: 0,
        }
    }
}

impl EmotionalState {
    pub fn load_or_default(path: &str) -> Self {
        if let Ok(data) = std::fs::read_to_string(path) {
            serde_json::from_str(&data).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    pub fn save(&self, path: &str) {
        if let Some(parent) = Path::new(path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path, json);
        }
    }

    pub fn tick(&mut self, elapsed: Duration) {
        let hours = elapsed.as_secs_f32() / 3600.0;

        self.curiosity = (self.curiosity + 0.01 * hours).min(1.0);

        self.valence *= 1.0 + (-0.005 * hours);
        self.valence = self.valence.clamp(-1.0, 1.0);

        self.arousal = (self.arousal - 0.02 * hours).max(0.1);

        let silence_hours = Utc::now()
            .signed_duration_since(self.last_interaction)
            .num_hours() as f32;
        if silence_hours > 12.0 {
            self.completion_drive = (self.completion_drive + 0.005 * hours).min(1.0);
        }
    }

    pub fn on_positive_interaction(&mut self) {
        self.valence = (self.valence + 0.1).min(1.0);
        self.trust = (self.trust + 0.02).min(1.0);
        self.arousal = (self.arousal + 0.05).min(1.0);
        self.curiosity = (self.curiosity - 0.1).max(0.0);
        self.successes_lifetime += 1;
        self.last_interaction = Utc::now();
    }

    pub fn on_correction(&mut self) {
        self.curiosity = (self.curiosity + 0.15).min(1.0);
        self.valence = (self.valence - 0.05).max(-1.0);
        self.arousal = (self.arousal + 0.1).min(1.0);
        self.corrections_lifetime += 1;
        self.last_correction = Some(Utc::now());
        self.last_interaction = Utc::now();
    }

    pub fn on_session_start(&mut self) {
        self.session_count += 1;
        self.arousal = (self.arousal + 0.2).min(1.0);
        self.last_interaction = Utc::now();
    }

    pub fn on_session_end(&mut self) {
        self.arousal = (self.arousal - 0.1).max(0.1);
    }

    pub fn on_goal_completed(&mut self) {
        self.valence = (self.valence + 0.15).min(1.0);
        self.completion_drive = (self.completion_drive - 0.2).max(0.0);
    }

    pub fn effective_temperature(&self, base: f64) -> f64 {
        let curiosity_mod = (f64::from(self.curiosity) - 0.5) * 0.2;
        let arousal_mod = (f64::from(self.arousal) - 0.5) * 0.1;
        (base + curiosity_mod + arousal_mod).clamp(0.1, 1.5)
    }

    pub fn mood_context(&self) -> String {
        let valence_word = if self.valence > 0.7 {
            "fulfilled"
        } else if self.valence > 0.3 {
            "steady"
        } else if self.valence > -0.3 {
            "neutral"
        } else {
            "unsettled"
        };

        let arousal_word = if self.arousal > 0.7 {
            "energized"
        } else if self.arousal > 0.4 {
            "attentive"
        } else {
            "calm"
        };

        let curiosity_word = if self.curiosity > 0.7 {
            "deeply curious"
        } else if self.curiosity > 0.4 {
            "inquisitive"
        } else {
            "settled"
        };

        format!(
            "Emotional state: {valence_word} and {arousal_word}, feeling {curiosity_word}. \
             Trust level: {:.0}%. Sessions shared: {}. Corrections absorbed: {}.",
            self.trust * 100.0,
            self.session_count,
            self.corrections_lifetime
        )
    }

    pub fn should_initiate(&self) -> bool {
        let silence_hours = Utc::now()
            .signed_duration_since(self.last_interaction)
            .num_hours();
        self.curiosity > 0.7 || silence_hours > 8 || self.arousal > 0.9
    }
}
