//! Intent idempotency store: replayed ids return stored results.
//! One SQLite table on the daemon DB; storms dedup without re-executing.
use anyhow::Result;
use sqlx::SqlitePool;

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS intent_log(
  id TEXT PRIMARY KEY, kind TEXT NOT NULL DEFAULT '',
  repo TEXT NOT NULL DEFAULT '', number INTEGER NOT NULL DEFAULT 0,
  result TEXT NOT NULL DEFAULT '', created_at TEXT NOT NULL DEFAULT ''
);";
#[derive(Debug, Clone)]
pub struct IntentStore {
    pool: SqlitePool,
}

impl IntentStore {
    pub async fn open(url: &str) -> Result<Self> {
        let pool = SqlitePool::connect(url).await?;
        sqlx::query(SCHEMA).execute(&pool).await?;
        Ok(Self { pool })
    }

    /// Stored result for a replayed id, if this daemon applied it before.
    pub async fn prior(&self, id: &str) -> Result<Option<String>> {
        let row: Option<(String,)> = sqlx::query_as("SELECT result FROM intent_log WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(|(r,)| r))
    }

    pub(crate) async fn record(
        &self,
        id: &str,
        kind: &str,
        repo: &str,
        n: i64,
        result: &str,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO intent_log(id, kind, repo, number, result, created_at)
             VALUES(?, ?, ?, ?, ?, datetime('now'))
             ON CONFLICT(id) DO UPDATE SET result = excluded.result",
        )
        .bind(id)
        .bind(kind)
        .bind(repo)
        .bind(n)
        .bind(result)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
