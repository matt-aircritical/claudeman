mod config;
mod indexer;
mod names;
mod parser;
mod resume;
mod scanner;
mod search;
mod session;
mod tui;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "claudeman", version, about = "Claude Code session manager")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Search sessions (opens TUI with search pre-filled)
    Search { query: String },
    /// List all sessions to stdout (non-interactive)
    List,
    /// Force a full re-index of all sessions
    Index,
    /// Show config path and current values
    Config,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = config::Config::load()?;

    match cli.command {
        None => {
            let idx = run_index(&config)?;
            let name_store = names::NameStore::load()?;
            let sessions = idx.all_sessions()?;
            if sessions.is_empty() {
                println!("No sessions found. Start a Claude session first, then run claudeman.");
                return Ok(());
            }
            let app = tui::App::new(sessions, name_store, config.clone(), idx);
            if let Some(resume_opts) = tui::run(app)? {
                resume::resume(&config, &resume_opts)?;
            }
        }
        Some(Commands::Search { query }) => {
            let idx = run_index(&config)?;
            let name_store = names::NameStore::load()?;
            let sessions = idx.all_sessions()?;
            let app = tui::App::new_with_search(sessions, name_store, config.clone(), idx, query);
            if let Some(resume_opts) = tui::run(app)? {
                resume::resume(&config, &resume_opts)?;
            }
        }
        Some(Commands::List) => {
            let idx = run_index(&config)?;
            let name_store = names::NameStore::load()?;
            let sessions = idx.all_sessions()?;
            for session in &sessions {
                let name = search::display_name(session, &name_store);
                let date = search::format_date(session.last_activity);
                println!("{} | {:>8} | {:>4} msgs | {}", &session.session_id[..8], date, session.message_count, name);
            }
            println!("\n{} sessions total", sessions.len());
        }
        Some(Commands::Index) => {
            run_full_index(&config)?;
        }
        Some(Commands::Config) => {
            println!("Config path: {}", config::Config::config_path().display());
            println!("Index path:  {}", config::Config::index_dir().display());
            println!("Claude bin:  {}", config.claude_bin);
            println!("Claude args: {:?}", config.claude_args);
            println!("Claude dir:  {}", config.claude_dir().display());
        }
    }
    Ok(())
}

fn run_index(config: &config::Config) -> Result<indexer::SessionIndex> {
    let index_dir = config::Config::index_dir();
    let idx = indexer::SessionIndex::create(&index_dir)?;
    let discovered = scanner::discover_sessions(&config.claude_dir());
    let mut indexed = 0;
    for d in &discovered {
        if idx.needs_reindex(&d.session_id, d.file_mtime)? {
            match parser::parse_session(d) {
                Ok(session) => { idx.add_session(&session)?; indexed += 1; }
                Err(e) => { eprintln!("Warning: failed to parse {}: {}", d.jsonl_path, e); }
            }
        }
    }
    if indexed > 0 {
        idx.commit()?;
        eprintln!("Indexed {} new/updated sessions", indexed);
    }
    Ok(idx)
}

fn run_full_index(config: &config::Config) -> Result<()> {
    let index_dir = config::Config::index_dir();
    if index_dir.exists() { std::fs::remove_dir_all(&index_dir)?; }
    let idx = indexer::SessionIndex::create(&index_dir)?;
    let discovered = scanner::discover_sessions(&config.claude_dir());
    let mut indexed = 0;
    for d in &discovered {
        match parser::parse_session(d) {
            Ok(session) => { idx.add_session(&session)?; indexed += 1; }
            Err(e) => { eprintln!("Warning: failed to parse {}: {}", d.jsonl_path, e); }
        }
    }
    idx.commit()?;
    eprintln!("Re-indexed {} sessions", indexed);
    Ok(())
}
