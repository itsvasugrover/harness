//! Public surface of model-switchboard.
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Parsed `provider/model@variant` reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelRef {
    pub provider: String,
    pub model: String,
    pub variant: Option<String>,
}

#[derive(Debug, Error)]
pub enum RefError {
    #[error("model ref is empty")]
    Empty,
    #[error("model ref '{0}' needs provider/model shape")]
    NoProvider(String),
}

impl ModelRef {
    pub fn parse(s: &str) -> Result<Self, RefError> {
        let s = s.trim();
        if s.is_empty() {
            return Err(RefError::Empty);
        }
        let (provider, rest) = s
            .split_once('/')
            .ok_or_else(|| RefError::NoProvider(s.into()))?;
        let (model, variant) = match rest.split_once('@') {
            Some((m, v)) => (m, Some(v)),
            None => (rest, None),
        };
        Ok(Self {
            provider: provider.into(),
            model: model.into(),
            variant: variant.map(|v| v.into()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_variant() {
        let r = ModelRef::parse("acme/m1@effort-low").unwrap();
        assert_eq!(r.provider, "acme");
        assert_eq!(r.model, "m1");
        assert_eq!(r.variant.unwrap(), "effort-low");
    }
}
