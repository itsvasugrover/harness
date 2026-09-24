//! Test-only mock forge: one-shot HTTP server asserting the request
//! shape (POST + path + needle) and serving canned JSON. Shared by the
//! adapter approve tests so the helper lives in exactly one file.
use std::io::{Read, Write};

/// Serve one request, assert it, reply `response`. Returns the base URL.
pub(crate) fn serve_once(expected_path: &str, expected_needle: &str, response: &str) -> String {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let path = expected_path.to_string();
    let needle = expected_needle.to_string();
    let body = response.to_string();
    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut buf = [0u8; 8192];
        let n = stream.read(&mut buf).unwrap_or(0);
        let req = String::from_utf8_lossy(&buf[..n]).into_owned();
        assert!(req.starts_with("POST "), "expected POST, got {req}");
        assert!(req.contains(&path), "expected {path} in {req}");
        assert!(req.contains(&needle), "expected {needle} in {req}");
        let res = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        let _ = stream.write_all(res.as_bytes());
    });
    format!("http://127.0.0.1:{port}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn github_approve_roundtrip() {
        use crate::port::Forge;
        let base = serve_once(
            "/repos/o/r/pulls/7/reviews",
            "APPROVE",
            r#"{"id": 42, "state": "APPROVED", "user": {"login": "octocat"}}"#,
        );
        let gh = crate::github::GitHub::new(&base, "t").unwrap();
        let review = gh.approve("o/r", 7, "lgtm from phone").await.unwrap();
        assert_eq!(review.id, "42");
        assert_eq!(review.state, "APPROVED");
        assert_eq!(review.author, "octocat");
    }

    #[tokio::test]
    async fn gitea_approve_roundtrip() {
        use crate::port::Forge;
        let base = serve_once(
            "/repos/o/r/pulls/7/reviews",
            "APPROVE",
            r#"{"id": 9, "state": "APPROVED", "user": {"login": "dev"}}"#,
        );
        let g = crate::gitea::Gitea::new(&base, "t").unwrap();
        let review = g.approve("o/r", 7, "lgtm from phone").await.unwrap();
        assert_eq!(review.id, "9");
        assert_eq!(review.state, "APPROVED");
        assert_eq!(review.author, "dev");
    }
}
