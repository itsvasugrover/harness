//! Workdir jail: every tool path resolves under the workdir.
//! `..` escapes and absolute paths outside the base fail loudly.
use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};

/// Resolve `user` (relative or absolute) to a path inside `workdir`.
pub fn resolve(workdir: &str, user: &str) -> Result<PathBuf> {
    let base = Path::new(workdir)
        .canonicalize()
        .context("workdir missing")?;
    let joined = base.join(user.trim_start_matches('/'));
    // Nearest existing ancestor -> canonicalize -> re-append remainder.
    // Works for not-yet-created files without mkdir side effects.
    let mut existing = joined.as_path();
    let mut rest = Vec::new();
    while !existing.exists() {
        match existing.file_name() {
            Some(name) => rest.push(name),
            None => bail!("path has no file name: {user}"),
        }
        existing = existing.parent().context("path has no parent")?;
    }
    let mut probe = existing.canonicalize().context("cannot resolve path")?;
    for comp in rest.iter().rev() {
        probe.push(comp);
    }
    if !probe.starts_with(&base) {
        bail!("path escapes workdir: {user}");
    }
    Ok(probe)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn setup(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("harness-jail-{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn allows_inside() {
        let dir = setup("in");
        let p = resolve(dir.to_str().unwrap(), "a/b.txt").unwrap();
        assert!(p.starts_with(&dir));
    }

    #[test]
    fn blocks_escape() {
        let dir = setup("out");
        assert!(resolve(dir.to_str().unwrap(), "../../etc/passwd").is_err());
    }
}
