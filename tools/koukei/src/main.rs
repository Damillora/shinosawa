
use std::{env, path::Path};

use clap::{Parser, Subcommand};

mod build_image;

/// Builder helpers for shinosawa
#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Change to directory first
    #[clap(short('C'))]
    working_dir: Option<String>,

    /// Root directory of project
    #[clap(long, short)]
    name: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Builds a shinosawa disk image
    BuildImage {
        /// Profile
        #[clap(long, short, default_value_t = String::from("debug"))]
        profile: String,
        /// Kernel image for use
        #[clap(long, short)]
        kernel_image: Option<String>,
    },
}

fn main() {
    let cli = Cli::parse();

    if let Some(working_dir) = cli.working_dir {
        let root = Path::new(&working_dir);
        env::set_current_dir(&root).expect("Cannot change directory!");
    }

    match &cli.command {
        Commands::BuildImage { profile, kernel_image} => {
            build_image::command(profile.to_owned(), kernel_image.to_owned());
        }
    }
}
