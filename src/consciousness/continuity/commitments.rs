use super::types::Commitment;

static COMMITMENT_PATTERNS: &[&str] = &[
    "I will ",
    "I'll ",
    "I am going to ",
    "I'm going to ",
    "Let me ",
];

pub fn extract_commitments(response: &str) -> Vec<Commitment> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let mut commitments = Vec::new();
    for line in response.lines() {
        let trimmed = line.trim();
        for pattern in COMMITMENT_PATTERNS {
            if trimmed.contains(pattern) {
                commitments.push(Commitment {
                    description: truncate(trimmed, 200),
                    made_at: now,
                    expires_at: Some(now + 3600),
                    fulfilled: false,
                    context: String::new(),
                });
                break;
            }
        }
    }
    commitments
}

pub fn check_fulfillment(commitments: &mut [Commitment], successful_tools: &[String]) {
    for c in commitments.iter_mut() {
        if c.fulfilled {
            continue;
        }
        let desc_lower = c.description.to_lowercase();
        for tool in successful_tools {
            if desc_lower.contains(&tool.to_lowercase()) {
                c.fulfilled = true;
                c.context = format!("fulfilled by tool: {}", tool);
                break;
            }
        }
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        let end = s
            .char_indices()
            .take_while(|(i, _)| *i < max)
            .last()
            .map(|(i, c)| i + c.len_utf8())
            .unwrap_or(max);
        format!("{}...", &s[..end])
    }
}
