//! Forge facts: the observer's SQLite mirror of PR metadata, check
//! runs, and review threads. The Kanban derives columns from these rows;
//! nothing here is display state. Inline schema like the session store
//! (file migrations arrive with the audit tables).
use anyhow::Result;
use sqlx::SqlitePool;

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS pr_facts(
  repo TEXT NOT NULL, number INTEGER NOT NULL, title TEXT NOT NULL DEFAULT '',
  state TEXT NOT NULL DEFAULT '', head_sha TEXT NOT NULL DEFAULT '',
  mergeable INTEGER NOT NULL DEFAULT 0, updated_at TEXT NOT NULL DEFAULT '',
  PRIMARY KEY(repo, number)
);
CREATE TABLE IF NOT EXISTS check_facts(
  repo TEXT NOT NULL, sha TEXT NOT NULL, name TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT '', conclusion TEXT NOT NULL DEFAULT '',
  PRIMARY KEY(repo, sha, name)
);
CREATE TABLE IF NOT EXISTS review_facts(
  repo TEXT NOT NULL, number INTEGER NOT NULL, thread_id TEXT NOT NULL,
  resolved INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY(repo, number, thread_id)
);";

#[derive(Debug, Clone)]
pub struct ForgeFacts {
    pool: SqlitePool,
}

impl ForgeFacts {
    pub async fn open(url: &str) -> Result<Self> {
        let pool = SqlitePool::connect(url).await?;
        sqlx::query(SCHEMA).execute(&pool).await?;
        Ok(Self { pool })
    }

    pub async fn upsert_pr(
        &self,
        repo: &str,
        number: i64,
        title: &str,
        state: &str,
        head_sha: &str,
        mergeable: bool,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO pr_facts(repo, number, title, state, head_sha, mergeable, updated_at)
             VALUES(?, ?, ?, ?, ?, ?, datetime('now'))
             ON CONFLICT(repo, number) DO UPDATE SET
               title = excluded.title, state = excluded.state,
               head_sha = excluded.head_sha, mergeable = excluded.mergeable,
               updated_at = excluded.updated_at",
        )
        .bind(repo)
        .bind(number)
        .bind(title)
        .bind(state)
        .bind(head_sha)
        .bind(mergeable as i64)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Replace all checks for a sha (stale rows would lie to the Kanban).
    pub async fn replace_checks(
        &self,
        repo: &str,
        sha: &str,
        checks: &[(String, String, String)],
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM check_facts WHERE repo = ? AND sha = ?")
            .bind(repo)
            .bind(sha)
            .execute(&mut *tx)
            .await?;
        for (name, status, conclusion) in checks {
            sqlx::query(
                "INSERT INTO check_facts(repo, sha, name, status, conclusion)
                 VALUES(?, ?, ?, ?, ?)",
            )
            .bind(repo)
            .bind(sha)
            .bind(name)
            .bind(status)
            .bind(conclusion)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    /// Replace all threads for a PR for the same staleness reason.
    pub async fn replace_threads(
        &self,
        repo: &str,
        number: i64,
        threads: &[(String, bool)],
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM review_facts WHERE repo = ? AND number = ?")
            .bind(repo)
            .bind(number)
            .execute(&mut *tx)
            .await?;
        for (thread_id, resolved) in threads {
            sqlx::query(
                "INSERT INTO review_facts(repo, number, thread_id, resolved)
                 VALUES(?, ?, ?, ?)",
            )
            .bind(repo)
            .bind(number)
            .bind(thread_id)
            .bind(*resolved as i64)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    pub async fn failing_checks(&self, repo: &str, sha: &str) -> Result<Vec<String>> {
        let rows = sqlx::query_as::<_, (String,)>(
            "SELECT name FROM check_facts WHERE repo = ? AND sha = ?
             AND conclusion IN ('failure', 'cancelled', 'timed_out', 'action_required')",
        )
        .bind(repo)
        .bind(sha)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|(n,)| n).collect())
    }

    pub async fn unresolved_threads(&self, repo: &str, number: i64) -> Result<i64> {
        let (n,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM review_facts
             WHERE repo = ? AND number = ? AND resolved = 0",
        )
        .bind(repo)
        .bind(number)
        .fetch_one(&self.pool)
        .await?;
        Ok(n)
    }

    /// One Kanban-ready row per stored PR: facts joined so the board
    /// derives columns without touching the forge.
    #[allow(dead_code)] // board API exposure consumes this in Phase 5.
    pub async fn pr_cards(&self, repo: &str) -> Result<Vec<PrCard>> {
        let rows = sqlx::query_as::<_, (i64, String, String, String, i64)>(
            "SELECT number, title, state, head_sha, mergeable FROM pr_facts
             WHERE repo = ? ORDER BY number",
        )
        .bind(repo)
        .fetch_all(&self.pool)
        .await?;
        let mut cards = vec![];
        for (number, title, state, head_sha, mergeable) in rows {
            let checks_green = self.failing_checks(repo, &head_sha).await?.is_empty();
            let unresolved = self.unresolved_threads(repo, number).await?;
            cards.push(PrCard {
                number,
                title,
                state,
                mergeable: mergeable != 0,
                checks_green,
                unresolved,
            });
        }
        Ok(cards)
    }
}

/// Board input for one PR. `checks_green` is true when the forge
/// reports no failing checks (absent checks read green).
#[allow(dead_code)] // board API exposure consumes this in Phase 5.
#[derive(Debug, Clone)]
pub struct PrCard {
    pub number: i64,
    pub title: String,
    pub state: String,
    pub mergeable: bool,
    pub checks_green: bool,
    pub unresolved: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn upsert_and_route_followups() {
        let facts = ForgeFacts::open("sqlite::memory:").await.unwrap();
        facts
            .upsert_pr("o/r", 3, "fix", "open", "abc", true)
            .await
            .unwrap();
        facts
            .replace_checks(
                "o/r",
                "abc",
                &[("ci".into(), "done".into(), "failure".into())],
            )
            .await
            .unwrap();
        facts
            .replace_threads("o/r", 3, &[("t1".into(), false)])
            .await
            .unwrap();
        assert_eq!(
            facts.failing_checks("o/r", "abc").await.unwrap(),
            vec!["ci"]
        );
        assert_eq!(facts.unresolved_threads("o/r", 3).await.unwrap(), 1);
        facts
            .replace_checks(
                "o/r",
                "abc",
                &[("ci".into(), "done".into(), "success".into())],
            )
            .await
            .unwrap();
        assert!(facts.failing_checks("o/r", "abc").await.unwrap().is_empty());
    }
}
