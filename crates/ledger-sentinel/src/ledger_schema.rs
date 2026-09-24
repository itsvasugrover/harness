//! Ledger schema: tables plus additive migration for DBs created
//! before a column existed. Fresh tables carry every column; old ones
//! gain what's missing. Only the fixed known columns are ever added,
//! so the statements stay static and injection-safe.
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

/// Create tables, then backfill columns on pre-existing DBs.
/// Duplicate-column errors mean "already there".
pub async fn init_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query(SCHEMA).execute(pool).await?;
    for col in ["summary", "agent", "skill", "mcp_server", "refs"] {
        let ddl: &'static str = match col {
            "summary" => "ALTER TABLE audit_index ADD COLUMN summary TEXT NOT NULL DEFAULT ''",
            "agent" => "ALTER TABLE audit_index ADD COLUMN agent TEXT NOT NULL DEFAULT ''",
            "skill" => "ALTER TABLE audit_index ADD COLUMN skill TEXT NOT NULL DEFAULT ''",
            "mcp_server" => {
                "ALTER TABLE audit_index ADD COLUMN mcp_server TEXT NOT NULL DEFAULT ''"
            }
            "refs" => "ALTER TABLE audit_index ADD COLUMN refs TEXT NOT NULL DEFAULT '[]'",
            _ => continue,
        };
        match sqlx::query(ddl).execute(pool).await {
            Ok(_) => {}
            Err(e) if e.to_string().contains("duplicate column name") => {}
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}
