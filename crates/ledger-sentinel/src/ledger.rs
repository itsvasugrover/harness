//! Append-only audit ledger: hash-chained JSONL files plus a SQLite
//! index for filtered reads. Files are truth (`audit/*.jsonl`, one per
//! day); the index is rebuildable cache. Every record carries full
//! attribution so a bad merge traces to the exact actor and advisor.
use super::port::{Attribution, AuditEvent};
use anyhow::Result;
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS audit_index(
  seq INTEGER NOT NULL, id TEXT NOT NULL, time TEXT NOT NULL DEFAULT '',
  actor TEXT NOT NULL DEFAULT '', repo TEXT NOT NULL DEFAULT '',
  kind TEXT NOT NULL DEFAULT '', verdict TEXT NOT NULL DEFAULT '',
  hash TEXT NOT NULL DEFAULT '',
  PRIMARY KEY(seq)
);
CREATE INDEX IF NOT EXISTS idx_audit_filter ON audit_index(repo, kind, time);";

/// Filter for audit reads. All fields optional; `limit` caps rows.
#[derive(Debug, Clone, Default)]
pub struct AuditFilter {
    pub actor: Option<String>,
    pub repo: Option<String>,
    pub kind: Option<String>,
    pub since: Option<String>,
    pub until: Option<String>,
    pub limit: u64,
}

/// Chain link: hex sha256 over the canonical fields plus `hash_prev`.
pub fn chain_hash(event: &AuditEvent) -> String {
    let canon = format!(
        "{}|{}|{}|{}|{}|{}|{}|{}",
        event.seq,
        event.time.to_rfc3339(),
        event.attribution.actor,
        event.attribution.agent,
        event.repo,
        event.kind,
        event.summary,
        event.hash_prev
    );
    Sha256::digest(canon.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[derive(Debug, Clone)]
pub struct Ledger {
    dir: std::path::PathBuf,
    pool: SqlitePool,
}

impl Ledger {
    async fn init(dir: std::path::PathBuf, pool: SqlitePool) -> Result<Self> {
        std::fs::create_dir_all(&dir)?;
        sqlx::query(SCHEMA).execute(&pool).await?;
        Ok(Self { dir, pool })
    }

    /// Open a file-backed ledger under `<data_dir>/` (`audit/` + db).
    pub async fn open(data_dir: &str) -> Result<Self> {
        let dir = std::path::PathBuf::from(format!("{data_dir}/audit"));
        let pool = SqlitePool::connect(&format!(
            "sqlite://{data_dir}/db/harness.db?create_if_missing=true"
        ))
        .await?;
        Self::init(dir, pool).await
    }

    #[cfg(test)]
    pub async fn open_memory() -> Result<Self> {
        let pool = SqlitePool::connect("sqlite::memory:").await?;
        Self::init(std::env::temp_dir().join("harness-audit-test"), pool).await
    }

    fn day_file(&self, event: &AuditEvent) -> std::path::PathBuf {
        self.dir
            .join(format!("{}.jsonl", event.time.format("%Y-%m-%d")))
    }

    /// Append one event: chain-link it, write JSONL, index the row.
    /// Returns the stored event (with seq and hash filled).
    pub async fn append(
        &self,
        attribution: Attribution,
        repo: &str,
        kind: &str,
        summary: &str,
        verdict: &str,
    ) -> Result<AuditEvent> {
        let (seq,): (i64,) = sqlx::query_as("SELECT COALESCE(MAX(seq), -1) + 1 FROM audit_index")
            .fetch_one(&self.pool)
            .await?;
        let prev: Option<(String,)> = sqlx::query_as("SELECT hash FROM audit_index WHERE seq = ?")
            .bind(seq - 1)
            .fetch_optional(&self.pool)
            .await?;
        let mut event = AuditEvent::new(
            seq as u64,
            attribution,
            repo,
            kind,
            summary,
            prev.map(|(h,)| h).as_deref().unwrap_or("genesis"),
        );
        event.verdict = verdict.into();
        let hash = chain_hash(&event);
        let mut stored = event.clone();
        stored.hash_prev = event.hash_prev.clone();
        let line = serde_json::to_string(&stored)?;
        {
            use std::io::Write;
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(self.day_file(&stored))?;
            writeln!(file, "{line}")?;
        }
        sqlx::query(
            "INSERT INTO audit_index(seq, id, time, actor, repo, kind, verdict, hash)
             VALUES(?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(seq)
        .bind(stored.id.to_string())
        .bind(stored.time.to_rfc3339())
        .bind(&stored.attribution.actor)
        .bind(&stored.repo)
        .bind(&stored.kind)
        .bind(&stored.verdict)
        .bind(&hash)
        .execute(&self.pool)
        .await?;
        Ok(stored)
    }

    /// Filtered read over the index (newest last). Times compare as
    /// RFC 3339 strings, which order chronologically. Built with
    /// `QueryBuilder` so the dynamic filter list stays injection-safe.
    pub async fn query(&self, filter: &AuditFilter) -> Result<Vec<AuditEvent>> {
        let mut qb = sqlx::QueryBuilder::new(
            "SELECT seq, id, time, actor, repo, kind, verdict FROM audit_index WHERE 1 = 1",
        );
        if let Some(v) = &filter.actor {
            qb.push(" AND actor = ");
            qb.push_bind(v);
        }
        if let Some(v) = &filter.repo {
            qb.push(" AND repo = ");
            qb.push_bind(v);
        }
        if let Some(v) = &filter.kind {
            qb.push(" AND kind = ");
            qb.push_bind(v);
        }
        if let Some(v) = &filter.since {
            qb.push(" AND time >= ");
            qb.push_bind(v);
        }
        if let Some(v) = &filter.until {
            qb.push(" AND time <= ");
            qb.push_bind(v);
        }
        qb.push(" ORDER BY seq LIMIT ");
        qb.push_bind(filter.limit.clamp(1, 500) as i64);
        let rows = qb
            .build_query_as::<(i64, String, String, String, String, String, String)>()
            .fetch_all(&self.pool)
            .await?;
        Ok(rows
            .into_iter()
            .map(|(seq, id, time, actor, repo, kind, verdict)| AuditEvent {
                seq: seq as u64,
                id: id.parse().unwrap_or_else(|_| uuid::Uuid::new_v4()),
                time: time.parse().unwrap_or_else(|_| chrono::Utc::now()),
                attribution: Attribution {
                    actor,
                    agent: String::new(),
                    skill: None,
                    mcp_server: None,
                },
                repo,
                kind,
                summary: String::new(),
                verdict,
                refs: vec![],
                hash_prev: String::new(),
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attr(actor: &str) -> Attribution {
        Attribution {
            actor: actor.into(),
            agent: "build".into(),
            skill: None,
            mcp_server: None,
        }
    }

    #[tokio::test]
    async fn appends_chain_and_filters() {
        let ledger = Ledger::open_memory().await.unwrap();
        ledger
            .append(attr("w1"), "o/r", "tool.exec", "ran read", "pass")
            .await
            .unwrap();
        ledger
            .append(attr("w2"), "o/r", "gate.decision", "Proceed", "pass")
            .await
            .unwrap();
        let all = ledger
            .query(&AuditFilter {
                limit: 10,
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!((all[0].seq, all[1].seq), (0, 1));
        let one = ledger
            .query(&AuditFilter {
                kind: Some("gate.decision".into()),
                limit: 10,
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(one.len(), 1);
        assert_eq!(one[0].attribution.actor, "w2");
    }
}
