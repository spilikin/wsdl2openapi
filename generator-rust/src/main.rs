use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;

use wsdl2openapi2rust::model::Api;
use wsdl2openapi2rust::naming::NamingStrategy;
use wsdl2openapi2rust::{Options, generate, write_files};

/// Generate Rust types and SOAP envelopes from a wsdl2openapi OpenAPI document.
#[derive(Debug, Parser)]
#[command(version)]
struct Cli {
    /// OpenAPI JSON produced by the wsdl2openapi converter.
    #[arg(short, long)]
    file: PathBuf,

    /// Directory to write the module tree to (its root file is `mod.rs`).
    #[arg(short, long)]
    output: PathBuf,

    /// Naming configuration JSON (package mappings, port mappings).
    #[arg(short, long)]
    naming: Option<PathBuf>,

    /// Module path the output is mounted at, used for cross-module references.
    #[arg(long, default_value = "crate")]
    module_root: String,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let api_json =
        fs::read_to_string(&cli.file).with_context(|| format!("reading {}", cli.file.display()))?;
    let api: Api = serde_json::from_str(&api_json)
        .with_context(|| format!("parsing {}", cli.file.display()))?;
    let naming = match &cli.naming {
        Some(path) => {
            let json =
                fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
            NamingStrategy::from_json(&json)?
        }
        None => NamingStrategy::default(),
    };

    let files = generate(
        api,
        &naming,
        &Options {
            module_root: cli.module_root,
        },
    )?;
    write_files(&cli.output, &files)?;
    eprintln!("wrote {} files to {}", files.len(), cli.output.display());
    Ok(())
}
