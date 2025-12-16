// SPDX-License-Identifier: MIT OR AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2025 Jonathan D.A. Jewell
//
// JanusKey CLI: Provably Reversible File Operations
// "Never lose data again"

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::Colorize;
use dialoguer::Confirm;
use indicatif::{ProgressBar, ProgressStyle};
use januskey::{
    operations::{FileOperation, OperationExecutor},
    transaction::TransactionPreview,
    JanusKey,
};
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "jk",
    version = "1.0.0",
    author = "Jonathan D.A. Jewell <jonathan.jewell@gmail.com>",
    about = "JanusKey: Provably reversible file operations",
    long_about = "JanusKey makes every file operation reversible through Maximal Principle Reduction.\n\
                  Delete files, modify content, move things around—and always be able to undo.\n\n\
                  Data loss is architecturally impossible."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Working directory (defaults to current directory)
    #[arg(short = 'C', long, global = true)]
    dir: Option<PathBuf>,

    /// Dry run mode (don't actually make changes)
    #[arg(long, global = true)]
    dry_run: bool,

    /// Skip confirmation prompts
    #[arg(short = 'y', long, global = true)]
    yes: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize JanusKey in the current directory
    Init,

    /// Delete files (reversible)
    #[command(alias = "rm")]
    Delete {
        /// Files or glob patterns to delete
        #[arg(required = true)]
        paths: Vec<String>,

        /// Delete recursively (for directories)
        #[arg(short, long)]
        recursive: bool,
    },

    /// Modify files with sed-like syntax or a script (reversible)
    Modify {
        /// Sed-like pattern (s/old/new/g) or script path
        pattern: String,

        /// Files to modify
        #[arg(required = true)]
        paths: Vec<String>,
    },

    /// Move or rename files (reversible)
    #[command(alias = "mv")]
    Move {
        /// Source file(s)
        source: String,

        /// Destination
        destination: PathBuf,
    },

    /// Copy files (reversible - the copy can be deleted)
    #[command(alias = "cp")]
    Copy {
        /// Source file
        source: PathBuf,

        /// Destination
        destination: PathBuf,
    },

    /// Rename a file (reversible)
    Rename {
        /// Original name
        old_name: PathBuf,

        /// New name
        new_name: PathBuf,
    },

    /// Undo the last operation(s)
    Undo {
        /// Number of operations to undo
        #[arg(short, long, default_value = "1")]
        count: usize,

        /// Undo a specific operation by ID
        #[arg(long)]
        id: Option<String>,
    },

    /// Begin a new transaction
    Begin {
        /// Optional name for the transaction
        name: Option<String>,
    },

    /// Commit the current transaction
    Commit,

    /// Rollback the current transaction
    Rollback,

    /// Preview pending changes in current transaction
    Preview,

    /// Show operation history
    History {
        /// Number of entries to show
        #[arg(short, long, default_value = "20")]
        limit: usize,

        /// Filter by operation type (DELETE, MODIFY, MOVE, COPY)
        #[arg(short, long)]
        filter: Option<String>,
    },

    /// Show current status
    Status,

    /// Garbage collect old operations
    Gc {
        /// Keep only the last N operations
        #[arg(long)]
        keep: Option<usize>,

        /// Delete operations older than N days
        #[arg(long)]
        older_than: Option<u32>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Determine working directory
    let working_dir = cli.dir.unwrap_or_else(|| std::env::current_dir().unwrap());

    match cli.command {
        Commands::Init => cmd_init(&working_dir),
        Commands::Delete { paths, recursive } => {
            cmd_delete(&working_dir, &paths, recursive, cli.dry_run, cli.yes)
        }
        Commands::Modify { pattern, paths } => {
            cmd_modify(&working_dir, &pattern, &paths, cli.dry_run, cli.yes)
        }
        Commands::Move { source, destination } => {
            cmd_move(&working_dir, &source, &destination, cli.dry_run)
        }
        Commands::Copy { source, destination } => {
            cmd_copy(&working_dir, &source, &destination, cli.dry_run)
        }
        Commands::Rename { old_name, new_name } => {
            cmd_move(&working_dir, &old_name.to_string_lossy(), &new_name, cli.dry_run)
        }
        Commands::Undo { count, id } => cmd_undo(&working_dir, count, id),
        Commands::Begin { name } => cmd_begin(&working_dir, name),
        Commands::Commit => cmd_commit(&working_dir),
        Commands::Rollback => cmd_rollback(&working_dir),
        Commands::Preview => cmd_preview(&working_dir),
        Commands::History { limit, filter } => cmd_history(&working_dir, limit, filter),
        Commands::Status => cmd_status(&working_dir),
        Commands::Gc { keep, older_than } => cmd_gc(&working_dir, keep, older_than),
    }
}

