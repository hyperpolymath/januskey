// SPDX-License-Identifier: MIT OR AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2025 Jonathan D.A. Jewell
//
// JanusKey Key Management CLI
// Minimal CLI for cryptographic key lifecycle management

use clap::{Parser, Subcommand};
use colored::Colorize;
use dialoguer::{Confirm, Password};
use std::path::PathBuf;
use uuid::Uuid;

mod attestation;
mod keys;
use attestation::AuditEventType;
use keys::{KeyAlgorithm, KeyManager, KeyPurpose, KeyState};

#[derive(Parser)]
#[command(name = "jk-keys")]
#[command(about = "JanusKey cryptographic key management")]
#[command(version)]
struct Cli {
    /// Working directory (defaults to current)
    #[arg(short, long, global = true)]
    dir: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new key store
    Init {
        /// Skip recovery key generation
        #[arg(long)]
        no_recovery: bool,
    },

    /// List all keys in the store
    List {
        /// Show only active keys
        #[arg(long)]
        active: bool,
    },

    /// Generate a new key
    Generate {
        /// Key type: aes256, ed25519, x25519
        #[arg(short, long, default_value = "aes256")]
        r#type: String,

        /// Key purpose: encryption, signing, keywrap, recovery
        #[arg(short, long, default_value = "encryption")]
        purpose: String,

        /// Description for the key
        #[arg(short, long)]
        description: Option<String>,

        /// Expiration in days
        #[arg(short, long)]
        expires: Option<u64>,
    },

    /// Show details for a specific key
    Show {
        /// Key ID (UUID)
        key_id: Uuid,
    },

    /// Rotate a key (generate new, revoke old)
    Rotate {
        /// Key ID to rotate
        key_id: Uuid,
    },

    /// Revoke a key
    Revoke {
        /// Key ID to revoke
        key_id: Uuid,

        /// Skip confirmation
        #[arg(short, long)]
        force: bool,
    },

    /// Create encrypted backup of key store
    Backup {
        /// Output path for backup file
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Show key store status
    Status,

    /// View audit log
    Audit {
        #[command(subcommand)]
        command: AuditCommands,
    },
}

#[derive(Subcommand)]
enum AuditCommands {
    /// Show recent audit entries
    Show {
        /// Number of entries to show
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },

    /// Show audit history for a specific key
    History {
        /// Key ID (UUID)
        key_id: Uuid,
    },

    /// Verify audit log integrity
    Verify,

    /// Export audit log to JSON
    Export {
        /// Output path for JSON file
        #[arg(short, long)]
        output: PathBuf,
    },
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{} {}", "error:".red().bold(), e);
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let dir = cli
        .dir
        .unwrap_or_else(|| std::env::current_dir().expect("Cannot get current directory"));

    let mut km = KeyManager::new(&dir);

