//! Key resolution: explicit value -> listed env vars -> OS keychain.
//! Nothing is logged, cached to disk, or sent anywhere except the
//! provider itself.
pub fn resolve(explicit: Option<&str>, env_names: &[&str]) -> Option<String> {
    if let Some(v) = explicit {
        if !v.is_empty() {
            return Some(v.to_string());
        }
    }
    env_names
        .iter()
        .find_map(|n| std::env::var(n).ok().filter(|v| !v.is_empty()))
}

/// OS keychain fallback (service `harness`, account = provider name).
/// Best-effort: any backend error (e.g. headless CI) yields `None`,
/// never a failure — env vars remain the primary path.
pub fn resolve_keychain(provider: &str) -> Option<String> {
    let entry = keyring::Entry::new("harness", provider).ok()?;
    entry.get_password().ok().filter(|v| !v.is_empty())
}

/// Full chain: explicit -> env -> keychain.
pub fn resolve_all(explicit: Option<&str>, provider: &str, env_names: &[&str]) -> Option<String> {
    resolve(explicit, env_names).or_else(|| resolve_keychain(provider))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefers_explicit_then_env() {
        std::env::set_var("HARNESS_TEST_KEY_X1", "env-key");
        assert_eq!(
            resolve(Some("direct"), &["HARNESS_TEST_KEY_X1"]).unwrap(),
            "direct"
        );
        assert_eq!(resolve(None, &["HARNESS_TEST_KEY_X1"]).unwrap(), "env-key");
        assert!(resolve(None, &["HARNESS_TEST_KEY_MISSING"]).is_none());
        std::env::remove_var("HARNESS_TEST_KEY_X1");
    }
}
