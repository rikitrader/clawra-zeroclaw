use reqwest::Client;

const DDG_HTML_URL: &str = "https://html.duckduckgo.com/html/";

const SEARCH_KEYWORDS: &[&str] = &[
    "busca", "buscar", "search", "google", "googlea",
    "investiga", "averigua", "dime sobre", "que es", "qué es",
    "quien es", "quién es", "what is", "who is", "where is",
    "cuando fue", "cuándo fue", "how to", "como se", "cómo se",
    "tell me about", "look up", "find out", "noticias de", "news about",
    "cuanto cuesta", "cuánto cuesta", "how much", "precio", "precios",
    "clima en", "weather in", "what happened", "que paso", "qué pasó",
    "explain", "explica", "define", "definicion", "meaning of",
    "dime", "cuanto", "cuánto", "cuantos", "cuántos",
    "cotizacion", "cotización", "valor del", "valor de",
    "noticias", "news", "latest", "current", "hoy",
    "stock", "crypto", "bitcoin", "dolar", "dólar", "euro",
    "oro", "gold", "silver", "plata", "petroleo", "petróleo",
];

const FACTUAL_PATTERNS: &[&str] = &[
    "en que año", "en qué año", "what year", "when did",
    "how many", "how old",
    "capital of", "capital de", "president of", "presidente de",
    "population", "poblacion", "how far", "a que distancia",
    "what country", "en que pais", "how tall", "cuanto mide",
    "quien gano", "quién ganó", "who won", "score",
];

pub fn needs_search(text: &str) -> bool {
    let lower = text.to_lowercase();

    if lower.len() < 5 {
        return false;
    }

    for kw in SEARCH_KEYWORDS {
        if lower.contains(kw) {
            return true;
        }
    }

    for pat in FACTUAL_PATTERNS {
        if lower.contains(pat) {
            return true;
        }
    }

    if lower.contains('?') {
        return true;
    }

    false
}

pub async fn web_search(client: &Client, query: &str) -> Option<String> {
    if let Ok(brave_key) = std::env::var("BRAVE_API_KEY") {
        if let Some(results) = brave_search(client, query, &brave_key).await {
            return Some(results);
        }
        eprintln!("\x1b[33m[SEARCH]\x1b[0m Brave API failed, falling back to DuckDuckGo");
    }
    ddg_search(client, query).await
}

async fn brave_search(client: &Client, query: &str, api_key: &str) -> Option<String> {
    let resp = client
        .get("https://api.search.brave.com/res/v1/web/search")
        .header("Accept", "application/json")
        .header("Accept-Encoding", "gzip")
        .header("X-Subscription-Token", api_key)
        .query(&[("q", query), ("count", "5")])
        .send()
        .await
        .ok()?;

    if !resp.status().is_success() {
        eprintln!(
            "\x1b[31m[SEARCH]\x1b[0m Brave Search error: {}",
            resp.status()
        );
        return None;
    }

    let body: serde_json::Value = resp.json().await.ok()?;
    let results = body.get("web")?.get("results")?.as_array()?;

    if results.is_empty() {
        return None;
    }

    let mut context = String::from("[Web Search Results (Brave)]\n");
    for (i, result) in results.iter().take(5).enumerate() {
        let title = result
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Untitled");
        let description = result
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let url = result
            .get("url")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        context.push_str(&format!(
            "{}. {} — {}\n   Source: {}\n",
            i + 1,
            title,
            description,
            url
        ));
    }
    context.push_str("[End of Search Results]\n");

    Some(context)
}

async fn ddg_search(client: &Client, query: &str) -> Option<String> {
    let resp = client
        .post(DDG_HTML_URL)
        .header("User-Agent", "Mozilla/5.0 (compatible; Clawra/1.0)")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(format!("q={}&b=&kl=", urlencoded(query)))
        .send()
        .await
        .ok()?;

    if !resp.status().is_success() {
        eprintln!(
            "\x1b[31m[SEARCH]\x1b[0m DuckDuckGo error: {}",
            resp.status()
        );
        return None;
    }

    let html = resp.text().await.ok()?;
    let results = parse_ddg_results(&html);

    if results.is_empty() {
        return None;
    }

    let mut context = String::from("[Web Search Results]\n");
    for (i, (title, snippet, url)) in results.iter().take(5).enumerate() {
        context.push_str(&format!(
            "{}. {} — {}\n   Source: {}\n",
            i + 1, title, snippet, url
        ));
    }
    context.push_str("[End of Search Results]\n");

    Some(context)
}

fn parse_ddg_results(html: &str) -> Vec<(String, String, String)> {
    let mut results = Vec::new();

    for block in html.split("class=\"result__body") {
        if results.len() >= 5 {
            break;
        }

        let title = extract_between(block, "class=\"result__a\"", "</a>")
            .map(|s| strip_html_tags(&s));
        let snippet = extract_between(block, "class=\"result__snippet", "</a>")
            .map(|s| {
                let after_tag = s.find('>').map(|i| &s[i + 1..]).unwrap_or(&s);
                strip_html_tags(after_tag)
            });
        let url = extract_href(block, "class=\"result__url\"");

        if let (Some(t), Some(s)) = (title, snippet) {
            if !t.is_empty() && !s.is_empty() {
                let u = url.unwrap_or_default();
                results.push((t, s, u));
            }
        }
    }

    results
}