    match cli.command {
        Commands::Init { no_recovery } => cmd_init(&mut km, no_recovery)?,
        Commands::List { active } => cmd_list(&mut km, active)?,
        Commands::Generate {
            r#type,
            purpose,
            description,
            expires,
        } => cmd_generate(&mut km, &r#type, &purpose, description, expires)?,
        Commands::Show { key_id } => cmd_show(&mut km, key_id)?,
        Commands::Rotate { key_id } => cmd_rotate(&mut km, key_id)?,
        Commands::Revoke { force, key_id } => cmd_revoke(&mut km, key_id, force)?,
        Commands::Backup { output } => cmd_backup(&mut km, &output)?,
        Commands::Status => cmd_status(&km)?,
        Commands::Audit { command } => match command {
            AuditCommands::Show { limit } => cmd_audit_show(&mut km, limit)?,
            AuditCommands::History { key_id } => cmd_audit_history(&mut km, key_id)?,
            AuditCommands::Verify => cmd_audit_verify(&mut km)?,
            AuditCommands::Export { output } => cmd_audit_export(&mut km, &output)?,
        },
    }

    Ok(())
}

fn cmd_init(km: &mut KeyManager, _no_recovery: bool) -> Result<(), Box<dyn std::error::Error>> {
    if km.is_initialized() {
        return Err("Key store already initialized".into());
    }

    println!("{}", "Initializing JanusKey key store...".cyan());
    println!();
    println!(
        "{}",
        "IMPORTANT: Choose a strong passphrase to protect your keys.".yellow()
    );
    println!(
        "{}",
        "This passphrase is required to unlock the key store.".yellow()
    );
    println!();

    let passphrase = Password::new()
        .with_prompt("Enter passphrase")
        .with_confirmation("Confirm passphrase", "Passphrases do not match")
        .interact()?;

    if passphrase.len() < 8 {
        return Err("Passphrase must be at least 8 characters".into());
    }

    km.init(&passphrase)?;

    println!();
    println!("{}", "✓ Key store initialized successfully".green());
    println!();
    println!("Location: {}/.januskey/keys/", std::env::current_dir()?.display());
    println!();
    println!("{}", "Next steps:".cyan());
    println!("  • Generate a key:  jk-keys generate --type aes256 --purpose encryption");
    println!("  • List keys:       jk-keys list");
    println!("  • Create backup:   jk-keys backup --output ~/keys-backup.jks");

    Ok(())
}

fn cmd_list(km: &mut KeyManager, active_only: bool) -> Result<(), Box<dyn std::error::Error>> {
    unlock_store(km)?;

    let keys = km.list()?;

    if keys.is_empty() {
        println!("{}", "No keys in store. Generate one with: jk-keys generate".yellow());
        return Ok(());
    }

    let filtered: Vec<_> = if active_only {
        keys.into_iter().filter(|k| k.state == KeyState::Active).collect()
    } else {
        keys
    };

    println!("{}", "Keys in store:".cyan().bold());
    println!();
    println!(
        "{:<38} {:<12} {:<12} {:<10} {}",
        "ID".bold(),
        "Algorithm".bold(),
        "Purpose".bold(),
        "State".bold(),
        "Fingerprint".bold()
    );
    println!("{}", "-".repeat(90));

    for key in filtered {
        let state_str = match key.state {
            KeyState::Active => key.state.to_string().green(),
            KeyState::Revoked => key.state.to_string().red(),
            KeyState::Rotating => key.state.to_string().yellow(),
            _ => key.state.to_string().normal(),
        };

        println!(
            "{:<38} {:<12} {:<12} {:<10} {}",
            key.id.to_string().dimmed(),
            key.algorithm.to_string(),
            key.purpose.to_string(),
            state_str,
            key.fingerprint.cyan()
        );
    }

    Ok(())
}

fn cmd_generate(
    km: &mut KeyManager,
    key_type: &str,
    purpose: &str,
    description: Option<String>,
    expires: Option<u64>,
) -> Result<(), Box<dyn std::error::Error>> {
    unlock_store(km)?;

    let algorithm = match key_type.to_lowercase().as_str() {
        "aes256" | "aes-256" | "aes256gcm" => KeyAlgorithm::Aes256Gcm,
        "ed25519" => KeyAlgorithm::Ed25519,
        "x25519" => KeyAlgorithm::X25519,
        _ => return Err(format!("Unknown key type: {}. Use: aes256, ed25519, x25519", key_type).into()),
    };

    let key_purpose = match purpose.to_lowercase().as_str() {
        "encryption" | "encrypt" => KeyPurpose::Encryption,
        "signing" | "sign" => KeyPurpose::Signing,
        "keywrap" | "key-wrap" | "wrap" => KeyPurpose::KeyWrap,
        "recovery" => KeyPurpose::Recovery,
        _ => return Err(format!("Unknown purpose: {}. Use: encryption, signing, keywrap, recovery", purpose).into()),
    };

    println!("{}", "Generating key...".cyan());

    let id = km.generate(algorithm, key_purpose, description.clone(), expires)?;
    let meta = km.get(id)?;

    println!();
    println!("{}", "✓ Key generated successfully".green());
    println!();
    println!("  ID:          {}", id.to_string().cyan());
    println!("  Algorithm:   {}", meta.algorithm);
    println!("  Purpose:     {}", meta.purpose);
    println!("  Fingerprint: {}", meta.fingerprint.cyan());
    if let Some(desc) = description {
        println!("  Description: {}", desc);
    }
    if let Some(exp) = meta.expires_at {
        println!("  Expires:     {}", exp.format("%Y-%m-%d"));
    }

    Ok(())
}

fn cmd_show(km: &mut KeyManager, key_id: Uuid) -> Result<(), Box<dyn std::error::Error>> {
    unlock_store(km)?;

    let meta = km.get(key_id)?;

    println!("{}", "Key Details:".cyan().bold());
    println!();
    println!("  ID:          {}", meta.id);
    println!("  Algorithm:   {}", meta.algorithm);
    println!("  Purpose:     {}", meta.purpose);
    println!("  State:       {}", format_state(meta.state));
    println!("  Fingerprint: {}", meta.fingerprint.cyan());
    println!("  Created:     {}", meta.created_at.format("%Y-%m-%d %H:%M:%S UTC"));

    if let Some(exp) = meta.expires_at {
        let now = chrono::Utc::now();
        let status = if exp < now {
            " (EXPIRED)".red()
        } else {
            "".normal()
        };
        println!("  Expires:     {}{}", exp.format("%Y-%m-%d %H:%M:%S UTC"), status);
    }

    if let Some(rot) = meta.rotation_of {
        println!("  Rotated from: {}", rot.to_string().dimmed());
    }

    if let Some(desc) = meta.description {
        println!("  Description: {}", desc);
    }

    Ok(())
}

fn cmd_rotate(km: &mut KeyManager, key_id: Uuid) -> Result<(), Box<dyn std::error::Error>> {
    unlock_store(km)?;

    let old_meta = km.get(key_id)?;

    if old_meta.state == KeyState::Revoked {
        return Err("Cannot rotate a revoked key".into());
    }

    println!("{}", "Rotating key...".cyan());
    println!("  Old key: {} ({})", key_id, old_meta.fingerprint);

    let new_id = km.rotate(key_id)?;
    let new_meta = km.get(new_id)?;

    println!();
    println!("{}", "✓ Key rotated successfully".green());
    println!();
    println!("  New key ID:    {}", new_id.to_string().cyan());
    println!("  Fingerprint:   {}", new_meta.fingerprint.cyan());
    println!("  Old key state: {}", "revoked".red());
    println!();
    println!(
        "{}",
        "Note: The old key can no longer be used for new operations.".yellow()
    );

    Ok(())
}

fn cmd_revoke(
    km: &mut KeyManager,
    key_id: Uuid,
    force: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    unlock_store(km)?;

    let meta = km.get(key_id)?;

    if meta.state == KeyState::Revoked {
        return Err("Key is already revoked".into());
    }

    if !force {
        println!("{}", "WARNING: Revoking a key is permanent!".red().bold());
        println!();
        println!("Key to revoke:");
        println!("  ID:          {}", key_id);
        println!("  Algorithm:   {}", meta.algorithm);
        println!("  Fingerprint: {}", meta.fingerprint);
        println!();

        let confirm = Confirm::new()
            .with_prompt("Are you sure you want to revoke this key?")
            .default(false)
            .interact()?;

        if !confirm {
            println!("{}", "Aborted.".yellow());
            return Ok(());
        }
    }

    km.revoke(key_id)?;

    println!();
    println!("{}", "✓ Key revoked".green());

    Ok(())
}

fn cmd_backup(km: &mut KeyManager, output: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    unlock_store(km)?;

    if output.exists() {
        let confirm = Confirm::new()
            .with_prompt(format!(
                "File {} already exists. Overwrite?",
                output.display()
            ))
            .default(false)
            .interact()?;

        if !confirm {
            println!("{}", "Aborted.".yellow());
            return Ok(());
        }
    }

    km.backup(output)?;

    println!("{}", "✓ Backup created successfully".green());
    println!();
    println!("  Location: {}", output.display());
    println!();
    println!(
        "{}",
        "Store this backup in a secure location separate from your main system.".yellow()
    );

    Ok(())
}

fn cmd_status(km: &KeyManager) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "Key Store Status:".cyan().bold());
    println!();

