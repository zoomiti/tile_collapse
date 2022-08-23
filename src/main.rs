use clap::{ArgAction, Parser};
use std::{
    fmt::Debug,
    path::{Path, PathBuf},
};
use tile_collapse::{
    model::{Heuristic, Model, SimpleTiled},
    Config,
};

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

    #[clap()]
    width: usize,
    #[clap()]
    height: usize,

    #[clap(short, long, default_value = "ScanLine")]
    heuristic: Heuristic,
}

fn is_dir(s: &str) -> Result<String, String> {
    if Path::new(s).is_dir() {
        Ok(s.to_string())
    } else {
        Err(format!("{} isn't a directory", s))
    }
}

fn main() {
    let args = Args::parse();

    let dir = Path::new(&args.input_folder);
    let mut config = PathBuf::from(&args.input_folder);
    config.push("config.toml");

    let content = std::fs::read_to_string(config).unwrap();
    let config: Config = toml::from_str(&content).unwrap();

    //println!("Hello, world! config={:?}", config);

    if let Ok(mut tiled_model) = SimpleTiled::new(
        config,
        dir.to_str().unwrap(),
        args.width,
        args.height,
        true,
        Heuristic::MRV,
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

#[test]
fn varify_cli() {
    use clap::CommandFactory;
    Args::command().debug_assert();
}