fn extract_between(html: &str, start_marker: &str, end_marker: &str) -> Option<String> {
    let start = html.find(start_marker)?;
    let after = &html[start + start_marker.len()..];
    let tag_close = after.find('>')?;
    let content_start = &after[tag_close + 1..];
    let end = content_start.find(end_marker)?;
    Some(content_start[..end].to_string())
}

fn extract_href(html: &str, marker: &str) -> Option<String> {
    let pos = html.find(marker)?;
    let region = &html[pos..];
    let href_start = region.find("href=\"")?;
    let after = &region[href_start + 6..];
    let href_end = after.find('"')?;
    let raw = &after[..href_end];
    if raw.starts_with("//duckduckgo.com/l/?uddg=") {
        let encoded = raw.strip_prefix("//duckduckgo.com/l/?uddg=")?;
        let decoded = urldecoded(encoded);
        let clean = decoded.split('&').next().unwrap_or(&decoded);
        Some(clean.to_string())
    } else {
        Some(raw.to_string())
    }
}

fn strip_html_tags(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut in_tag = false;
    for ch in s.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(ch),
            _ => {}
        }
    }
    decode_html_entities(&result).trim().to_string()
}

fn decode_html_entities(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#x27;", "'")
        .replace("&apos;", "'")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
}

fn urlencoded(s: &str) -> String {
    let mut result = String::with_capacity(s.len() * 3);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(b as char);
            }
            b' ' => result.push('+'),
            _ => {
                result.push('%');
                result.push_str(&format!("{:02X}", b));
            }
        }
    }
    result
}

fn urldecoded(s: &str) -> String {
    let mut result = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(val) = u8::from_str_radix(
                std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("00"),
                16,
            ) {
                result.push(val);
                i += 3;
                continue;
            }
        }
        if bytes[i] == b'+' {
            result.push(b' ');
        } else {
            result.push(bytes[i]);
        }
        i += 1;
    }
    String::from_utf8_lossy(&result).to_string()
}

pub fn extract_url(text: &str) -> Option<String> {
    let words: Vec<&str> = text.split_whitespace().collect();
    for word in &words {
        let w = word.trim_matches(|c: char| !c.is_alphanumeric() && c != ':' && c != '/' && c != '.' && c != '-' && c != '_' && c != '?' && c != '=' && c != '&' && c != '%');
        if w.starts_with("http://") || w.starts_with("https://") {
            return Some(w.to_string());
        }
        if w.contains('.') && !w.contains(' ') && (w.ends_with(".com") || w.ends_with(".org") || w.ends_with(".net") || w.ends_with(".io") || w.ends_with(".dev") || w.ends_with(".xyz") || w.contains(".com/") || w.contains(".org/") || w.contains(".io/")) {
            return Some(format!("https://{w}"));
        }
    }
    None
}

pub async fn fetch_url(client: &Client, url: &str) -> Option<String> {
    let resp = client
        .get(url)
        .header("User-Agent", "Mozilla/5.0 (compatible; Clawra/1.0)")
        .send()
        .await
        .ok()?;

    if !resp.status().is_success() {
        eprintln!("\x1b[31m[BROWSE]\x1b[0m HTTP {}", resp.status());
        return None;
    }

    let content_type = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if !content_type.contains("text/html") && !content_type.contains("text/plain") {
        return Some(format!("[Non-HTML content: {content_type}]"));
    }

    let html = resp.text().await.ok()?;
    let text = html_to_text(&html);

    let truncated = if text.len() > 3000 {
        format!("{}...\n[Truncated — full page was {} chars]", &text[..3000], text.len())
    } else {
        text
    };

    Some(format!("[Web Page Content from {url}]\n{truncated}\n[End of Page Content]"))
}

fn html_to_text(html: &str) -> String {
    let mut text = html.to_string();

    let remove_tags = ["<script", "<style", "<nav", "<footer", "<header"];
    for tag in remove_tags {
        let close_tag = format!("</{}>", &tag[1..]);
        while let Some(start) = text.find(tag) {
            if let Some(end) = text[start..].find(&close_tag) {
                text = format!("{}{}", &text[..start], &text[start + end + close_tag.len()..]);
            } else {
                break;
            }
        }
    }

    let text = strip_html_tags(&text);
    let lines: Vec<&str> = text.lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && l.len() > 2)
        .collect();
    lines.join("\n")
}

pub fn extract_search_query(text: &str) -> String {
    let lower = text.to_lowercase();

    let prefixes = [
        "busca ", "buscar ", "search ", "google ", "googlea ",
        "investiga ", "averigua ", "look up ", "find out ",
        "dime sobre ", "tell me about ",
    ];

    for prefix in prefixes {
        if lower.starts_with(prefix) {
            return text[prefix.len()..].trim().to_string();
        }
    }

    text.trim()
        .trim_end_matches('?')
        .trim_end_matches('!')
        .to_string()
}
