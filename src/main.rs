/// cogito — operational TBox CLI for the wintermute box
///
/// Subcommands:
///   cogito tbox build [--out cogito.owl]
///   cogito tbox check
///   cogito tbox stats <file>

use std::path::PathBuf;
use std::process;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

mod spec;
mod tbox;

#[derive(Parser)]
#[command(
    name = "cogito",
    version,
    about = "Operational TBox for the wintermute box"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Manage the operational TBox
    Tbox {
        #[command(subcommand)]
        action: TboxAction,
    },
}

#[derive(Subcommand)]
enum TboxAction {
    /// Build the operational TBox to OWL 2 DL XML
    Build {
        /// Output file path
        #[arg(long, default_value = "cogito.owl")]
        out: PathBuf,
        /// Spec directory (default: built-in spec)
        #[arg(long)]
        spec: Option<PathBuf>,
    },
    /// Validate the spec without emitting output
    Check {
        /// Spec directory (default: built-in spec)
        #[arg(long)]
        spec: Option<PathBuf>,
    },
    /// Print class/property/axiom counts for a built ontology
    Stats {
        /// OWL file to inspect
        file: PathBuf,
    },
}

fn main() {
    if let Err(e) = run() {
        eprintln!("cogito: error: {:#}", e);
        process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Tbox { action } => match action {
            TboxAction::Build { out, spec } => {
                tbox::build(spec.as_deref(), &out)?;
            }
            TboxAction::Check { spec } => {
                tbox::check(spec.as_deref())?;
            }
            TboxAction::Stats { file } => {
                tbox::stats(&file)?;
            }
        },
    }
    Ok(())
}
