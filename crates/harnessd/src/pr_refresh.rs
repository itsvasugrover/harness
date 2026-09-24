//! Served PR cards: boot hydration plus a background refresh task.
//! Split from main.rs per the 300-line rule. Ticks publish a board
//! event only when the PR set actually changes, keeping the bus quiet.
use anyhow::Result;

/// Best-effort hydration so the unified board has rows on boot.
/// The observer keeps mirroring after; failures stay empty and never
/// fail boot.
pub async fn hydrate(
    prs: &tokio::sync::RwLock<Vec<crate::forge_facts::PrCard>>,
    forges: &[crate::config_sections::ForgeCfg],
    db_url: &str,
) -> Result<()> {
    let facts_db = crate::forge_facts::ForgeFacts::open(db_url).await?;
    let mut all_prs = vec![];
    for f in forges {
        for repo in &f.repos {
            if let Ok(cards) = facts_db.pr_cards(repo).await {
                all_prs.extend(cards);
            }
        }
    }
    *prs.write().await = all_prs;
    Ok(())
}

/// Background refresh without blocking boot.
pub fn spawn(
    prs: std::sync::Arc<tokio::sync::RwLock<Vec<crate::forge_facts::PrCard>>>,
    hub: crate::events::Hub,
    forges: Vec<crate::config_sections::ForgeCfg>,
    db_url: String,
) {
    tokio::spawn(async move {
        let Ok(facts_db) = crate::forge_facts::ForgeFacts::open(&db_url).await else {
            return;
        };
        let mut last = serde_json::to_string(&*prs.read().await).unwrap_or_default();
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            let mut all = vec![];
            for f in &forges {
                for repo in &f.repos {
                    if let Ok(cards) = facts_db.pr_cards(repo).await {
                        all.extend(cards);
                    }
                }
            }
            let snapshot = serde_json::to_string(&all).unwrap_or_default();
            if snapshot != last {
                last = snapshot;
                *prs.write().await = all;
                hub.publish("board", "pr facts refreshed").await;
            }
        }
    });
}
