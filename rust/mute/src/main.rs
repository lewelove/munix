use clap::{Parser, Subcommand};

mod album;
mod film;

#[derive(Parser)]
#[command(name = "mute", version = "0.1.0")]
struct Cli {
    #[arg(global = true, long)]
    debug: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Album {
        #[command(subcommand)]
        command: AlbumCommands,
    },
    Film {
        #[command(subcommand)]
        command: FilmCommands,
    },
}

#[derive(Subcommand)]
enum AlbumCommands {
    Build {
        #[arg(default_value = ".")]
        path: String,
        #[arg(long)]
        flake: Option<String>,
    },
    Fetch {
        #[arg(default_value = ".")]
        path: String,
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
    Library {
        #[command(subcommand)]
        command: AlbumLibraryCommands,
    },
}

#[derive(Subcommand)]
enum AlbumLibraryCommands {
    Rebuild,
    Init {
        path: String,
    },
    MigrateStore,
}

#[derive(Subcommand)]
enum FilmCommands {
    Build {
        #[arg(default_value = ".")]
        path: String,
    },
    Fetch {
        #[arg(default_value = ".")]
        path: String,
    },
    Manifest {
        #[arg(long)]
        torrent: Option<String>,
        #[arg(default_value = ".")]
        path: String,
        #[arg(long)]
        video: Option<String>,
        #[arg(long)]
        intermediary: bool,
    },
    Library {
        #[command(subcommand)]
        command: FilmLibraryCommands,
    },
}

#[derive(Subcommand)]
enum FilmLibraryCommands {
    Rebuild,
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
        Commands::Album { command } => match command {
            AlbumCommands::Build { path, flake } => {
                if let Err(e) = album::build::run(&path, flake.as_deref()) {
                    log::error!("{e}");
                    std::process::exit(1);
                }
            }
            AlbumCommands::Fetch { path } => {
                if let Err(e) = album::fetch::run(&path) {
                    log::error!("{e}");
                    std::process::exit(1);
                }
            }
            AlbumCommands::Manifest { torrent, path, tracks, metadata, intermediary } => {
                if let Err(e) = album::manifest::run(&path, &tracks, torrent.as_deref(), metadata.as_deref(), intermediary) {
                    log::error!("{e}");
                    std::process::exit(1);
                }
            }
            AlbumCommands::Library { command } => match command {
                AlbumLibraryCommands::Rebuild => {
                    if let Err(e) = album::library::rebuild::run() {
                        log::error!("{e}");
                        std::process::exit(1);
                    }
                }
                AlbumLibraryCommands::Init { path } => {
                    if let Err(e) = album::library::init::run(&path) {
                        log::error!("{e}");
                        std::process::exit(1);
                    }
                }
                AlbumLibraryCommands::MigrateStore => {
                    if let Err(e) = album::library::migrate_store::run() {
                        log::error!("{e}");
                        std::process::exit(1);
                    }
                }
            }
        },
        Commands::Film { command } => match command {
            FilmCommands::Build { path } => {
                if let Err(e) = film::build::run(&path) {
                    log::error!("{e}");
                    std::process::exit(1);
                }
            }
            FilmCommands::Fetch { path } => {
                if let Err(e) = film::fetch::run(&path) {
                    log::error!("{e}");
                    std::process::exit(1);
                }
            }
            FilmCommands::Manifest { torrent, path, video, intermediary } => {
                if let Err(e) = film::manifest::run(&path, video.as_deref(), torrent.as_deref(), intermediary) {
                    log::error!("{e}");
                    std::process::exit(1);
                }
            }
            FilmCommands::Library { command } => match command {
                FilmLibraryCommands::Rebuild => {
                    if let Err(e) = film::library::rebuild::run() {
                        log::error!("{e}");
                        std::process::exit(1);
                    }
                }
            }
        }
    }
}
