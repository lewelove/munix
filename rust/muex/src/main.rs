use clap::{Parser, Subcommand};

mod discid;

#[derive(Parser)]
#[command(name = "muex", version = "0.1.0")]
struct Cli {
    #[arg(global = true, long)]
    debug: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Discid {
        #[arg(long)]
        source: Option<String>,
        #[arg(long)]
        tracks: Option<String>,
    },
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
        Commands::Discid { source, tracks } => {
            discid::run(source.as_deref(), tracks.as_deref());
        }
    }
}
