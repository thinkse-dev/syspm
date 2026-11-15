use anyhow::bail;
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Package manager for SysML v2 and KerML projects (early development).
#[derive(Debug, Parser)]
#[command(name = "syspm", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Initialize a SysML v2 project (not yet implemented)
    Init,
    /// Inspect project metadata and dependencies from .project.json
    /// (does not validate against the OMG schema yet)
    Inspect {
        /// Directory containing .project.json, or path to the manifest itself
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Install project dependencies (not yet implemented)
    Install,
    /// Update project dependencies (not yet implemented)
    Update,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command {
        Commands::Init => not_implemented("init"),
        Commands::Inspect { path } => inspect(&path),
        Commands::Install => not_implemented("install"),
        Commands::Update => not_implemented("update"),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            // `{:#}` prints the full cause chain on one line: "outer: inner: ...".
            eprintln!("syspm: {error:#}");
            ExitCode::FAILURE
        }
    }
}

fn not_implemented(command: &str) -> anyhow::Result<()> {
    bail!("`{command}` is not implemented yet")
}

fn inspect(path: &Path) -> anyhow::Result<()> {
    let manifest_path = if path.is_dir() {
        path.join(".project.json")
    } else {
        path.to_path_buf()
    };
    let manifest = syspm_core::read_project(&manifest_path)?;
    println!("Project: {}", manifest.name);
    println!("Version: {}", manifest.version);
    println!("Declared usages: {}", manifest.usage.len());
    for usage in manifest.usage {
        match usage.version_constraint {
            Some(constraint) => println!("  {} ({constraint})", usage.resource),
            None => println!("  {}", usage.resource),
        }
    }
    Ok(())
}
