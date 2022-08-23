use clap::{Parser, Subcommand};
use model::{Heuristic, Model, SimpleTiled};
use std::{
    fs,
    path::{Path, PathBuf},
    process::exit,
};
use tile_collapse::{model, Config};

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
/// Implementation of the tilemap version of wavefuntion collapse
struct Args {
    #[clap(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Runs this program headless
    Cli {
        /// The folder including the tile images and a config.toml
        #[clap(value_parser = is_dir)]
        input_folder: String,

        /// The width of the output image in tiles
        #[clap()]
        width: usize,
        /// The height of the output image in tiles
        #[clap()]
        height: usize,

        /// The heuristic used to generate the next tile
        #[clap(short = 'H', long, default_value = "scan-line", arg_enum)]
        heuristic: Heuristic,

        /// Whether the output image should be tileable
        #[clap(short, long)]
        periodic: bool,
    },
    /// Runs this program in a gui [default subcommand]
    Gui,
}

fn is_dir(s: &str) -> Result<String, String> {
    let mut path = Path::new(s).to_owned();
    if path.is_dir() {
        path.push("config.toml");
        if !path.exists() {
            Err(format!("{} doesn't exist", path.to_string_lossy()))
        } else if fs::read_dir(Path::new(s))
            .map_err(|err| format!("{}", err))?
            .count()
            < 2
        {
            Err("Missing tile pictures".to_string())
        } else {
            Ok(s.to_string())
        }
    } else {
        Err(format!("{} isn't a directory", s))
    }
}

fn main() {
    let args = Args::parse();

    match args.command.unwrap_or(Commands::Gui) {
        Commands::Cli {
            input_folder,
            width,
            height,
            heuristic,
            periodic,
        } => {
            let dir = Path::new(&input_folder);
            let mut config = PathBuf::from(&input_folder);
            config.push("config.toml");

            let content = std::fs::read_to_string(config).unwrap();
            let config: Config = toml::from_str(&content).unwrap_or_else(|err| {
                println!("config.toml does not have the correct format: {err}");
                exit(1)
            });

            //println!("Hello, world! config={:?}", config);

            if let Ok(mut tiled_model) = SimpleTiled::new(
                config,
                dir.to_str().unwrap(),
                width,
                height,
                periodic,
                heuristic,
            )
            .map_err(|err| println!("{err}"))
            {
                //println!("{tiled_model}");
                while !tiled_model.run(rand::random(), usize::MAX) {}
                //println!("{tiled_model}");
                let res = tiled_model.save(Path::new("a.png"));
                println!("{:?}", res);
            }
        }
        Commands::Gui => todo!("Gui"),
    }
}

#[test]
fn varify_cli() {
    use clap::CommandFactory;
    Args::command().debug_assert();
}
