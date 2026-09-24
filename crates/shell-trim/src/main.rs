//! trim: stdin -> filter -> stdout, meter on stderr, JSONL ledger.
//! Hook auto-rewrite mapping lives in hook.rs; per-agent installers Phase 3.
mod cmds;
mod hook;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use std::io::Read as _;

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Filter {
    GitStatus,
    GitDiff,
    GitLog,
    Test,
    Tree,
    Read,
}

#[derive(Debug, Subcommand)]
enum Cmd {
    /// Filter stdin through a trim filter.
    Run {
        filter: Filter,
        /// JSONL ledger path; when set, this invocation is recorded.
        #[arg(long)]
        log: Option<String>,
    },
    /// Show savings totals from a JSONL ledger.
    Gain {
        #[arg(long)]
        log: String,
    },
    /// Print the trim rewrite for a shell command (hook helper).
    Rewrite {
        cmd: String,
        #[arg(long, default_value = "")]
        exclude: String,
        /// When set, log the verdict for `discover` ranking.
        #[arg(long)]
        log: Option<String>,
    },
    /// Rank unrewritten commands: which filter to write next.
    Discover {
        #[arg(long)]
        log: String,
        #[arg(long, default_value = "10")]
        top: usize,
    },
}

#[derive(Debug, Parser)]
#[command(name = "trim", version, about = "Shell-output trimmer")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

fn main() -> Result<()> {
    match Cli::parse().cmd {
        Cmd::Run { filter, log } => {
            let mut input = String::new();
            std::io::stdin()
                .read_to_string(&mut input)
                .context("read stdin")?;
            let bytes_in = input.len() as u64;
            let output = match filter {
                Filter::GitStatus => cmds::git::compact_status(&input),
                Filter::GitDiff => cmds::git::condense_diff(&input),
                Filter::GitLog => cmds::git::oneline_log(&input),
                Filter::Test => cmds::tests::failures_only(&input),
                Filter::Tree => {
                    let paths: Vec<&str> = input.lines().collect();
                    cmds::files::tree(&paths)
                }
                Filter::Read => cmds::files::shape_read(&input),
            };
            if let Some(path) = log {
                let name = format!("{filter:?}");
                cmds::tracking::append_ledger(
                    &path,
                    &cmds::tracking::Entry {
                        filter: name,
                        bytes_in,
                        bytes_out: output.len() as u64,
                        note: String::new(),
                    },
                )?;
            }
            let stats = cmds::tracking::Stats {
                bytes_in,
                bytes_out: output.len() as u64,
            };
            print!("{output}");
            eprintln!("{}", stats.report());
            Ok(())
        }
        Cmd::Gain { log } => {
            let g = cmds::tracking::summarize(&log);
            let saved = g.bytes_in.saturating_sub(g.bytes_out);
            println!(
                "gain: {} invocations, {} -> {} bytes (~{} tokens saved*) *estimate",
                g.invocations,
                g.bytes_in,
                g.bytes_out,
                saved / 4
            );
            Ok(())
        }
        Cmd::Rewrite { cmd, exclude, log } => {
            let xs: Vec<&str> = exclude
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .collect();
            let rewritten = hook::rewrite(&cmd, &xs);
            if let Some(path) = log {
                let note = if rewritten == cmd {
                    "passthrough".to_string()
                } else {
                    format!(
                        "rewritten:{}",
                        rewritten.strip_prefix("trim ").unwrap_or(&rewritten)
                    )
                };
                cmds::tracking::append_ledger(
                    &path,
                    &cmds::tracking::Entry {
                        filter: cmd.clone(),
                        bytes_in: 0,
                        bytes_out: 0,
                        note,
                    },
                )?;
            }
            println!("{rewritten}");
            Ok(())
        }
        Cmd::Discover { log, top } => {
            for (cmd, count) in cmds::tracking::discover(&log, top) {
                println!("{count:>5}  {cmd}");
            }
            Ok(())
        }
    }
}
