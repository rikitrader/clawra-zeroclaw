use super::{PreferenceCategory, PreferenceModel};

pub fn sanitize_tool_name(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
        .take(64)
        .collect()
}

pub fn extract_tool_preference(
    pref: &mut PreferenceModel,
    tool_name: &str,
    success: bool,
) -> Result<(), String> {
    if !success {
        return Ok(());
    }
    let key = format!("tool_affinity:{}", sanitize_tool_name(tool_name));
    pref.update(&key, "preferred", 0.3, PreferenceCategory::Technical)
}

pub fn extract_channel_preference(
    pref: &mut PreferenceModel,
    channel_name: &str,
) -> Result<(), String> {
    pref.update(
        "preferred_channel",
        channel_name,
        0.4,
        PreferenceCategory::Communication,
    )
}
