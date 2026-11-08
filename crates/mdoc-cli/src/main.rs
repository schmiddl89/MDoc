use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "mdoc", version, about = "Markdown and Word interoperability")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Convert a Markdown file to DOCX.
    Convert {
        input: PathBuf,
        output: PathBuf,
        /// Use styles from a .dotx or .docx Word template.
        #[arg(long)]
        template: Option<PathBuf>,
        /// Map Markdown semantics to Word style IDs with a JSON profile.
        #[arg(long)]
        style_profile: Option<PathBuf>,
        /// Replace the output file if it already exists.
        #[arg(long)]
        force: bool,
    },
    /// Print the normalized document model as JSON.
    Ast {
        input: PathBuf,
        #[arg(long)]
        pretty: bool,
    },
    /// List the style IDs and display names in a Word template.
    Styles {
        template: PathBuf,
        #[arg(long)]
        pretty: bool,
    },
    /// Convert a DOCX file to Markdown.
    Import {
        input: PathBuf,
        output: PathBuf,
        /// Interpret custom Word style IDs with a JSON profile.
        #[arg(long)]
        style_profile: Option<PathBuf>,
        /// Replace the output file if it already exists.
        #[arg(long)]
        force: bool,
    },
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Convert {
            input,
            output,
            template,
            style_profile,
            force,
        } => {
            if output.exists() && !force {
                bail!(
                    "output file already exists: {} (pass --force to replace it)",
                    output.display()
                );
            }
            let markdown = read_markdown(&input)?;
            let profile = style_profile
                .as_ref()
                .map(read_style_profile)
                .transpose()?
                .unwrap_or_default();
            let options = mdoc_core::ExportOptions {
                style_profile: profile,
                template,
            };
            let document = mdoc_core::markdown_to_docx_with_options(&markdown, &output, &options)
                .with_context(|| format!("could not convert {}", input.display()))?;
            println!(
                "Wrote {} blocks to {}",
                document.blocks.len(),
                output.display()
            );
        }
        Command::Ast { input, pretty } => {
            let markdown = read_markdown(&input)?;
            let document = mdoc_core::markdown::parse(&markdown)
                .with_context(|| format!("could not parse {}", input.display()))?;
            let json = if pretty {
                serde_json::to_string_pretty(&document)?
            } else {
                serde_json::to_string(&document)?
            };
            println!("{json}");
        }
        Command::Styles { template, pretty } => {
            let styles = mdoc_core::template::inspect(&template)
                .with_context(|| format!("could not inspect {}", template.display()))?;
            let json = if pretty {
                serde_json::to_string_pretty(&styles)?
            } else {
                serde_json::to_string(&styles)?
            };
            println!("{json}");
        }
        Command::Import {
            input,
            output,
            style_profile,
            force,
        } => {
            if output.exists() && !force {
                bail!(
                    "output file already exists: {} (pass --force to replace it)",
                    output.display()
                );
            }
            let profile = style_profile
                .as_ref()
                .map(read_style_profile)
                .transpose()?
                .unwrap_or_default();
            let markdown = mdoc_core::docx_to_markdown(&input, &profile)
                .with_context(|| format!("could not import {}", input.display()))?;
            fs::write(&output, markdown)
                .with_context(|| format!("could not write {}", output.display()))?;
            println!("Imported {} to {}", input.display(), output.display());
        }
    }
    Ok(())
}

fn read_markdown(path: &PathBuf) -> Result<String> {
    fs::read_to_string(path)
        .with_context(|| format!("could not read Markdown file {}", path.display()))
}

fn read_style_profile(path: &PathBuf) -> Result<mdoc_core::StyleProfile> {
    let json = fs::read_to_string(path)
        .with_context(|| format!("could not read style profile {}", path.display()))?;
    let profile: mdoc_core::StyleProfile = serde_json::from_str(&json)
        .with_context(|| format!("invalid style profile {}", path.display()))?;
    Ok(mdoc_core::migrate_legacy_english_ids(profile))
}