    if km.is_initialized() {
        println!("  Initialized: {}", "yes".green());

        // Try to get key count without unlocking (just check file exists)
        println!(
            "  Store path:  {}/.januskey/keys/keystore.jks",
            std::env::current_dir()?.display()
        );
    } else {
        println!("  Initialized: {}", "no".red());
        println!();
        println!("Run 'jk-keys init' to create a key store.");
    }

    Ok(())
}

fn unlock_store(km: &mut KeyManager) -> Result<(), Box<dyn std::error::Error>> {
    if !km.is_initialized() {
        return Err("Key store not initialized. Run 'jk-keys init' first.".into());
    }

    let passphrase = Password::new()
        .with_prompt("Enter passphrase")
        .interact()?;

    km.unlock(&passphrase)?;
    Ok(())
}

fn format_state(state: KeyState) -> colored::ColoredString {
    match state {
        KeyState::Active => "active".green(),
        KeyState::Revoked => "revoked".red(),
        KeyState::Rotating => "rotating".yellow(),
        KeyState::Suspended => "suspended".yellow(),
        KeyState::Generated => "generated".normal(),
        KeyState::Obliterated => "obliterated".red().bold(),
    }
}

fn format_event_type(event_type: AuditEventType) -> colored::ColoredString {
    match event_type {
        AuditEventType::StoreInitialized => "INIT".cyan(),
        AuditEventType::StoreUnlocked => "UNLOCK".normal(),
        AuditEventType::KeyGenerated => "GENERATE".green(),
        AuditEventType::KeyRetrieved => "RETRIEVE".yellow(),
        AuditEventType::KeyRotated => "ROTATE".blue(),
        AuditEventType::KeyRevoked => "REVOKE".red(),
        AuditEventType::KeyObliterated => "OBLITERATE".red().bold(),
        AuditEventType::BackupCreated => "BACKUP".cyan(),
        AuditEventType::BackupRestored => "RESTORE".cyan(),
    }
}

