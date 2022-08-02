use clap::ArgAction;
use clap::Parser;
use serde::Deserialize;
use std::fmt::Debug;
use std::path::Path;

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

#[derive(Deserialize, Debug)]
struct Config {
    #[allow(dead_code)]
    tiles: Vec<Tile>,
}

#[derive(Deserialize, Debug)]
struct Tile {
    #[allow(dead_code)]
    name: String,
    #[allow(dead_code)]
    symmetry: String,
    #[allow(dead_code)]
    weight: Option<f64>,
}

fn main() {
    let args = Args::parse();

    let dir = Path::new(&args.input_folder);
    let mut config = dir.clone().to_path_buf();
    config.push("config.toml");

    let content = std::fs::read_to_string(config).unwrap();
    let config: Config = toml::from_str(&content).unwrap();

    println!("Hello, world! config={:?}", config);
}

#[test]
fn varify_cli() {
    use clap::CommandFactory;
    Args::command().debug_assert();
}