fn cmd_init(dir: &PathBuf) -> Result<()> {
    if JanusKey::is_initialized(dir) {
        println!(
            "{} JanusKey already initialized in {}",
            "✓".green(),
            dir.display()
        );
        return Ok(());
    }

    JanusKey::init(dir).context("Failed to initialize JanusKey")?;
    println!(
        "{} JanusKey initialized in {}",
        "✓".green(),
        dir.display()
    );
    println!("  Metadata stored in: {}/.januskey/", dir.display());
    println!("\n  You can now use reversible file operations:");
    println!("    jk delete <files>    - Delete files (reversible)");
    println!("    jk modify <pattern> <files> - Modify files (reversible)");
    println!("    jk move <src> <dst>  - Move files (reversible)");
    println!("    jk undo              - Undo last operation");
    Ok(())
}

fn cmd_delete(
    dir: &PathBuf,
    paths: &[String],
    recursive: bool,
    dry_run: bool,
    auto_yes: bool,
) -> Result<()> {
    let mut jk = JanusKey::open(dir).context("Failed to open JanusKey directory")?;

    // Expand glob patterns and collect files
    let mut files_to_delete = Vec::new();
    for pattern in paths {
        let full_pattern = dir.join(pattern);
        let pattern_str = full_pattern.to_string_lossy();
        for entry in glob::glob(&pattern_str)? {
            let path = entry?;
            if path.is_file() {
                files_to_delete.push(path);
            } else if path.is_dir() && recursive {
                // Collect all files in directory
                for entry in walkdir::WalkDir::new(&path) {
                    let entry = entry?;
                    if entry.file_type().is_file() {
                        files_to_delete.push(entry.path().to_path_buf());
                    }
                }
            }
        }
    }

    if files_to_delete.is_empty() {
        println!("{} No files matched the pattern(s)", "!".yellow());
        return Ok(());
    }

    // Show what will be deleted
    if dry_run {
        println!("{} Dry run - would delete:", "[DRY RUN]".cyan());
        for file in &files_to_delete {
            println!("  - {}", file.display());
        }
        return Ok(());
    }

    // Confirm if many files
    if files_to_delete.len() > 10 && !auto_yes {
        println!(
            "{} This will delete {} files:",
            "⚠".yellow(),
            files_to_delete.len()
        );
        for file in files_to_delete.iter().take(5) {
            println!("  - {}", file.display());
        }
        if files_to_delete.len() > 5 {
            println!("  ... and {} more", files_to_delete.len() - 5);
        }
        if !Confirm::new()
            .with_prompt("Continue?")
            .default(false)
            .interact()?
        {
            println!("{}", "Cancelled".red());
            return Ok(());
        }
    }

    let transaction_id = jk.transaction_manager.active_id().map(String::from);

    // Progress bar for multiple files
    let progress = if files_to_delete.len() > 1 {
        let pb = ProgressBar::new(files_to_delete.len() as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
                .unwrap()
                .progress_chars("#>-"),
        );
        Some(pb)
    } else {
        None
    };

    let mut deleted_count = 0;
    for path in &files_to_delete {
        let mut executor = OperationExecutor::new(&jk.content_store, &mut jk.metadata_store);
        if let Some(ref tid) = transaction_id {
            executor = executor.with_transaction(tid.clone());
        }

        match executor.execute(FileOperation::Delete { path: path.clone() }) {
            Ok(meta) => {
                deleted_count += 1;
                if let Some(ref pb) = progress {
                    pb.inc(1);
                    pb.set_message(format!("{}", path.file_name().unwrap_or_default().to_string_lossy()));
                }
                // Record in transaction if active
                if transaction_id.is_some() {
                    jk.transaction_manager.add_operation(meta.id)?;
                }
            }
            Err(e) => {
                eprintln!("{} Failed to delete {}: {}", "✗".red(), path.display(), e);
            }
        }
    }

    if let Some(pb) = progress {
        pb.finish_and_clear();
    }

    println!(
        "{} Deleted {} file(s)",
        "✓".green(),
        deleted_count
    );
    println!("  Use {} to restore", "jk undo".cyan());

    Ok(())
}

