use anyhow::bail;
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use syspm_core::{MetadataStatus, ModelFileScan, ProjectManifest, ProjectReport, Severity};

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
    /// Report a project's metadata, model files, and validation results
    Inspect {
        /// File or directory in the project; the nearest .project.json at or above it is used
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
    let root = syspm_core::find_project_root(path)?;
    let report = syspm_core::inspect_project(&root)?;
    print_report(&report);
    if let MetadataStatus::Failed(error) = report.metadata {
        return Err(error.into());
    }
    if report.has_errors() {
        bail!("project has validation errors");
    }

    Ok(())
}

fn print_report(report: &ProjectReport) {
    println!("Root: {}", report.root.display());
    print_manifest(&report.manifest);
    print_metadata(&report.metadata);
    print_model_files(&report.scan);
    print_validation(report);
}

fn print_manifest(manifest: &ProjectManifest) {
    println!("Project: {}", manifest.name);
    println!("Version: {}", manifest.version);
    if let Some(description) = &manifest.description {
        println!("Description: {description}");
    }
    if let Some(license) = &manifest.license {
        println!("License: {license}");
    }
    if !manifest.maintainer.is_empty() {
        println!("Maintainers: {}", manifest.maintainer.join(", "));
    }
    if let Some(website) = &manifest.website {
        println!("Website: {website}");
    }
    if !manifest.topic.is_empty() {
        println!("Topics: {}", manifest.topic.join(", "));
    }
    println!("Declared usages: {}", manifest.usage.len());
    for usage in &manifest.usage {
        match &usage.version_constraint {
            Some(constraint) => println!("  {} ({constraint})", usage.resource),
            None => println!("  {}", usage.resource),
        }
    }
}

fn print_metadata(metadata: &MetadataStatus) {
    match metadata {
        MetadataStatus::Parsed(parsed) => {
            println!("Metadata:");
            println!("  Created: {}", parsed.created);
            if let Some(metamodel) = &parsed.metamodel {
                println!("  Metamodel: {metamodel}");
            }
            if let Some(includes_derived) = &parsed.includes_derived {
                println!("  Includes derived: {includes_derived}");
            }
            if let Some(includes_implied) = &parsed.includes_implied {
                println!("  Includes implied: {includes_implied}");
            }
            println!("  Index entries: {}", parsed.index.len());
        }
        MetadataStatus::Missing => println!("Metadata: none"),
        MetadataStatus::Failed(_) => println!("Metadata: unreadable"),
    }
}

fn print_model_files(scan: &ModelFileScan) {
    let suffix = if scan.is_complete() { "" } else { " (partial)" };
    println!("Model files: {}{suffix}", scan.files.len());
    for file in &scan.files {
        println!("  {file}");
    }
}

/// Validation results exclude a `.meta.json` that could not be read, so they are partial then.
fn print_validation(report: &ProjectReport) {
    let count = |severity| {
        report
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == severity)
            .count()
    };
    let suffix = if matches!(report.metadata, MetadataStatus::Failed(_)) {
        " (partial)"
    } else {
        ""
    };
    println!(
        "Validation: {}, {}{suffix}",
        count_label(count(Severity::Error), "error"),
        count_label(count(Severity::Warning), "warning")
    );
    for diagnostic in &report.diagnostics {
        println!(
            "  {}[{}] {}: {}",
            diagnostic.severity, diagnostic.rule, diagnostic.path, diagnostic.message
        );
    }
}

fn count_label(count: usize, noun: &str) -> String {
    if count == 1 {
        format!("{count} {noun}")
    } else {
        format!("{count} {noun}s")
    }
}
