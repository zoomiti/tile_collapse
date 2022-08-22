use std::{collections::HashMap, error::Error, fmt::Display};

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
        pub image: DynamicImage,
        pub weight: f64,
    }

    impl TileObject {
        pub fn rotate_90(&mut self) {
            self.image = self.image.rotate90();
            self.image = self.image.rotate90();
            self.image = self.image.rotate90();
        }

        pub fn fliph(&mut self) {
            self.image = self.image.fliph()
        }
    }
}
#[derive(Deserialize, Debug)]
struct Neighbor {
    left: String,
    right: String,
}

pub mod model {
    use std::path::Path;

    use image::{GenericImage, ImageBuffer};
    use rand::prelude::*;
    use rand_chacha::ChaCha8Rng;

    use crate::tile::TileObject;

    use super::*;

    static OPPOSITE: [usize; 4] = [2, 3, 0, 1];
    static DX: [isize; 4] = [-1, 0, 1, 0];
    static DY: [isize; 4] = [0, 1, 0, -1];

    #[derive(PartialEq, Debug)]
    #[allow(dead_code)]
    enum Heuristic {
        Entropy,
        MRV,
        ScanLine,
    }

    pub trait Model {
        fn run(&mut self, seed: u64, limit: isize) -> bool;
        fn save(&self, path: &Path) -> Result<(), Box<dyn Error>>;
    }

    #[derive(Debug)]
    pub struct SimpleTiledModel {
        tiles: Vec<TileObject>,

        tile_names: Vec<String>,

        tile_size: usize,

        // Model.cs stuff
        wave: Vec<Vec<bool>>,
        propagator: Vec<Vec<Vec<usize>>>,
        compatible: Vec<Vec<Vec<isize>>>,
        observed: Vec<Option<usize>>,

        stack: Vec<(usize, usize)>,
        observed_so_far: usize,

        width: usize,
        height: usize,
        t: usize,
        n: usize,

        //periodic: bool,
        weight_log_weights: Vec<f64>,
        distribution: Vec<f64>,

        sums_of_ones: Vec<isize>,

        sum_of_weights: f64,
        sum_of_weight_log_weights: f64,
        starting_entropy: f64,

        sums_of_weights: Vec<f64>,
        sums_of_weight_log_weights: Vec<f64>,
        entropies: Vec<f64>,

