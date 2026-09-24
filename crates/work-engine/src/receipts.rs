//! Tool-call receipts: SHA-256 over (tool + input + output).
//! Tamper-evident without storing full outputs twice — the audit log
//! keeps receipts, the recall store keeps bodies.
use sha2::{Digest, Sha256};

pub fn receipt(tool: &str, input: &str, output: &str) -> String {
    let mut h = Sha256::new();
    h.update(tool.as_bytes());
    h.update([0u8]);
    h.update(input.as_bytes());
    h.update([0u8]);
    h.update(output.as_bytes());
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_and_sensitive() {
        let a = receipt("read", "a.txt", "hi");
        assert_eq!(a.len(), 64);
        assert_eq!(a, receipt("read", "a.txt", "hi"));
        assert_ne!(a, receipt("read", "a.txt", "bye"));
    }
}
