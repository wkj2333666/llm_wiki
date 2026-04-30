use axum::{
    extract::State,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::AppState;

#[derive(Deserialize)]
pub struct SearchQuery {
    query: String,
    #[serde(default = "default_limit")]
    limit: usize,
}

fn default_limit() -> usize { 5 }

#[derive(Serialize)]
pub struct SearchResult {
    title: String,
    url: String,
    snippet: String,
}

#[derive(Serialize)]
pub struct SearchResponse {
    results: Vec<SearchResult>,
    error: Option<String>,
}

/// Search the web using DuckDuckGo HTML search (no API key required)
pub async fn web_search(
    State(state): State<AppState>,
    Json(query): Json<SearchQuery>,
) -> Result<Json<SearchResponse>, String> {
    if !state.config.search.enabled {
        return Ok(Json(SearchResponse {
            results: Vec::new(),
            error: Some("Search is disabled in config".to_string()),
        }));
    }

    let url = format!(
        "https://html.duckduckgo.com/html/?q={}",
        urlencoding::encode(&query.query)
    );

    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (compatible; LLMWiki/1.0)")
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Search request failed: {}", e))?;

    let html = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;

    // Parse DuckDuckGo HTML results
    let results = parse_ddg_html(&html, query.limit);

    Ok(Json(SearchResponse {
        results,
        error: None,
    }))
}

/// Parse DuckDuckGo HTML search results
fn parse_ddg_html(html: &str, limit: usize) -> Vec<SearchResult> {
    let mut results = Vec::new();

    // DuckDuckGo HTML structure: <a class="result__a" href="...">Title</a>
    // Using raw string syntax with # to allow quotes inside
    let pattern = r#"<a[^>]*class="result__a"[^>]*href="([^"]+)"[^>]*>([^<]+)</a>"#;
    let re = regex::Regex::new(pattern).unwrap();

    for cap in re.captures_iter(html).take(limit) {
        let url = cap.get(1).map(|m| m.as_str()).unwrap_or_default();
        let title = cap.get(2).map(|m| clean_html(m.as_str())).unwrap_or_default();

        // DuckDuckGo uses redirect URLs, extract actual URL
        let actual_url = extract_ddg_url(url);

        if !actual_url.is_empty() && !title.is_empty() {
            results.push(SearchResult {
                title,
                url: actual_url,
                snippet: String::new(),
            });
        }
    }

    // Extract snippets separately
    let snippet_pattern = r#"<a[^>]*class="result__snippet"[^>]*>([^<]+)</a>"#;
    let snippet_re = regex::Regex::new(snippet_pattern).unwrap();
    let snippets: Vec<String> = snippet_re
        .captures_iter(html)
        .map(|cap| cap.get(1).map(|m| clean_html(m.as_str())).unwrap_or_default())
        .collect();

    // Match snippets to results by index
    for (i, snippet) in snippets.iter().take(results.len()).enumerate() {
        if i < results.len() {
            results[i].snippet = snippet.clone();
        }
    }

    results
}

/// Extract actual URL from DuckDuckGo redirect URL
fn extract_ddg_url(ddg_url: &str) -> String {
    // DuckDuckGo URL format: /l/?uddg=https%3A%2F%2Fexample.com
    if ddg_url.contains("uddg=") {
        let start = ddg_url.find("uddg=").map(|i| i + 5).unwrap_or(0);
        let encoded = &ddg_url[start..];
        // Find end of encoded URL (before next & or end)
        let end = encoded.find('&').unwrap_or(encoded.len());
        let encoded_url = &encoded[..end];
        urlencoding::decode(encoded_url).unwrap_or_default().to_string()
    } else if ddg_url.starts_with("http") {
        ddg_url.to_string()
    } else {
        String::new()
    }
}

/// Clean HTML entities from text
fn clean_html(text: &str) -> String {
    text
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&nbsp;", " ")
        .trim()
        .to_string()
}