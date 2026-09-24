//! Doctor: preflight checks with one readable report.
//! New-user failures become a checklist, not five mysteries.
#[derive(Debug, Clone)]
pub struct Check {
    pub name: String,
    pub ok: bool,
    pub detail: String,
}

impl Check {
    pub fn report(checks: &[Check]) -> String {
        let mut lines = vec!["harness doctor:".to_string()];
        for c in checks {
            let mark = if c.ok { "ok  " } else { "FAIL" };
            lines.push(format!("  [{mark}] {} — {}", c.name, c.detail));
        }
        lines.join("\n")
    }
}

pub fn check_git() -> Check {
    match std::process::Command::new("git").arg("--version").output() {
        Ok(o) if o.status.success() => Check {
            name: "git".into(),
            ok: true,
            detail: String::from_utf8_lossy(&o.stdout).trim().into(),
        },
        _ => Check {
            name: "git".into(),
            ok: false,
            detail: "not found on PATH".into(),
        },
    }
}

pub fn check_writable_dir(path: &str) -> Check {
    match std::fs::create_dir_all(path).and_then(|_| std::fs::remove_dir_all(path).or(Ok(()))) {
        Ok(()) => Check {
            name: format!("writable {path}"),
            ok: true,
            detail: "ok".into(),
        },
        Err(e) => Check {
            name: format!("writable {path}"),
            ok: false,
            detail: e.to_string(),
        },
    }
}

pub fn check_port_free(port: u16) -> Check {
    match std::net::TcpListener::bind(("127.0.0.1", port)) {
        Ok(_) => Check {
            name: format!("port {port}"),
            ok: true,
            detail: "free".into(),
        },
        Err(_) => Check {
            name: format!("port {port}"),
            ok: false,
            detail: "in use".into(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_marks_failures() {
        let out = Check::report(&[Check {
            name: "x".into(),
            ok: false,
            detail: "why".into(),
        }]);
        assert!(out.contains("[FAIL]"));
    }
}
