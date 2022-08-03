use std::collections::HashMap;
use std::error::Error;

use image::DynamicImage;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Config {
    tiles: Vec<Tile>,
    neighbors: Vec<Neighbor>,
}

#[derive(Deserialize, Debug)]
struct Tile {
    name: String,
    symmetry: String,
    weight: Option<f64>,
}

#[derive(Clone, Debug)]
struct TileObject {
    #[allow(dead_code)]
    image: DynamicImage,
    #[allow(dead_code)]
    weight: f64,
}

#[derive(Deserialize, Debug)]
struct Neighbor {
    #[allow(dead_code)]
    left: String,
    #[allow(dead_code)]
    right: String,
}

#[derive(Debug)]
pub struct SimpleTiledModel {
    #[allow(dead_code)]
    tiles: Vec<TileObject>,

    #[allow(dead_code)]
    tile_names: Vec<String>,

    #[allow(dead_code)]
    neighbors: Vec<Neighbor>,

    #[allow(dead_code)]
    tile_size: usize,
}

impl TileObject {
    fn rotate_90(&mut self) {
        self.image = self.image.rotate90();
    }

    fn fliph(&mut self) {
        self.image = self.image.fliph()
    }
}

impl SimpleTiledModel {
    pub fn new(config: Config, folder: &str) -> Result<SimpleTiledModel, Box<dyn Error>> {
        if config.tiles.is_empty() {
            Err("No tiles in config file")?
        }

        let mut tiles = Vec::new();
        let mut tile_names = Vec::new();

        let mut action = Vec::new();
        let mut first_occurence = HashMap::new();

        for tile in config.tiles {
            let a: fn(i32) -> i32;
            let b: fn(i32) -> i32;
            let cardinality: i32;
            match tile.symmetry.as_bytes()[0] {
                b'L' => {
                    cardinality = 4;
                    a = |i| (i + 1) % 4;
                    b = |i| if i % 2 == 0 { i + 1 } else { i - 1 };
                }
                b'T' => {
                    cardinality = 4;
                    a = |i| (i + 1) % 4;
                    b = |i| if i % 2 == 0 { i } else { 4 - i };
                }
                b'I' => {
                    cardinality = 2;
                    a = |i| i - 1;
                    b = |i| i;
                }
                b'\\' => {
                    cardinality = 2;
                    a = |i| i - 1;
                    b = |i| i - 1;
                }
                b'F' => {
                    cardinality = 8;
                    a = |i| if i < 4 { (i + 1) % 4 } else { 4 + (i - 1) % 4 };
                    b = |i| if i < 4 { i + 4 } else { i - 4 };
                }
                _ => {
                    cardinality = 4;
                    a = |i| i;
                    b = |i| i;
                }
            }

            let t = action.len();
            first_occurence.insert(tile.name.clone(), t);

            let mut map = [[0; 8]; 8];
            for i in 0..cardinality {
                let index: usize = i.try_into().unwrap();
                let t: i32 = t.try_into().unwrap();
                map[index][0] = i + t;
                map[index][1] = a(i) + t;
                map[index][2] = a(a(i)) + t;
                map[index][3] = a(a(a(i))) + t;
                map[index][4] = b(i) + t;
                map[index][5] = b(a(i)) + t;
                map[index][6] = b(a(a(i))) + t;
                map[index][7] = b(a(a(a(i)))) + t;

                action.push(map[index]);
            }

            {
                let img = image::open(format!("{}{}", folder, tile.name));
                if let Ok(image) = img {
                    tiles.push(TileObject {
                        image: image.clone(),
                        weight: tile.weight.unwrap_or(1.0),
                    });
                    tile_names.push(format!("{} 0", tile.name));

                    for i in 1..cardinality {
                        if i <= 3 {
                            let mut new_tile = tiles.get(t + i as usize - 1).unwrap().clone();
                            new_tile.rotate_90();
                            tiles.push(new_tile);
                        } else if i >= 4 {
                            let mut new_tile = tiles.get(t + i as usize - 1).unwrap().clone();
                            new_tile.fliph();
                            tiles.push(new_tile);
                        }
                        tile_names.push(format!("{} {}", tile.name, i));
                    }
                } else if let Err(err) = img {
                    Err(err)?
                }
            }
        }
        let tile_size = tiles.get(0).unwrap().image.width() as usize;

        Ok(SimpleTiledModel {
            tiles,
            tile_names,
            neighbors: config.neighbors,
            tile_size,
        })
    }
}