        heuristic: Heuristic,
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
                        cardinality = 1;
                        a = |i| i;
                        b = |i| i;
                    }
                }

                let t = action.len();
                first_occurence.insert(
                    Path::new(&tile.name)
                        .file_stem()
                        .and_then(std::ffi::OsStr::to_str)
                        .and_then(|s| Some(s.to_owned()))
                        .unwrap(),
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
                    let image = image::open(format!("{}/{}", folder, tile.name))?;
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
                            let mut new_tile = tiles.get(t + i as usize - 4).unwrap().clone();
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
                    0
                } else {
                    left[1].parse().unwrap()
                }]
                .try_into()
                .unwrap();
                let d = action[l][1] as usize;
                let r: usize = action[*first_occurence.get(&right[0]).unwrap()][if right.len() == 1
                {
                    0
                } else {
                    right[1].parse().unwrap()
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

            let mut sparse_propagator: Vec<Vec<Vec<usize>>> = vec![vec![vec![]; t]; 4];

            for (d, (sp, tp)) in sparse_propagator
                .iter_mut()
                .zip(dense_propagater)
                .enumerate()
            {
                for (t1, (sp, tp)) in sp.iter_mut().zip(tp).enumerate() {
                    for t2 in 0..t {
                        if tp[t2] {
                            sp.push(t2);
                        }
                    }

                    if sp.is_empty() {
                        eprintln!(
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
            let sum_of_weights = tiles.iter().map(|t| t.weight).sum::<f64>();
            let sum_of_weight_log_weights =
                tiles.iter().map(|t| t.weight).map(|w| w * w.ln()).sum();
            let starting_entropy = sum_of_weights.ln() - sum_of_weight_log_weights / sum_of_weights;

            Ok(SimpleTiledModel {
                tiles,
                tile_names,
                tile_size,
                wave: vec![vec![true; t]; width * height],
                propagator,
                compatible: vec![vec![vec![0; 4]; t]; width * height],
                observed: vec![None; width * height],
                stack: vec![],
                observed_so_far: 0,
                width,
                height,
                t,
                n: 1,
                weight_log_weights: vec![0.; t],
                distribution: vec![0.; t],
                sums_of_ones: vec![0; width * height],
                sum_of_weights,
                sum_of_weight_log_weights,
                starting_entropy,
                sums_of_weights: vec![0.; width * height],
                sums_of_weight_log_weights: vec![0.0; width * height],
                entropies: vec![starting_entropy; width * height],
                heuristic: Heuristic::Entropy,
            })
        }
        fn clear(&mut self) {
            for i in 0..self.wave.len() {
                for t in 0..self.t {
                    self.wave[i][t] = true;
                    for d in 0..4 {
                        self.compatible[i][t][d] = self.propagator[OPPOSITE[d]][t].len() as isize;
                    }
                }
                self.sums_of_ones[i] = self.tiles.len() as isize;
                self.sums_of_weights[i] = self.sum_of_weights;
                self.sums_of_weight_log_weights[i] = self.sum_of_weight_log_weights;
                self.entropies[i] = self.starting_entropy;
                self.observed[i] = None;
            }
            self.observed_so_far = 0;
        }
        fn next_unobserved_node(&mut self, rng: &mut ChaCha8Rng) -> Option<usize> {
            let mut min = 10_000.;
            let mut argmin = None;
            for (i, remaining_values) in self.sums_of_ones.iter().enumerate() {
                // TODO: add periodic
                if i % self.width + self.n > self.width || i / self.width + self.n > self.height {
                    continue;
                }
                let entropy = if self.heuristic == Heuristic::Entropy {
                    self.entropies[i]
                } else {
                    *remaining_values as f64
                };
                if *remaining_values > 1 && entropy <= min {
                    let noise = 0.000001 * rng.gen::<f64>();
                    if entropy + noise < min {
                        min = entropy + noise;
                        argmin = Some(i);
                    }
                }
            }
            argmin
        }
        fn observe(&mut self, node: usize, rng: &mut ChaCha8Rng) {
            let w = &self.wave[node];
            for ((distribution, w), weight) in self
                .distribution
                .iter_mut()
                .zip(w)
                .zip(self.tiles.iter().map(|t| t.weight))
            {
                *distribution = if *w { weight } else { 0.0 };
            }
            let r = random_from_distr(&self.distribution, rng.gen());
            for t in 0..self.t {
                if self.wave[node][t] != (t == r) {
                    self.ban(node, t);
                }
            }
        }
        fn ban(&mut self, i: usize, t: usize) {
            self.wave[i][t] = false;

            let comp = &mut self.compatible[i][t];
            for c in comp {
                *c = 0;
            }
            self.stack.push((i, t));

            self.sums_of_ones[i] -= 1;
            self.sums_of_weights[i] -= self.tiles[t].weight;
            self.sums_of_weight_log_weights[i] -= self.weight_log_weights[t];

            let sum = self.sums_of_weights[i];
            self.entropies[i] = sum.ln() - self.sums_of_weight_log_weights[i] / sum;
        }
        fn propagate(&mut self) -> bool {
            while let Some((i1, t1)) = self.stack.pop() {
                let x1 = i1 % self.width;
                let y1 = i1 / self.height;

                for d in 0..4 {
                    let width = self.width as isize;
                    let height = self.height as isize;
                    let mut x2 = x1 as isize + DX[d];
                    let mut y2 = y1 as isize + DY[d];
                    //TODO: implement periodic
                    if x2 < 0
                        || y2 < 0
                        || x2 as usize + self.n > self.width
                        || y2 as usize + self.n > self.height
                    {
                        continue;
                    }

                    if x2 < 0 {
                        x2 += width;
                    } else if x2 >= width {
                        x2 -= width;
                    }
                    if y2 < 0 {
                        y2 += height;
                    } else if y2 >= height {
                        y2 -= height;
                    }

                    let i2 = x2 + y2 * width;

                    let mut ban_list = vec![];
                    for t2 in &self.propagator[d][t1] {
                        self.compatible[i2 as usize][*t2][d] -= 1;
                        if self.compatible[i2 as usize][*t2][d] == 0 {
                            ban_list.push(*t2);
                        }
                    }

                    for t2 in ban_list {
                        self.ban(i2 as usize, t2)
                    }
                }
            }
            self.sums_of_ones[0] > 0
        }
    }

    impl Model for SimpleTiledModel {
        fn run(&mut self, seed: u64, limit: isize) -> bool {
            println!("Ran this model");
            self.clear();
            let mut rng = ChaCha8Rng::seed_from_u64(seed);

            for _ in 0..limit {
                if let Some(node) = self.next_unobserved_node(&mut rng) {
                    self.observe(node, &mut rng);
                    let success = self.propagate();
                    if !success {
                        return false;
                    }
                } else {
                    for i in 0..self.wave.len() {
                        for t in 0..self.wave[i].len() {
                            if self.wave[i][t] {
                                self.observed[i] = Some(t);
                                break;
                            }
                        }
                    }
                    println!("{:?}", self.observed);
                    return self.observed.iter().any(|obs| obs.is_none());
                }
            }
            true
        }

        fn save(&self, path: &Path) -> Result<(), Box<dyn Error>> {
            let mut imgbuf = ImageBuffer::new(
                (self.width * self.tile_size) as u32,
                (self.height * self.tile_size) as u32,
            );
            for y in 0..self.height {
                for x in 0..self.width {
                    imgbuf.copy_from(
                        &self.tiles[self.observed[x + y * self.width].unwrap()].image,
                        (x * self.tile_size) as u32,
                        (y * self.tile_size) as u32,
                    )?;
                }
            }
            imgbuf.save(path)?;
            Ok(())
        }
    }

    impl Display for SimpleTiledModel {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let count = self
                .observed
                .iter()
                .fold(0, |acc, obs| if obs.is_none() { acc + 1 } else { acc });
            if count > 0 {
                write!(f, "{count} unobserved tiles")?;
            } else {
                for y in 0..self.height {
                    for x in 0..self.width {
                        write!(
                            f,
                            "{}, ",
                            self.tile_names[self.observed[x + y * self.width].unwrap()]
                        )?;
                    }
                    writeln!(f)?;
                }
            }
            Ok(())
        }
    }
}

fn random_from_distr(weights: &[f64], r: f64) -> usize {
    let sum = weights.iter().fold(0., |acc, w| acc + w);
    let threshold = r * sum;
    let mut partial_sum = 0.;
    for (i, weight) in weights.iter().enumerate() {
        partial_sum += weight;
        if partial_sum >= threshold {
            return i;
        }
    }
    0
}
