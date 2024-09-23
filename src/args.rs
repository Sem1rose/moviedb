use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    #[arg(short, long, value_name = "FILE")]
    pub config: Option<PathBuf>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Add movie to the db.
    Add,
    /// Remove a movie from the db.
    Remove {
        /// Name of the movie to remove.
        name: String,
    },
    /// Get a specific movie from the db.
    Get {
        /// Name of the movie to get.
        name: String,
    },
}