fn cmd_modify(
    dir: &PathBuf,
    pattern: &str,
    paths: &[String],
    dry_run: bool,
    auto_yes: bool,
) -> Result<()> {
    let mut jk = JanusKey::open(dir).context("Failed to open JanusKey directory")?;

    // Parse sed-like pattern: s/old/new/g
    let (search, replace, global) = parse_sed_pattern(pattern)?;

    // Expand glob patterns
    let mut files = Vec::new();
    for p in paths {
        let full_pattern = dir.join(p);
        let pattern_str = full_pattern.to_string_lossy();
        for entry in glob::glob(&pattern_str)? {
            let path = entry?;
            if path.is_file() {
                files.push(path);
            }
        }
    }

    if files.is_empty() {
        println!("{} No files matched the pattern(s)", "!".yellow());
        return Ok(());
    }

    // Preview changes
    let mut changes = Vec::new();
    for file in &files {
        let content = fs::read_to_string(file)?;
        let new_content = if global {
            content.replace(&search, &replace)
        } else {
            content.replacen(&search, &replace, 1)
        };
        if content != new_content {
            changes.push((file.clone(), new_content));
        }
    }

    if changes.is_empty() {
        println!("{} No changes would be made", "!".yellow());
        return Ok(());
    }

    if dry_run {
        println!("{} Dry run - would modify:", "[DRY RUN]".cyan());
        for (file, _) in &changes {
            println!("  - {}", file.display());
        }
        return Ok(());
    }

    // Confirm
    if changes.len() > 5 && !auto_yes {
        println!(
            "{} This will modify {} files",
            "⚠".yellow(),
            changes.len()
        );
        if !Confirm::new()
            .with_prompt("Continue?")
            .default(false)
            .interact()?
        {
            println!("{}", "Cancelled".red());
            return Ok(());
        }
    }

    let transaction_id = jk.transaction_manager.active_id().map(String::from);

    for (file, new_content) in changes {
        let mut executor = OperationExecutor::new(&jk.content_store, &mut jk.metadata_store);
        if let Some(ref tid) = transaction_id {
            executor = executor.with_transaction(tid.clone());
        }

        match executor.execute(FileOperation::Modify {
            path: file.clone(),
            new_content: new_content.into_bytes(),
        }) {
            Ok(meta) => {
                println!("  {} {}", "✓".green(), file.display());
                if transaction_id.is_some() {
                    jk.transaction_manager.add_operation(meta.id)?;
                }
            }
            Err(e) => {
                eprintln!("  {} {}: {}", "✗".red(), file.display(), e);
            }
        }
    }

    println!("  Use {} to restore original content", "jk undo".cyan());

    Ok(())
}

fn parse_sed_pattern(pattern: &str) -> Result<(String, String, bool)> {
    // Parse s/old/new/g pattern
    if !pattern.starts_with("s/") {
        anyhow::bail!("Pattern must be in format: s/search/replace/[g]");
    }

    let rest = &pattern[2..];
    let parts: Vec<&str> = rest.split('/').collect();

    if parts.len() < 2 {
        anyhow::bail!("Invalid pattern format. Use: s/search/replace/[g]");
    }

    let search = parts[0].to_string();
    let replace = parts[1].to_string();
    let global = parts.get(2).map_or(false, |f| f.contains('g'));

    Ok((search, replace, global))
}

fn cmd_move(dir: &PathBuf, source: &str, destination: &PathBuf, dry_run: bool) -> Result<()> {
    let mut jk = JanusKey::open(dir).context("Failed to open JanusKey directory")?;

    let source_path = if PathBuf::from(source).is_absolute() {
        PathBuf::from(source)
    } else {
        dir.join(source)
    };

    let dest_path = if destination.is_absolute() {
        destination.clone()
    } else {
        dir.join(destination)
    };

    if dry_run {
        println!(
            "{} Would move {} -> {}",
            "[DRY RUN]".cyan(),
            source_path.display(),
            dest_path.display()
        );
        return Ok(());
    }

    let transaction_id = jk.transaction_manager.active_id().map(String::from);
    let mut executor = OperationExecutor::new(&jk.content_store, &mut jk.metadata_store);
    if let Some(ref tid) = transaction_id {
        executor = executor.with_transaction(tid.clone());
    }

    let meta = executor.execute(FileOperation::Move {
        source: source_path.clone(),
        destination: dest_path.clone(),
    })?;

    if transaction_id.is_some() {
        jk.transaction_manager.add_operation(meta.id)?;
    }

    println!(
        "{} Moved {} -> {}",
        "✓".green(),
        source_path.display(),
        dest_path.display()
    );
    println!("  Use {} to move back", "jk undo".cyan());

    Ok(())
}

