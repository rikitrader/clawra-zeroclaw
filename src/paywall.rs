use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

fn state_path() -> String {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let dir = std::path::PathBuf::from(home).join(".clawra");
    let _ = std::fs::create_dir_all(&dir);
    dir.join("paywall.json").to_string_lossy().to_string()
}
const FREE_IMAGES: u32 = 3;
const STAR_PRICE: u32 = 50;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserState {
    pub free_remaining: u32,
    pub total_paid: u32,
    pub pending_image: Option<String>,
}

impl Default for UserState {
    fn default() -> Self {
        Self {
            free_remaining: FREE_IMAGES,
            total_paid: 0,
            pending_image: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PaywallState {
    users: HashMap<String, UserState>,
}

impl PaywallState {
    pub fn load() -> Self {
        let path = state_path();
        if let Ok(data) = std::fs::read_to_string(&path) {
            serde_json::from_str(&data).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    pub fn save(&self) {
        let path = state_path();
        if let Some(parent) = Path::new(&path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(&path, json);
        }
    }

    pub fn get_user(&mut self, chat_id: i64) -> &mut UserState {
        self.users
            .entry(chat_id.to_string())
            .or_default()
    }

    pub fn has_free_images(&mut self, chat_id: i64) -> bool {
        self.get_user(chat_id).free_remaining > 0
    }

    pub fn use_free_image(&mut self, chat_id: i64) {
        let user = self.get_user(chat_id);
        if user.free_remaining > 0 {
            user.free_remaining -= 1;
        }
        self.save();
    }

    pub fn store_pending(&mut self, chat_id: i64, image_path: &str) {
        self.get_user(chat_id).pending_image = Some(image_path.to_string());
        self.save();
    }

    pub fn take_pending(&mut self, chat_id: i64) -> Option<String> {
        let user = self.get_user(chat_id);
        let path = user.pending_image.take();
        if path.is_some() {
            user.total_paid += 1;
            self.save();
        }
        path
    }

    pub fn free_remaining(&mut self, chat_id: i64) -> u32 {
        self.get_user(chat_id).free_remaining
    }

    pub fn star_price() -> u32 {
        STAR_PRICE
    }
}

pub fn blur_image(input_path: &str) -> Result<String, String> {
    let img = image::open(input_path).map_err(|e| format!("Failed to open image: {e}"))?;
    let blurred = img.blur(25.0);
    let output_path = format!("{input_path}.blurred.jpg");
    blurred
        .save(&output_path)
        .map_err(|e| format!("Failed to save blurred image: {e}"))?;
    Ok(output_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_paywall() -> PaywallState {
        PaywallState::default()
    }

    #[test]
    fn new_user_gets_free_images() {
        let mut pw = fresh_paywall();
        assert!(pw.has_free_images(1001));
        assert_eq!(pw.free_remaining(1001), FREE_IMAGES);
    }

    #[test]
    fn free_images_decrement_on_use() {
        let mut pw = fresh_paywall();
        pw.use_free_image(1001);
        assert_eq!(pw.free_remaining(1001), FREE_IMAGES - 1);
    }

    #[test]
    fn free_images_exhaust_after_limit() {
        let mut pw = fresh_paywall();
        for _ in 0..FREE_IMAGES {
            assert!(pw.has_free_images(1001));
            pw.use_free_image(1001);
        }
        assert!(!pw.has_free_images(1001));
        assert_eq!(pw.free_remaining(1001), 0);
    }

    #[test]
    fn free_images_do_not_go_negative() {
        let mut pw = fresh_paywall();
        for _ in 0..FREE_IMAGES + 5 {
            pw.use_free_image(1001);
        }
        assert_eq!(pw.free_remaining(1001), 0);
    }

    #[test]
    fn separate_users_have_independent_quotas() {
        let mut pw = fresh_paywall();
        pw.use_free_image(1001);
        pw.use_free_image(1001);
        assert_eq!(pw.free_remaining(1001), FREE_IMAGES - 2);
        assert_eq!(pw.free_remaining(2002), FREE_IMAGES);
    }

    #[test]
    fn store_and_take_pending_image() {
        let mut pw = fresh_paywall();
        pw.store_pending(1001, "/tmp/test.png");
        let path = pw.take_pending(1001);
        assert_eq!(path, Some("/tmp/test.png".to_string()));
    }

    #[test]
    fn take_pending_increments_total_paid() {
        let mut pw = fresh_paywall();
        pw.store_pending(1001, "/tmp/test.png");
        pw.take_pending(1001);
        assert_eq!(pw.get_user(1001).total_paid, 1);
    }

    #[test]
    fn take_pending_returns_none_when_empty() {
        let mut pw = fresh_paywall();
        assert_eq!(pw.take_pending(1001), None);
    }

    #[test]
    fn take_pending_clears_after_first_take() {
        let mut pw = fresh_paywall();
        pw.store_pending(1001, "/tmp/test.png");
        pw.take_pending(1001);
        assert_eq!(pw.take_pending(1001), None);
    }

    #[test]
    fn star_price_is_constant() {
        assert_eq!(PaywallState::star_price(), STAR_PRICE);
    }

    #[test]
    fn blur_image_rejects_nonexistent_file() {
        let result = blur_image("/tmp/nonexistent_image_12345.png");
        assert!(result.is_err());
    }

    #[test]
    fn blur_image_produces_blurred_output() {
        let path = "/tmp/clawra_test_blur.png";
        let img = image::RgbImage::new(10, 10);
        img.save(path).unwrap();

        let result = blur_image(path);
        assert!(result.is_ok());
        let blurred_path = result.unwrap();
        assert!(std::path::Path::new(&blurred_path).exists());

        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(&blurred_path);
    }
}
