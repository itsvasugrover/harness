//! todo: session-local task list (add/list/done).
//! File-backed under `<workdir>/.todos/<session>.json`; no daemon needed.
use super::super::tool::{Tool, ToolCtx, ToolOutput};
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Item {
    id: u64,
    text: String,
    done: bool,
}

fn path_for(ctx: &ToolCtx) -> String {
    format!(
        "{}/.todos/{}.json",
        ctx.workdir.trim_end_matches('/'),
        ctx.session_id
    )
}

fn load(ctx: &ToolCtx) -> Vec<Item> {
    std::fs::read_to_string(path_for(ctx))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn save(ctx: &ToolCtx, items: &[Item]) -> Result<()> {
    let path = path_for(ctx);
    if let Some(parent) = std::path::Path::new(&path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, serde_json::to_string_pretty(items)?)?;
    Ok(())
}

pub struct Todo;

impl Tool for Todo {
    fn name(&self) -> &'static str {
        "todo"
    }

    fn description(&self) -> &'static str {
        "Session todos as JSON: {op:add|list|done, text?, id?}."
    }

    fn run(&self, ctx: &ToolCtx, input: &str) -> Result<ToolOutput> {
        let v: serde_json::Value =
            serde_json::from_str(input).map_err(|_| anyhow::anyhow!("todo: input must be JSON"))?;
        let op = v.get("op").and_then(|x| x.as_str()).unwrap_or("list");
        let mut items = load(ctx);
        let out = match op {
            "add" => {
                let text = v.get("text").and_then(|x| x.as_str()).unwrap_or("").trim();
                if text.is_empty() {
                    bail!("todo: add needs {{op:add, text}}");
                }
                let id = items.iter().map(|i| i.id).max().unwrap_or(0) + 1;
                items.push(Item {
                    id,
                    text: text.into(),
                    done: false,
                });
                save(ctx, &items)?;
                format!("added #{id}")
            }
            "done" => {
                let id = v.get("id").and_then(|x| x.as_u64()).unwrap_or(0);
                let Some(item) = items.iter_mut().find(|i| i.id == id) else {
                    bail!("todo: unknown id {id}");
                };
                item.done = true;
                save(ctx, &items)?;
                format!("done #{id}")
            }
            "list" => {
                if items.is_empty() {
                    "no todos".into()
                } else {
                    items
                        .iter()
                        .map(|i| {
                            format!("#{} [{}] {}", i.id, if i.done { "x" } else { " " }, i.text)
                        })
                        .collect::<Vec<_>>()
                        .join("\n")
                }
            }
            other => bail!("todo: unknown op '{other}' (add|list|done)"),
        };
        Ok(ToolOutput {
            title: format!("todo {op}"),
            output: super::super::tool::truncate(&out),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> ToolCtx {
        let dir = std::env::temp_dir().join(format!("harness-todo-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        ToolCtx {
            session_id: "s1".into(),
            workdir: dir.to_string_lossy().into_owned(),
        }
    }

    #[test]
    fn add_list_done_roundtrip() {
        let c = ctx();
        let t = Todo;
        assert!(t.run(&c, r#"{"op":"add","text":"write test"}"#).is_ok());
        let list = t.run(&c, r#"{"op":"list"}"#).unwrap();
        assert!(list.output.contains("write test"));
        assert!(t.run(&c, r#"{"op":"done","id":1}"#).is_ok());
        let list2 = t.run(&c, r#"{"op":"list"}"#).unwrap();
        assert!(list2.output.contains("[x]"));
        let _ = std::fs::remove_dir_all(&c.workdir);
    }
}