fn cmd_copy(dir: &PathBuf, source: &PathBuf, destination: &PathBuf, dry_run: bool) -> Result<()> {
    let mut jk = JanusKey::open(dir).context("Failed to open JanusKey directory")?;

    let source_path = if source.is_absolute() {
        source.clone()
    } else {
        dir.join(source)
    };

    let dest_path = if destination.is_absolute() {
        destination.clone()
    } else {
        dir.join(destination)
    };

    if dry_run {
        println!(
            "{} Would copy {} -> {}",
            "[DRY RUN]".cyan(),
            source_path.display(),
            dest_path.display()
        );
        return Ok(());
    }

    let transaction_id = jk.transaction_manager.active_id().map(String::from);
    let mut executor = OperationExecutor::new(&jk.content_store, &mut jk.metadata_store);
    if let Some(ref tid) = transaction_id {
        executor = executor.with_transaction(tid.clone());
    }

    let meta = executor.execute(FileOperation::Copy {
        source: source_path.clone(),
        destination: dest_path.clone(),
    })?;

    if transaction_id.is_some() {
        jk.transaction_manager.add_operation(meta.id)?;
    }

    println!(
        "{} Copied {} -> {}",
        "✓".green(),
        source_path.display(),
        dest_path.display()
    );
    println!("  Use {} to delete the copy", "jk undo".cyan());

    Ok(())
}

fn cmd_undo(dir: &PathBuf, count: usize, id: Option<String>) -> Result<()> {
    let mut jk = JanusKey::open(dir).context("Failed to open JanusKey directory")?;

    if let Some(op_id) = id {
        // Undo specific operation
        let mut executor = OperationExecutor::new(&jk.content_store, &mut jk.metadata_store);
        let meta = executor.undo(&op_id)?;
        println!(
            "{} Undid {} on {}",
            "✓".green(),
            meta.op_type,
            meta.path.display()
        );
    } else {
        // Undo last N operations
        let ops_to_undo: Vec<_> = jk.metadata_store.last_n(count).into_iter().cloned().collect();

        if ops_to_undo.is_empty() {
            println!("{} Nothing to undo", "!".yellow());
            return Ok(());
        }

        for op in ops_to_undo {
            let mut executor = OperationExecutor::new(&jk.content_store, &mut jk.metadata_store);
            match executor.undo(&op.id) {
                Ok(_) => {
                    println!(
                        "{} Undid {} on {}",
                        "✓".green(),
                        op.op_type,
                        op.path.display()
                    );
                }
                Err(e) => {
                    eprintln!(
                        "{} Failed to undo {} on {}: {}",
                        "✗".red(),
                        op.op_type,
                        op.path.display(),
                        e
                    );
                }
            }
        }
    }

    Ok(())
}

fn cmd_begin(dir: &PathBuf, name: Option<String>) -> Result<()> {
    let mut jk = JanusKey::open(dir).context("Failed to open JanusKey directory")?;

    let tx = jk.transaction_manager.begin(name.clone())?;
    let display_name = name.unwrap_or_else(|| tx.id[..8].to_string());
    println!(
        "{} Started transaction: {}",
        "✓".green(),
        display_name.cyan()
    );
    println!("  Run operations, then use {} or {}", "jk commit".cyan(), "jk rollback".cyan());

    Ok(())
}

fn cmd_commit(dir: &PathBuf) -> Result<()> {
    let mut jk = JanusKey::open(dir).context("Failed to open JanusKey directory")?;

    let tx = jk.transaction_manager.commit()?;
    let display_name = tx.name.unwrap_or_else(|| tx.id[..8].to_string());
    println!(
        "{} Committed transaction: {} ({} operations)",
        "✓".green(),
        display_name.cyan(),
        tx.operation_ids.len()
    );

    Ok(())
}

fn cmd_rollback(dir: &PathBuf) -> Result<()> {
    let mut jk = JanusKey::open(dir).context("Failed to open JanusKey directory")?;

    let tx = jk
        .transaction_manager
        .rollback(&jk.content_store, &mut jk.metadata_store)?;
    let display_name = tx.name.unwrap_or_else(|| tx.id[..8].to_string());
    println!(
        "{} Rolled back transaction: {} ({} operations undone)",
        "✓".green(),
        display_name.cyan(),
        tx.operation_ids.len()
    );

    Ok(())
}

