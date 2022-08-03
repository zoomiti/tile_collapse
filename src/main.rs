use clap::{ArgAction, Parser};
use std::path::Path;
use std::{fmt::Debug, path::PathBuf};
use tile_collapse::{Config, SimpleTiledModel};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
/// Implementation of the tilemap version of wavefuntion collapse
struct Args {
    /// Runs this program headless
    #[clap(short, long, action = ArgAction::SetTrue, default_value_t)]
    //default value is true because gui is not implemented yet
    cli: bool,

    #[clap(value_parser = is_dir)]
    input_folder: String,
}

fn is_dir(s: &str) -> Result<String, String> {
    match Path::new(s).is_dir() {
        true => Ok(s.to_string()),
        false => Err(format!("{} isn't a directory", s)),
    }
}

fn main() {
    let args = Args::parse();

    let dir = Path::new(&args.input_folder);
    let mut config = PathBuf::new();
    config.set_file_name(&args.input_folder);
    config.push("config.toml");

    let content = std::fs::read_to_string(config).unwrap();
    let config: Config = toml::from_str(&content).unwrap();

    println!("Hello, world! config={:?}", config);

    let tiled_model = SimpleTiledModel::new(config, dir.to_str().unwrap());
    println!("{:?}", tiled_model);
}

#[test]
fn varify_cli() {
    use clap::CommandFactory;
    Args::command().debug_assert();
}
