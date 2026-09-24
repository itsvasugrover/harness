//! Shared forge HTTP: authed send with one rate-limit retry, plus
//! pure helpers (backoff math, Link cursors, repo splits). Both adapters
//! funnel through here so backoff and auth stay uniform.
use anyhow::{Context, Result};

pub fn client() -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .user_agent("harness-forge/0.1.0")
        .timeout(std::time::Duration::from_secs(30))
        .build()?)
}

/// Send with a caller-built `Authorization` value (`Bearer …` on GitHub,
/// `token …` on Gitea); on a rate-limit signal sleep once (capped)
/// and retry a single time.
pub async fn send(auth: &str, req: reqwest::RequestBuilder) -> Result<reqwest::Response> {
    let retry = req.try_clone().context("request is not cloneable")?;
    let resp = req
        .header(reqwest::header::AUTHORIZATION, auth)
        .send()
        .await?;
    let status = resp.status().as_u16();
    let headers = resp.headers().clone();
    if let Some(wait) = backoff_secs(status, &headers) {
        tokio::time::sleep(std::time::Duration::from_secs(wait)).await;
        return Ok(retry
            .header(reqwest::header::AUTHORIZATION, auth)
            .send()
            .await?);
    }
    Ok(resp)
}

pub async fn ok_json(resp: reqwest::Response, forge: &str) -> Result<serde_json::Value> {
    let status = resp.status();
    let doc: serde_json::Value = resp.json().await?;
    if !status.is_success() {
        let msg = doc
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("forge error");
        anyhow::bail!("{forge} {status}: {msg}");
    }
    Ok(doc)
}

pub fn split_repo(repo: &str) -> Result<(&str, &str)> {
    repo.split_once('/').context("repo must be owner/name")
}

pub fn url_encode(s: &str) -> String {
    s.replace(' ', "+")
}

/// Seconds to wait when rate-limited, capped at 60. Pure for tests:
/// 429 honors `Retry-After`; a 4xx with an exhausted quota honors the
/// reset timestamp.
pub fn backoff_secs(status: u16, headers: &reqwest::header::HeaderMap) -> Option<u64> {
    let get = |k: &str| headers.get(k).and_then(|v| v.to_str().ok());
    if status == 429 {
        let wait: u64 = get("retry-after").and_then(|v| v.parse().ok()).unwrap_or(5);
        return Some(wait.min(60));
    }
    if (400..500).contains(&status) && get("x-ratelimit-remaining") == Some("0") {
        let reset: i64 = get("x-ratelimit-reset")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        let now = chrono::Utc::now().timestamp();
        return Some((reset - now).clamp(1, 60) as u64);
    }
    None
}

/// `Link: <...page=3>; rel="next"` → `Some("3")`. Pure for tests.
pub fn next_cursor(link: Option<&str>) -> Option<String> {
    for part in link?.split(',') {
        let mut it = part.split(';');
        let url = it.next()?.trim();
        if it.any(|p| p.trim() == r#"rel="next""#) {
            let page: String = url
                .split("page=")
                .nth(1)?
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .collect();
            if !page.is_empty() {
                return Some(page);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headers(pairs: &[(&str, &str)]) -> reqwest::header::HeaderMap {
        let mut h = reqwest::header::HeaderMap::new();
        for (k, v) in pairs {
            h.insert(
                reqwest::header::HeaderName::from_bytes(k.as_bytes()).unwrap(),
                v.parse().unwrap(),
            );
        }
        h
    }

    #[test]
    fn backoff_honors_retry_after_and_cap() {
        let h = headers(&[("retry-after", "120")]);
        assert_eq!(backoff_secs(429, &h), Some(60));
        assert_eq!(backoff_secs(200, &h), None);
    }

    #[test]
    fn backoff_honors_quota_reset() {
        let reset = chrono::Utc::now().timestamp() + 30;
        let h = headers(&[
            ("x-ratelimit-remaining", "0"),
            ("x-ratelimit-reset", &reset.to_string()),
        ]);
        let wait = backoff_secs(403, &h).unwrap();
        assert!((1..=60).contains(&wait));
    }

    #[test]
    fn link_cursor_parses_next_page() {
        let link = r#"<https://x?page=2>; rel="prev", <https://x?page=3>; rel="next""#;
        assert_eq!(next_cursor(Some(link)), Some("3".into()));
        assert_eq!(next_cursor(None), None);
    }

    #[test]
    fn rejects_bare_repo() {
        assert!(split_repo("noslash").is_err());
    }
}