fn cmd_preview(dir: &PathBuf) -> Result<()> {
    let jk = JanusKey::open(dir).context("Failed to open JanusKey directory")?;

    let tx = jk
        .transaction_manager
        .active()
        .ok_or_else(|| anyhow::anyhow!("No active transaction"))?;

    let preview = TransactionPreview::from_transaction(tx, &jk.metadata_store);

    let name = preview.transaction_name.unwrap_or_else(|| tx.id[..8].to_string());
    println!("{} Transaction: {}", "📋".to_string(), name.cyan());
    println!("Operations pending: {}", preview.operations.len());
    println!();

    for op in &preview.operations {
        let arrow = if op.secondary_path.is_some() { " → " } else { "" };
        let secondary = op
            .secondary_path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_default();

        println!(
            "  {} {} {}{}{}",
            match op.op_type.as_str() {
                "DELETE" => "🗑".to_string(),
                "MODIFY" => "✏".to_string(),
                "MOVE" => "📦".to_string(),
                "COPY" => "📄".to_string(),
                _ => "•".to_string(),
            },
            op.op_type.yellow(),
            op.path.display(),
            arrow,
            secondary
        );
    }

    println!();
    println!("Total files affected: {}", preview.total_files_affected);
    println!();
    println!(
        "Use {} to apply or {} to cancel",
        "jk commit".cyan(),
        "jk rollback".cyan()
    );

    Ok(())
}

fn cmd_history(dir: &PathBuf, limit: usize, filter: Option<String>) -> Result<()> {
    let jk = JanusKey::open(dir).context("Failed to open JanusKey directory")?;

    let ops: Vec<_> = if let Some(ref filter_str) = filter {
        let filter_upper = filter_str.to_uppercase();
        jk.metadata_store
            .operations()
            .iter()
            .filter(|op| op.op_type.to_string() == filter_upper)
            .rev()
            .take(limit)
            .collect()
    } else {
        jk.metadata_store.operations().iter().rev().take(limit).collect()
    };

    if ops.is_empty() {
        println!("{} No operations in history", "!".yellow());
        return Ok(());
    }

    println!("{}", "Operation History".bold());
    println!("{}", "─".repeat(70));

    for op in ops {
        let status = if op.undone {
            "[UNDONE]".dimmed()
        } else {
            "".normal()
        };

        let time = op.timestamp.format("%Y-%m-%d %H:%M:%S");
        let op_type = match op.op_type.to_string().as_str() {
            "DELETE" => "DELETE".red(),
            "MODIFY" => "MODIFY".yellow(),
            "MOVE" => "MOVE".blue(),
            "COPY" => "COPY".cyan(),
            "CREATE" => "CREATE".green(),
            other => other.normal(),
        };

        println!(
            "{} | {:8} | {} | {} {}",
            time,
            op_type,
            op.path.display(),
            op.user.dimmed(),
            status
        );
    }

    println!("{}", "─".repeat(70));
    println!("Total: {} operations", jk.metadata_store.count());

    Ok(())
}

fn cmd_status(dir: &PathBuf) -> Result<()> {
    let jk = JanusKey::open(dir).context("Failed to open JanusKey directory")?;

    println!("{}", "JanusKey Status".bold());
    println!("{}", "─".repeat(40));
    println!("Directory: {}", dir.display());
    println!("Operations logged: {}", jk.metadata_store.count());
    println!(
        "Content store: {} blobs ({} bytes)",
        jk.content_store.count()?,
        human_bytes(jk.content_store.total_size()?)
    );

    if let Some(tx) = jk.transaction_manager.active() {
        let name = tx.name.clone().unwrap_or_else(|| tx.id[..8].to_string());
        println!();
        println!(
            "{} Active transaction: {}",
            "📝".to_string(),
            name.cyan()
        );
        println!("  Started: {}", tx.started_at.format("%Y-%m-%d %H:%M:%S"));
        println!("  Operations: {}", tx.operation_ids.len());
    } else {
        println!();
        println!("No active transaction");
    }

    Ok(())
}

fn cmd_gc(dir: &PathBuf, keep: Option<usize>, _older_than: Option<u32>) -> Result<()> {
    let mut jk = JanusKey::open(dir).context("Failed to open JanusKey directory")?;

    let keep_count = keep.unwrap_or(jk.config.max_history);

    let pruned = jk.metadata_store.prune(keep_count)?;

    if pruned > 0 {
        println!(
            "{} Pruned {} old operations (keeping last {})",
            "✓".green(),
            pruned,
            keep_count
        );
    } else {
        println!("{} Nothing to prune", "✓".green());
    }

    Ok(())
}

fn human_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}
