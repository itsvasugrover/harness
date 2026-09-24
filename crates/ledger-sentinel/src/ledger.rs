//! Append-only audit ledger: hash-chained JSONL files plus a SQLite
//! index for filtered reads. Files are truth (`audit/*.jsonl`, one per
//! day); the index is rebuildable cache. Every record carries full
//! attribution so a bad merge traces to the exact actor and advisor.
use super::port::{chain_hash, Attribution, AuditEvent};
use anyhow::Result;
use sqlx::SqlitePool;

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS audit_index(
  seq INTEGER NOT NULL, id TEXT NOT NULL, time TEXT NOT NULL DEFAULT '',
  actor TEXT NOT NULL DEFAULT '', repo TEXT NOT NULL DEFAULT '',
  kind TEXT NOT NULL DEFAULT '', verdict TEXT NOT NULL DEFAULT '',
  hash TEXT NOT NULL DEFAULT '',
  summary TEXT NOT NULL DEFAULT '', agent TEXT NOT NULL DEFAULT '',
  skill TEXT NOT NULL DEFAULT '', mcp_server TEXT NOT NULL DEFAULT '',
  refs TEXT NOT NULL DEFAULT '[]',
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

/// `ALTER TABLE ... ADD COLUMN` for DBs created before a column
/// existed. Only the fixed known columns are ever added, so the
/// statements stay static; duplicate-column errors mean "already there".
async fn ensure_column(pool: &SqlitePool, col: &str) -> Result<()> {
    let ddl: &'static str = match col {
        "summary" => "ALTER TABLE audit_index ADD COLUMN summary TEXT NOT NULL DEFAULT ''",
        "agent" => "ALTER TABLE audit_index ADD COLUMN agent TEXT NOT NULL DEFAULT ''",
        "skill" => "ALTER TABLE audit_index ADD COLUMN skill TEXT NOT NULL DEFAULT ''",
        "mcp_server" => "ALTER TABLE audit_index ADD COLUMN mcp_server TEXT NOT NULL DEFAULT ''",
        "refs" => "ALTER TABLE audit_index ADD COLUMN refs TEXT NOT NULL DEFAULT '[]'",
        _ => return Ok(()),
    };
    match sqlx::query(ddl).execute(pool).await {
        Ok(_) => Ok(()),
        Err(e) if e.to_string().contains("duplicate column name") => Ok(()),
        Err(e) => Err(e.into()),
    }
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
        // Existing DBs predate the detail columns: add what's missing.
        for col in ["summary", "agent", "skill", "mcp_server", "refs"] {
            ensure_column(&pool, col).await?;
        }
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
    /// Returns the stored event (with seq filled).
    pub async fn append(
        &self,
        attribution: Attribution,
        repo: &str,
        kind: &str,
        summary: &str,
        verdict: &str,
        refs: &[String],
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
        event.refs = refs.to_vec();
        let hash = chain_hash(&event);
        let stored = event;
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
            "INSERT INTO audit_index(seq, id, time, actor, repo, kind, verdict, hash,
              summary, agent, skill, mcp_server, refs)
             VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(seq)
        .bind(stored.id.to_string())
        .bind(stored.time.to_rfc3339())
        .bind(&stored.attribution.actor)
        .bind(&stored.repo)
        .bind(&stored.kind)
        .bind(&stored.verdict)
        .bind(&hash)
        .bind(&stored.summary)
        .bind(&stored.attribution.agent)
        .bind(stored.attribution.skill.clone().unwrap_or_default())
        .bind(stored.attribution.mcp_server.clone().unwrap_or_default())
        .bind(serde_json::to_string(&stored.refs)?)
        .execute(&self.pool)
        .await?;
        Ok(stored)
    }

    /// Filtered read over the index (newest last). Times compare as
    /// RFC 3339 strings, which order chronologically. Built with
    /// `QueryBuilder` so the dynamic filter list stays injection-safe.
    pub async fn query(&self, filter: &AuditFilter) -> Result<Vec<AuditEvent>> {
        let mut qb = sqlx::QueryBuilder::new(
            "SELECT seq, id, time, actor, repo, kind, verdict,
              summary, agent, skill, mcp_server, refs FROM audit_index WHERE 1 = 1",
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
            .build_query_as::<(
                i64,
                String,
                String,
                String,
                String,
                String,
                String,
                String,
                String,
                String,
                String,
                String,
            )>()
            .fetch_all(&self.pool)
            .await?;
        Ok(rows
            .into_iter()
            .map(
                |(seq, id, time, actor, repo, kind, verdict, summary, agent, skill, mcp, refs)| {
                    AuditEvent {
                        seq: seq as u64,
                        id: id.parse().unwrap_or_else(|_| uuid::Uuid::new_v4()),
                        time: time.parse().unwrap_or_else(|_| chrono::Utc::now()),
                        attribution: Attribution {
                            actor,
                            agent,
                            skill: none_if_empty(skill),
                            mcp_server: none_if_empty(mcp),
                        },
                        repo,
                        kind,
                        summary,
                        verdict,
                        refs: serde_json::from_str(&refs).unwrap_or_default(),
                        hash_prev: String::new(),
                    }
                },
            )
            .collect())
    }
}

fn none_if_empty(s: String) -> Option<String> {
    if s.is_empty() {
        None
    } else {
        Some(s)
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
            .append(attr("w1"), "o/r", "tool.exec", "ran read", "pass", &[])
            .await
            .unwrap();
        ledger
            .append(
                attr("w2"),
                "o/r",
                "gate.decision",
                "Proceed",
                "pass",
                &["pr:3".into()],
            )
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
        // Reads hydrate the full event, not just the filter keys.
        assert_eq!(all[0].summary, "ran read");
        assert_eq!(all[0].attribution.agent, "build");
        assert_eq!(all[1].refs, vec!["pr:3"]);
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
