use super::model::SoulModel;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::Path;

pub fn parse_soul_file(path: &Path) -> Result<SoulModel> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read soul file: {}", path.display()))?;
    parse_soul_content(&content)
}

pub fn parse_soul_content(content: &str) -> Result<SoulModel> {
    let mut soul = SoulModel::default();
    let body = extract_frontmatter(content, &mut soul);
    parse_sections(body, &mut soul);
    Ok(soul)
}

fn extract_frontmatter<'a>(content: &'a str, soul: &mut SoulModel) -> &'a str {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return content;
    }

    let after_open = &trimmed[3..];
    let close_pos = match after_open.find("\n---") {
        Some(pos) => pos,
        None => return content,
    };

    let frontmatter = &after_open[..close_pos];
    let body_start = 3 + close_pos + 4;
    let body = if body_start < trimmed.len() {
        &trimmed[body_start..]
    } else {
        ""
    };

    for line in frontmatter.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim();
            let value = value.trim();
            match key {
                "name" => soul.name = value.to_string(),
                "soul_version" | "version" => {
                    if value != "1" {
                        eprintln!("Warning: unsupported soul version: {value}, expected 1");
                    }
                }
                _ => {}
            }
        }
    }

    body
}

fn parse_sections(body: &str, soul: &mut SoulModel) {
    let mut current_section: Option<&str> = None;
    let mut current_lines: Vec<&str> = Vec::new();

    for line in body.lines() {
        if let Some(header) = line.strip_prefix("## ") {
            if let Some(section) = current_section {
                apply_section(soul, section, &current_lines);
            }
            current_section = Some(header.trim());
            current_lines.clear();
        } else {
            current_lines.push(line);
        }
    }

    if let Some(section) = current_section {
        apply_section(soul, section, &current_lines);
    }
}

fn apply_section(soul: &mut SoulModel, section: &str, lines: &[&str]) {
    let section_lower = section.to_lowercase();
    match section_lower.as_str() {
        "values" => {
            soul.values = parse_list_items(lines);
        }
        "personality" => {
            soul.personality = parse_key_value_items(lines);
        }
        "boundaries" => {
            soul.boundaries = parse_list_items(lines);
        }
        "capabilities" => {
            soul.capabilities = parse_list_items(lines);
        }
        "relationships" => {
            soul.relationships = parse_key_value_items(lines);
        }
        "financial character" => {
            soul.financial_character = parse_key_value_items(lines);
        }
        "bio" => {
            let text = lines.join("\n").trim().to_string();
            if !text.is_empty() {
                soul.bio = Some(text);
            }
        }
        "genesis prompt" => {
            let text = lines.join("\n").trim().to_string();
            if !text.is_empty() {
                soul.genesis_prompt = Some(text);
            }
        }
        _ => {}
    }
}

fn parse_list_items(lines: &[&str]) -> Vec<String> {
    lines
        .iter()
        .filter_map(|line| {
            let trimmed = line.trim();
            trimmed
                .strip_prefix("- ")
                .or_else(|| trimmed.strip_prefix("* "))
        })
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn parse_key_value_items(lines: &[&str]) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in lines {
        let trimmed = line.trim();
        let item = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "));
        if let Some(item) = item {
            if let Some((key, value)) = item.split_once(':') {
                let key = key.trim().to_string();
                let value = value.trim().to_string();
                if !key.is_empty() && !value.is_empty() {
                    map.insert(key, value);
                }
            }
        }
    }
    map
}