fn cmd_audit_show(km: &mut KeyManager, limit: usize) -> Result<(), Box<dyn std::error::Error>> {
    unlock_store(km)?;

    let entries = km.audit_log().read_last_n(limit)?;

    if entries.is_empty() {
        println!("{}", "No audit entries found.".yellow());
        return Ok(());
    }

    println!("{}", "Audit Log:".cyan().bold());
    println!();
    println!(
        "{:<20} {:<12} {:<20} {}",
        "Timestamp".bold(),
        "Event".bold(),
        "Actor".bold(),
        "Details".bold()
    );
    println!("{}", "-".repeat(80));

    for entry in entries {
        let timestamp = entry.timestamp.format("%Y-%m-%d %H:%M:%S");
        let event_str = format_event_type(entry.event_type);
        let actor = if entry.actor.len() > 18 {
            format!("{}...", &entry.actor[..15])
        } else {
            entry.actor.clone()
        };

        let details = if let Some(ref kd) = entry.key_details {
            format!("key:{}", &kd.fingerprint)
        } else if let Some(ref reason) = entry.reason {
            if reason.len() > 30 {
                format!("{}...", &reason[..27])
            } else {
                reason.clone()
            }
        } else {
            "-".to_string()
        };

        println!(
            "{:<20} {:<12} {:<20} {}",
            timestamp.to_string().dimmed(),
            event_str,
            actor,
            details.dimmed()
        );
    }

    Ok(())
}

fn cmd_audit_history(km: &mut KeyManager, key_id: Uuid) -> Result<(), Box<dyn std::error::Error>> {
    unlock_store(km)?;

    let entries = km.audit_log().get_key_history(key_id)?;

    if entries.is_empty() {
        println!("{}", format!("No audit entries found for key {}", key_id).yellow());
        return Ok(());
    }

    println!("{}", format!("Audit History for Key: {}", key_id).cyan().bold());
    println!();

    for entry in entries {
        let timestamp = entry.timestamp.format("%Y-%m-%d %H:%M:%S UTC");
        let event_str = format_event_type(entry.event_type);

        println!("{} {} by {}", timestamp.to_string().dimmed(), event_str, entry.actor);

        if let Some(ref kd) = entry.key_details {
            println!("  Fingerprint: {}", kd.fingerprint.cyan());
            if let Some(old) = kd.old_state {
                if let Some(new) = kd.new_state {
                    println!("  State: {} → {}", format_state(old), format_state(new));
                }
            }
            if let Some(from) = kd.rotated_from {
                println!("  Rotated from: {}", from.to_string().dimmed());
            }
        }

        if let Some(ref reason) = entry.reason {
            println!("  Reason: {}", reason);
        }

        println!();
    }

    Ok(())
}

fn cmd_audit_verify(km: &mut KeyManager) -> Result<(), Box<dyn std::error::Error>> {
    unlock_store(km)?;

    println!("{}", "Verifying audit log integrity...".cyan());

    let report = km.audit_log().verify_integrity()?;

    println!();
    if report.valid {
        println!("{}", "✓ Audit log integrity verified".green());
        println!();
        println!("  Total entries: {}", report.total_entries);
        println!("  Chain status:  {}", "intact".green());
        println!("  Attestations:  {}", "valid".green());
    } else {
        println!("{}", "✗ Audit log integrity check FAILED".red().bold());
        println!();
        println!("  {}", report.message.red());
        if let Some(idx) = report.first_invalid_index {
            println!("  First invalid entry: {}", idx);
        }
        println!();
        println!(
            "{}",
            "WARNING: The audit log may have been tampered with!".yellow().bold()
        );
    }

    Ok(())
}

fn cmd_audit_export(km: &mut KeyManager, output: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    unlock_store(km)?;

    if output.exists() {
        let confirm = Confirm::new()
            .with_prompt(format!("File {} already exists. Overwrite?", output.display()))
            .default(false)
            .interact()?;

        if !confirm {
            println!("{}", "Aborted.".yellow());
            return Ok(());
        }
    }

    km.audit_log().export_json(output)?;

    println!("{}", "✓ Audit log exported successfully".green());
    println!();
    println!("  Location: {}", output.display());

    Ok(())
}
