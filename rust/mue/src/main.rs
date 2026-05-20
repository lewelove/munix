use clap::{Parser, Subcommand};

mod build;
mod fetch;
mod manifest;
mod library;

#[derive(Parser)]
#[command(name = "mue", version = "0.1.0")]
struct Cli {
    #[arg(global = true, long)]
    debug: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Album {
        #[arg(default_value = ".")]
        path: String,
        #[arg(long)]
        source: Option<String>,
        #[arg(long)]
        flake: Option<String>,
    },
    Library {
        #[command(subcommand)]
        command: LibraryCommands,
    },
    Manifest {
        #[arg(long)]
        torrent: Option<String>,
        #[arg(default_value = ".")]
        path: String,
        #[arg(long, default_value = "flac,mp3,wav")]
        tracks: String,
        #[arg(long)]
        metadata: Option<String>,
        #[arg(long)]
        intermediary: bool,
    },
    Fetch {
        #[arg(long)]
        source: String,
        #[arg(default_value = ".")]
        path: String,
    },
}

#[derive(Subcommand)]
enum LibraryCommands {
    Rebuild,
    Init {
        path: String,
    },
    MigrateStore,
}

fn main() {
    let cli = Cli::parse();

    let log_level = if cli.debug {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    };

    simple_logger::SimpleLogger::new()
        .with_level(log_level)
        .env()
        .init()
        .unwrap();

    match cli.command {
        Commands::Album { path, source, flake } => {
            if let Err(e) = build::run(&path, source.as_deref(), flake.as_deref()) {
                log::error!("{e}");
                std::process::exit(1);
            }
        }
        Commands::Library { command } => {
            match command {
                LibraryCommands::Rebuild => {
                    if let Err(e) = library::rebuild() {
                        log::error!("{e}");
                        std::process::exit(1);
                    }
                }
                LibraryCommands::Init { path } => {
                    if let Err(e) = library::init(&path) {
                        log::error!("{e}");
                        std::process::exit(1);
                    }
                }
                LibraryCommands::MigrateStore => {
                    if let Err(e) = library::migrate_store() {
                        log::error!("{e}");
                        std::process::exit(1);
                    }
                }
            }
        }
        Commands::Manifest { torrent, path, tracks, metadata, intermediary } => {
            if let Err(e) = manifest::run(&path, &tracks, torrent.as_deref(), metadata.as_deref(), intermediary) {
                log::error!("{e}");
                std::process::exit(1);
            }
        }
        Commands::Fetch { source, path } => {
            if let Err(e) = fetch::run(&source, &path) {
                log::error!("{e}");
                std::process::exit(1);
            }
        }
    }
}
