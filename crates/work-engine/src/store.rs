//! SQLite session store (sqlx). Files are truth for code;
//! this DB is truth for conversations. Index rebuild = re-read rows.
use super::session::{Message, Session};
use anyhow::Result;
use sqlx::SqlitePool;

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS sessions(
  id TEXT PRIMARY KEY, agent TEXT NOT NULL, model TEXT NOT NULL,
  input_tokens INTEGER NOT NULL DEFAULT 0, output_tokens INTEGER NOT NULL DEFAULT 0,
  cost_usd REAL NOT NULL DEFAULT 0.0, created_at TEXT NOT NULL DEFAULT ''
);
CREATE TABLE IF NOT EXISTS messages(
  id TEXT PRIMARY KEY, session_id TEXT NOT NULL, role TEXT NOT NULL,
  body TEXT NOT NULL DEFAULT '', input_tokens INTEGER NOT NULL DEFAULT 0,
  output_tokens INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_messages_session ON messages(session_id);";

#[derive(Debug, Clone)]
pub struct Store {
    pool: SqlitePool,
}
impl Store {
    pub async fn open(url: &str) -> Result<Self> {
        let pool = SqlitePool::connect(url).await?;
        sqlx::query(SCHEMA).execute(&pool).await?;
        Ok(Self { pool })
    }

    pub async fn create_session(&self, id: &str, agent: &str, model: &str) -> Result<()> {
        sqlx::query("INSERT INTO sessions(id, agent, model) VALUES(?, ?, ?)")
            .bind(id)
            .bind(agent)
            .bind(model)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn add_message(&self, m: &Message) -> Result<()> {
        sqlx::query(
            "INSERT INTO messages(id, session_id, role, body, input_tokens, output_tokens)
             VALUES(?, ?, ?, ?, ?, ?)",
        )
        .bind(&m.id)
        .bind(&m.session_id)
        .bind(&m.role)
        .bind(&m.body)
        .bind(m.input_tokens)
        .bind(m.output_tokens)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn messages(&self, session: &str) -> Result<Vec<Message>> {
        let rows = sqlx::query_as::<_, (String, String, String, String, i64, i64)>(
            "SELECT id, session_id, role, body, input_tokens, output_tokens
             FROM messages WHERE session_id = ? ORDER BY rowid",
        )
        .bind(session)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(
                |(id, session_id, role, body, input_tokens, output_tokens)| Message {
                    id,
                    session_id,
                    role,
                    body,
                    input_tokens,
                    output_tokens,
                },
            )
            .collect())
    }

    /// Add provider-reported usage to the session ledger (Bet 10 reads this).
    pub async fn record_usage(
        &self,
        session: &str,
        input: i64,
        output: i64,
        cost: f64,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE sessions SET input_tokens = input_tokens + ?,
             output_tokens = output_tokens + ?, cost_usd = cost_usd + ? WHERE id = ?",
        )
        .bind(input)
        .bind(output)
        .bind(cost)
        .bind(session)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Every session, oldest first — the resume scan walks this.
    pub async fn all_sessions(&self) -> Result<Vec<Session>> {
        let rows = sqlx::query_as::<_, (String, String, String, i64, i64, f64)>(
            "SELECT id, agent, model, input_tokens, output_tokens, cost_usd
             FROM sessions ORDER BY rowid",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(
                |(id, agent, model, input_tokens, output_tokens, cost_usd)| Session {
                    id,
                    agent,
                    model,
                    input_tokens,
                    output_tokens,
                    cost_usd,
                },
            )
            .collect())
    }

    pub async fn session(&self, id: &str) -> Result<Option<Session>> {
        let row = sqlx::query_as::<_, (String, String, String, i64, i64, f64)>(
            "SELECT id, agent, model, input_tokens, output_tokens, cost_usd
             FROM sessions WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(
            |(id, agent, model, input_tokens, output_tokens, cost_usd)| Session {
                id,
                agent,
                model,
                input_tokens,
                output_tokens,
                cost_usd,
            },
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msg(id: &str, s: &str, role: &str, body: &str) -> Message {
        Message {
            id: id.into(),
            session_id: s.into(),
            role: role.into(),
            body: body.into(),
            input_tokens: 0,
            output_tokens: 0,
        }
    }

    #[tokio::test]
    async fn roundtrip_and_ledger() {
        let store = Store::open("sqlite::memory:").await.unwrap();
        store
            .create_session("s1", "build", "acme/m1")
            .await
            .unwrap();
        store
            .add_message(&msg("m1", "s1", "user", "hi"))
            .await
            .unwrap();
        store
            .add_message(&msg("m2", "s1", "assistant", "hello"))
            .await
            .unwrap();
        assert_eq!(store.messages("s1").await.unwrap().len(), 2);
        store.record_usage("s1", 100, 50, 0.02).await.unwrap();
        let s = store.session("s1").await.unwrap().unwrap();
        assert_eq!((s.input_tokens, s.output_tokens), (100, 50));
        assert!((s.cost_usd - 0.02).abs() < 1e-9);
    }
}
