use clap::{Parser, Subcommand};
use commons::get_settings;
use std::{
    env,
    path::Path,
};

mod commons;
mod server;

#[derive(Parser)]
#[command(
    author = "didicodethat",
    about = "MMO Server Engine",
    long_about = "A Lua MMO server engine framework taking advantage of modern rs features."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Starts the Server. alias: s
    #[clap(alias = "s")]
    Server {
        /// Project folder to work on, if left empty will work on current directory.
        project_folder: Option<String>,
    },
    /// Creates the Database
    CreateDb {
        /// Project folder to work on, if left empty will work on current directory.
        project_folder: Option<String>,
    },
    /// Execute Migrations
    DbMigrate {
        /// Project folder to work on, if left empty will work on current directory.
        project_folder: Option<String>,
    },
    /// Creates migration files
    CreateDbMigration {
        /// Project folder to work on, if left empty will work on current directory.
        project_folder: Option<String>,
        migration_name: String,
    },
    /// Standalone Script Generation From Messages
    GenerateScripts {
        /// Project folder to work on, if left empty will work on current directory.
        project_folder: Option<String>,
    },
}

fn extract_project_folder(command: &Commands) -> &Option<String> {
    match command {
        Commands::Server { project_folder } => project_folder,
        Commands::CreateDb { project_folder } => project_folder,
        Commands::DbMigrate { project_folder } => project_folder,
        Commands::CreateDbMigration {
            project_folder,
            migration_name: _,
        } => project_folder,
        Commands::GenerateScripts { project_folder } => project_folder,
    }
}

fn main() {
    let cli = Cli::parse();
    let command = cli.command;
    let current_folder = ".".to_string();
    let project_folder_string = extract_project_folder(&command)
        .as_ref()
        .unwrap_or(&current_folder);
    let project_folder = Path::new(project_folder_string);
    env::set_current_dir(project_folder).expect("Couldn't work with the set directory.");
    let settings = get_settings();
    match command {
        Commands::Server { project_folder: _ } => crate::server::start_server(&settings),
        Commands::CreateDb { project_folder: _ } => todo!(),
        Commands::DbMigrate { project_folder: _ } => todo!(),
        Commands::CreateDbMigration {
            project_folder: _,
            migration_name: _,
        } => todo!(),
        Commands::GenerateScripts { project_folder: _ } => todo!(),
    }
}
