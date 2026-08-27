use clap::Parser;
use editag::cli::Cli;
use editag::cli::Commands;
use editag::parser::parse_config;
use editag::writer::write_metadata;
use editag::generator::generate_template;
use std::path::Path;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Write { config, dry_run } => {
            let config_path = Path::new(&config);
            let (global, tracks) = parse_config(config_path)?;

            if dry_run {
                println!("DRY RUN - would write metadata to {} tracks", tracks.len());
                for track in &tracks {
                    println!("  {:?}", track);
                }
            } else {
                write_metadata(&global, &tracks)?;
                println!("Metadata applied successfully.");
            }
        }
        Commands::Generate { output } => {
            generate_template(output)?;
        }
    }

    Ok(())
}
