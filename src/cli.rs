use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "editag")]
#[command(about = "Edit audio metadata from config files")]
#[command(version = "0.1.0")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Write metadata from config file
    Write {
        /// Path to config file
        #[arg(short, long, default_value = "tags.ini")]
        config: String,

        /// Preview changes without writing
        #[arg(short, long)]
        dry_run: bool,
    },

    /// Generate template config from current directory
    #[command(name = "gen", visible_alias = "generate")]
    Generate {
        /// Output config file path (default: tags.ini)
        #[arg(short, long, default_value = "tags.ini")]
        output: Option<String>,
    },
}
