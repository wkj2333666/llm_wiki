use axum::{
    extract::State,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

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

/// Lazily-initialized reqwest client with cookie store enabled.
/// Reusing the client across requests means DuckDuckGo's `d` cookie
/// persists, avoiding repeated JS challenges.
fn search_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .cookie_store(true)
            .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .build()
            .expect("Failed to build reqwest client")
    })
}

/// Warm up DuckDuckGo cookies by visiting the homepage first.
/// DuckDuckGo sets a `d` cookie that is required to pass the JS challenge
/// on the HTML search endpoint.
async fn ensure_ddg_cookies() {
    static WARMED: OnceLock<()> = OnceLock::new();
    if WARMED.get().is_some() {
        return;
    }
    // Best-effort — if this fails, search will still try
    let _ = search_client()
        .get("https://duckduckgo.com/")
        .send()
        .await;
    // Mark warmed regardless of outcome; we won't retry
    let _ = WARMED.set(());
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

    // Ensure we have DDG cookies before searching
    ensure_ddg_cookies().await;

    let client = search_client();

    // Try HTML endpoint first, fall back to Lite
    let results = match search_html(client, &query.query, query.limit).await {
        Some(r) => r,
        None => search_lite(client, &query.query, query.limit)
            .await
            .unwrap_or_default(),
    };

    Ok(Json(SearchResponse {
        error: if results.is_empty() { Some("No results found".to_string()) } else { None },
        results,
    }))
}

async fn search_html(
    client: &reqwest::Client,
    query: &str,
    limit: usize,
) -> Option<Vec<SearchResult>> {
    let url = format!("https://html.duckduckgo.com/html/?q={}", urlencoding::encode(query));
    let html = client.get(&url).send().await.ok()?.text().await.ok()?;

    if html.contains("result__a") {
        Some(parse_ddg_html(&html, limit))
    } else {
        None
    }
}

async fn search_lite(
    client: &reqwest::Client,
    query: &str,
    limit: usize,
) -> Option<Vec<SearchResult>> {
    let url = format!("https://lite.duckduckgo.com/lite/?q={}", urlencoding::encode(query));
    let html = client.get(&url).send().await.ok()?.text().await.ok()?;

    if html.contains("captcha") || html.contains("challenge") {
        return None;
    }

    let results = parse_ddg_lite(&html, limit);
    if results.is_empty() { None } else { Some(results) }
}

fn parse_ddg_html(html: &str, limit: usize) -> Vec<SearchResult> {
    let mut results = Vec::new();

    let link_pattern = r#"<a[^>]*class="result__a"[^>]*href="([^"]+)"[^>]*>([^<]+)</a>"#;
    let re = regex::Regex::new(link_pattern).unwrap();

    for cap in re.captures_iter(html).take(limit) {
        let url = cap.get(1).map(|m| m.as_str()).unwrap_or_default();
        let title = cap.get(2).map(|m| clean_html(m.as_str())).unwrap_or_default();
        let actual_url = extract_ddg_url(url);

        if !actual_url.is_empty() && !title.is_empty() {
            results.push(SearchResult {
                title,
                url: actual_url,
                snippet: String::new(),
            });
        }
    }

    // Extract snippets
    let snippet_pattern = r#"<a[^>]*class="result__snippet"[^>]*>([^<]+)</a>"#;
    let snippet_re = regex::Regex::new(snippet_pattern).unwrap();
    let snippets: Vec<String> = snippet_re
        .captures_iter(html)
        .map(|cap| cap.get(1).map(|m| clean_html(m.as_str())).unwrap_or_default())
        .collect();

    for (i, snippet) in snippets.iter().take(results.len()).enumerate() {
        if i < results.len() {
            results[i].snippet = snippet.clone();
        }
    }

    // Try fallback: extract snippets from result__snippet spans (not links)
    if results.iter().all(|r| r.snippet.is_empty()) {
        let fallback = r#"<span[^>]*class="result__snippet"[^>]*>([^<]+)</span>"#;
        let fb_re = regex::Regex::new(fallback).unwrap();
        let fb_snippets: Vec<String> = fb_re
            .captures_iter(html)
            .map(|cap| cap.get(1).map(|m| clean_html(m.as_str())).unwrap_or_default())
            .collect();
        for (i, snippet) in fb_snippets.iter().take(results.len()).enumerate() {
            if i < results.len() {
                results[i].snippet = snippet.clone();
            }
        }
    }

    results
}

/// Parse DuckDuckGo Lite results: <a rel="nofollow" href="//duckduckgo.com/l/?uddg=...">Title</a>
fn parse_ddg_lite(html: &str, limit: usize) -> Vec<SearchResult> {
    let mut results = Vec::new();

    // Lite format: result rows are <tr> with <td> containing links and snippets
    // Links: <a rel="nofollow" href="...">Title</a>
    // Snippets: <td class="result-snippet">snippet text</td>

    let link_re = regex::Regex::new(
        r#"<a[^>]*rel="nofollow"[^>]*href="([^"]+)"[^>]*>([^<]+)</a>"#
    ).unwrap();

    let snippet_re = regex::Regex::new(
        r#"<td[^>]*class="result-snippet"[^>]*>([^<]*(?:<[^/][^>]*>[^<]*</[^>]*>[^<]*)*)</td>"#
    ).unwrap();

    for cap in link_re.captures_iter(html).take(limit) {
        let url = cap.get(1).map(|m| m.as_str()).unwrap_or_default();
        let title = cap.get(2).map(|m| clean_html(m.as_str())).unwrap_or_default();
        let actual_url = extract_ddg_url(url);

        if !actual_url.is_empty() && !title.is_empty() {
            results.push(SearchResult {
                title,
                url: actual_url,
                snippet: String::new(),
            });
        }
    }

    for (i, cap) in snippet_re.captures_iter(html).take(results.len()).enumerate() {
        let snippet = cap.get(1).map(|m| clean_html(m.as_str())).unwrap_or_default();
        if i < results.len() {
            results[i].snippet = snippet;
        }
    }

    results
}

fn extract_ddg_url(ddg_url: &str) -> String {
    if ddg_url.contains("uddg=") {
        let start = ddg_url.find("uddg=").map(|i| i + 5).unwrap_or(0);
        let encoded = &ddg_url[start..];
        let end = encoded.find('&').unwrap_or(encoded.len());
        let encoded_url = &encoded[..end];
        urlencoding::decode(encoded_url).unwrap_or_default().to_string()
    } else if ddg_url.starts_with("http") {
        ddg_url.to_string()
    } else {
        String::new()
    }
}

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