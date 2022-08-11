use std::collections::HashMap;
use std::error::Error;
use std::fmt::Display;

use image::DynamicImage;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Config {
    tiles: Vec<tile::Tile>,
    neighbors: Vec<Neighbor>,
}

mod tile {
    use super::*;

    #[derive(Deserialize, Debug)]
    pub struct Tile {
        pub name: String,
        pub symmetry: String,
        pub weight: Option<f64>,
    }

    #[derive(Clone, Debug)]
    pub struct TileObject {
        #[allow(dead_code)]
        pub image: DynamicImage,
        #[allow(dead_code)]
        pub weight: f64,
    }

    impl TileObject {
        pub fn rotate_90(&mut self) {
            self.image = self.image.rotate90();
        }

        pub fn fliph(&mut self) {
            self.image = self.image.fliph()
        }
    }
}
#[derive(Deserialize, Debug)]
struct Neighbor {
    #[allow(dead_code)]
    left: String,
    #[allow(dead_code)]
    right: String,
}

pub mod model {
    use std::path::Path;

    use crate::tile::TileObject;

    use super::*;

    trait Model {
        fn get_width(&self) -> usize;
        fn get_height(&self) -> usize;
        fn get_observed(&mut self) -> &mut Vec<i32>;
    }

    #[derive(Debug)]
    pub struct SimpleTiledModel {
        #[allow(dead_code)]
        tiles: Vec<TileObject>,

        #[allow(dead_code)]
        tile_names: Vec<String>,

        #[allow(dead_code)]
        tile_size: usize,

        #[allow(dead_code)]
        propagator: Vec<Vec<Vec<i32>>>,

        width: usize,
        height: usize,

        observed: Vec<i32>,
    }

    impl SimpleTiledModel {
        pub fn new(
            config: Config,
            folder: &str,
            width: usize,
            height: usize,
        ) -> Result<SimpleTiledModel, Box<dyn Error>> {
            if config.tiles.is_empty() {
                Err("No tiles in config file")?
            }

            let mut tiles = Vec::new();
            let mut tile_names = Vec::new();

            let mut action: Vec<[i32; 8]> = Vec::new();
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
                        a = |i| 1 - i;
                        b = |i| i;
                    }
                    b'\\' => {
                        cardinality = 2;
                        a = |i| 1 - i;
                        b = |i| 1 - i;
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
                first_occurence.insert(
                    Path::new(&tile.name)
                        .file_stem()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .to_string(),
                    t,
                );

                let mut map: [[i32; 8]; 8] = [[0; 8]; 8];
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
                    let image = image::open(format!("{}{}", folder, tile.name))?;
                    tiles.push(TileObject {
                        image: image.clone(),
                        weight: tile.weight.unwrap_or(1.0),
                    });
                    tile_names.push(format!(
                        "{} 0",
                        Path::new(&tile.name).file_stem().unwrap().to_str().unwrap()
                    ));

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
                        tile_names.push(format!(
                            "{} {}",
                            Path::new(&tile.name).file_stem().unwrap().to_str().unwrap(),
                            i
                        ));
                    }
                }
            }
            let t: usize = action.len();

            let mut dense_propagater = vec![vec![vec![false; t]; t]; 4];
            let mut propagator = vec![vec![vec![]; t]; 4];

            for neighbor in &config.neighbors {
                // TODO: implement subsets here
                let left: Vec<String> = neighbor.left.split(' ').map(str::to_string).collect();
                let right: Vec<String> = neighbor.right.split(' ').map(str::to_string).collect();
                let l: usize = action[*first_occurence.get(&left[0]).unwrap()][if left.len() == 1 {
                    0 as usize
                } else {
                    left[1].parse().unwrap()
                }]
                .try_into()
                .unwrap();
                let d = action[l][1] as usize;
                let r: usize =
                    action[*first_occurence.get(&left[0]).unwrap()][if right.len() == 1 {
                        0
                    } else {
                        left[1].parse().unwrap()
                    }]
                    .try_into()
                    .unwrap();
                let u = action[r][1] as usize;

                dense_propagater[0][r][l] = true;
                dense_propagater[0][action[r][6] as usize][action[l][6] as usize] = true;
                dense_propagater[0][action[l][4] as usize][action[r][4] as usize] = true;
                dense_propagater[0][action[l][2] as usize][action[r][2] as usize] = true;

                dense_propagater[1][u][d] = true;
                dense_propagater[1][action[d][6] as usize][action[u][6] as usize] = true;
                dense_propagater[1][action[u][4] as usize][action[d][4] as usize] = true;
                dense_propagater[1][action[d][2] as usize][action[u][2] as usize] = true;
            }

            for t2 in 0..t {
                for t1 in 0..t {
                    dense_propagater[2][t2][t1] = dense_propagater[0][t1][t2];
                    dense_propagater[3][t2][t1] = dense_propagater[1][t1][t2];
                }
            }

            let mut sparse_propagator: Vec<Vec<Vec<i32>>> = vec![vec![vec![]; t]; 4];

            for (d, (sp, tp)) in sparse_propagator
                .iter_mut()
                .zip(dense_propagater)
                .enumerate()
            {
                for (t1, (sp, tp)) in sp.iter_mut().zip(tp).enumerate() {
                    for t2 in 0..t {
                        if tp[t2] {
                            sp.push(t2 as i32);
                        }
                    }

                    if sp.is_empty() {
                        println!(
                            "ERROR: tile {} has no neighbors in direction {}",
                            tile_names[t1], d
                        );
                    }
                    for (st, _) in sp.iter().enumerate() {
                        propagator[d][t1].push(sp[st]);
                    }
                }
            }

            let tile_size = tiles.get(0).unwrap().image.width() as usize;

            Ok(SimpleTiledModel {
                tiles,
                tile_names,
                tile_size,
                propagator,
                width,
                height,
                observed: vec![],
            })
        }
    }

    impl Model for SimpleTiledModel {
        fn get_width(&self) -> usize {
            self.width
        }

        fn get_height(&self) -> usize {
            self.height
        }

        fn get_observed(&mut self) -> &mut Vec<i32> {
            &mut self.observed
        }
    }

    impl Display for SimpleTiledModel {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            if self.observed.is_empty() {
                write!(f, "No observed tiles")?;
            } else {
                for y in 0..self.height {
                    for x in 0..self.width {
                        write!(
                            f,
                            "{}",
                            self.tile_names[self.observed[x + y * self.width] as usize]
                        )?;
                    }
                    writeln!(f)?;
                }
            }
            Ok(())
        }
    }
}
