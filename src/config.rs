use array_init::array_init;
use rand::prelude::*;
use rand_distr::weighted_alias::WeightedAliasIndex;
use rustc_hash::FxHashMap as HashMap;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::path::Path;
use std::vec::IntoIter;

pub const PATH_LENGTH: i8 = 3;
#[allow(dead_code)]
/// in byte
pub const PAYLOAD_SIZE: usize = 2048;
/// Default sample size for guards -- todo move this in clap
pub const GUARDS_SAMPLE_SIZE: usize = 5;
/// How much do we extend the sample size each time we ran out of guard?
pub const GUARDS_SAMPLE_SIZE_EXTEND: usize = 2;
/// guards are use in layer... [0..n[
pub const GUARDS_LAYER: usize = 1;
/// Default sample size for vanguards.
pub const VANGUARDS_SAMPLE_SIZE: usize = 5;
/// How much do we extend the vanguard sample size each time we ran out.
pub const VANGUARDS_SAMPLE_SIZE_EXTEND: usize = 2;
/// vanguards are used in the first route hop.
pub const VANGUARDS_LAYER: usize = 0;

#[derive(Debug, Clone)]
pub struct Mixnode {
    pub layer: i8,
    pub weight: f64,
    pub mixid: u32,
    pub is_malicious: bool,
}

/// A config is a set of mixes for each layer
/// and a hashmap for unselected mixes.
#[derive(Default, Clone)]
pub struct TopologyConfig {
    #[allow(dead_code)]
    pub filename: String,

    pub epoch: u32,
    /// The path length
    layers: [Vec<Mixnode>; PATH_LENGTH as usize],
    wc_layers: [Box<Option<WeightedAliasIndex<f64>>>; PATH_LENGTH as usize],
    unselected: HashMap<u32, Mixnode>,
}

impl TopologyConfig {
    pub fn new(filename: String, epoch: u32) -> Self {
        TopologyConfig {
            filename,
            epoch,
            wc_layers: array_init(|_| Box::new(None)),
            ..Default::default()
        }
    }

    #[allow(dead_code)]
    pub fn layers(&self) -> &[Vec<Mixnode>] {
        &self.layers
    }
    #[allow(dead_code)]
    pub fn unselected(&self) -> &HashMap<u32, Mixnode> {
        &self.unselected
    }
    pub fn has_mix_in_layer(&self, layer: usize, mixid: u32) -> bool {
        self.layers[layer].iter().any(|mix| mix.mixid == mixid)
    }

    fn push_mix(&mut self, mut mix: Mixnode, layer: i8) {
        mix.layer = layer;
        match mix.layer {
            // This trick gets arround the unsupported excluded range syntax for now
            0..=PATH_LENGTH if mix.layer < PATH_LENGTH => self.layers[mix.layer as usize].push(mix),
            _ => {
                self.unselected.insert(mix.mixid, mix);
            }
        };
    }

    fn build_weighted_samplers(&mut self) {
        for i in 0..PATH_LENGTH {
            self.wc_layers[i as usize] = Box::new(Some(
                WeightedAliasIndex::new(
                    self.layers[i as usize]
                        .iter()
                        .map(|item| item.weight)
                        .collect(),
                )
                .unwrap_or_else(|_| {
                    panic!(
                        "Unable to build weighted sampler for {} epoch {} layer {}",
                        self.filename, self.epoch, i
                    )
                }),
            ));
        }
    }

    /// Sample n mixes from layer l.
    pub fn sample_layer<'a, R: Rng + ?Sized>(
        &'a self,
        l: usize,
        n_mixes: usize,
        rng: &mut R,
    ) -> IntoIter<&'a Mixnode> {
        let mut sample_mixes = vec![];
        for _ in 0..n_mixes {
            if let Some(wc) = &*self.wc_layers[l] {
                sample_mixes.push(&self.layers[l][wc.sample(rng)]);
            }
        }
        sample_mixes.into_iter()
    }

    /// Sample a route from the network layer configuration
    #[inline]
    pub fn sample_path<'a, R: Rng + ?Sized>(
        &'a self,
        rng: &mut R,
        vanguard: Option<&'a Mixnode>,
        guard: Option<&'a Mixnode>,
    ) -> IntoIter<&'a Mixnode> {
        let mut path = vec![];
        // returns an owned iterator
        for i in 0..PATH_LENGTH {
            if let Some(wc) = &*self.wc_layers[i as usize] {
                match (vanguard, guard) {
                    (Some(v), _) if i as usize == VANGUARDS_LAYER => path.push(v),
                    (_, Some(g)) if i as usize == GUARDS_LAYER => path.push(g),
                    _ => path.push(&self.layers[i as usize][wc.sample(rng)]),
                }
            }
        }
        path.into_iter()
    }
}

/// Load all topology epochs from one layout file.
///
/// Each row must start with:
/// mixid [integer], weight [float], is_malicious [bool]
///
/// Remaining columns are layer assignments. A single `layer` column produces one topology.
/// Multiple `epoch_N` columns produce one topology per epoch.
pub fn load<P>(filename: P) -> Vec<TopologyConfig>
where
    P: AsRef<Path>,
{
    let file = File::open(&filename).expect("Unable to open the file");
    let filename_string: String = filename.as_ref().to_str().unwrap().to_owned();
    let mut line_reader = BufReader::new(file).lines();
    let header = line_reader
        .next()
        .expect("Topology file is empty")
        .expect("Unable to read topology header");
    let header_cols: Vec<_> = header.split(',').map(|col| col.trim()).collect();
    if header_cols.len() < 4 {
        panic!(
            "Topology file {} must have at least 4 columns",
            filename.as_ref().display()
        );
    }

    let epochs = parse_epoch_columns(&header_cols[3..]);
    let mut configs: Vec<TopologyConfig> = epochs
        .iter()
        .map(|epoch| TopologyConfig::new(filename_string.clone(), *epoch))
        .collect();

    for line_r in line_reader {
        let line = line_r.expect("Something went wrong while reading topology file");
        let cols: Vec<_> = line.split(',').map(|col| col.trim()).collect();
        if cols.len() < header_cols.len() {
            panic!(
                "Unable to parse {} into a topology row -- expected {} columns, got {}: {}",
                filename.as_ref().display(),
                header_cols.len(),
                cols.len(),
                line
            );
        }

        let mix = parse_mix_row(&cols, filename.as_ref());
        for (idx, config) in configs.iter_mut().enumerate() {
            let layer = cols[idx + 3].parse::<i8>().unwrap_or_else(|_| {
                panic!(
                    "Unable to parse layer value '{}' in {}",
                    cols[idx + 3],
                    filename.as_ref().display()
                )
            });
            config.push_mix(mix.clone(), layer);
        }
    }

    for config in &mut configs {
        config.build_weighted_samplers();
    }

    configs
}

fn parse_epoch_columns(columns: &[&str]) -> Vec<u32> {
    columns
        .iter()
        .enumerate()
        .map(|(idx, column)| {
            if *column == "layer" {
                idx as u32
            } else {
                column
                    .split('_')
                    .nth(1)
                    .and_then(|epoch| epoch.parse::<u32>().ok())
                    .unwrap_or(idx as u32)
            }
        })
        .collect()
}

fn parse_mix_row(cols: &[&str], filename: &Path) -> Mixnode {
    let mixid = cols[0].parse::<u32>().unwrap_or_else(|_| {
        panic!(
            "Unable to parse mix id '{}' in {}",
            cols[0],
            filename.display()
        )
    });
    let weight = cols[1].parse::<f64>().unwrap_or_else(|_| {
        panic!(
            "Unable to parse bandwidth '{}' in {}",
            cols[1],
            filename.display()
        )
    });
    let is_malicious = cols[2].to_lowercase().parse::<bool>().unwrap_or(false);

    Mixnode {
        layer: -1,
        weight,
        mixid,
        is_malicious,
    }
}

#[test]
fn load_test_topology_config() {
    let configs = load("testfiles/single_layout/1000_137_Random_BP_layout.csv");
    let config = &configs[0];
    let mix = &config.layers()[0][42];
    //42│20.430784458454426│False│0
    assert_eq!(mix.is_malicious, false);
    assert_eq!(mix.weight, 1.8480844586590168);
}
#[test]
fn test_sample_path() {
    let configs = load("testfiles/single_layout/1000_137_Random_BP_layout.csv");
    let config = &configs[0];
    let mut rng = thread_rng();
    let path = config.sample_path(&mut rng, None, None);
    let path = path.collect::<Vec<_>>();
    assert_eq!(path.len(), 3);
}

#[test]
fn test_sample_path_uses_fixed_vanguard_and_guard() {
    let configs = load("testfiles/single_layout/1000_137_Random_BP_layout.csv");
    let config = &configs[0];
    let mut rng = thread_rng();
    let vanguard = &config.layers()[VANGUARDS_LAYER][0];
    let guard = &config.layers()[GUARDS_LAYER][0];
    let path = config
        .sample_path(&mut rng, Some(vanguard), Some(guard))
        .collect::<Vec<_>>();

    assert_eq!(path.len(), 3);
    assert_eq!(path[0].mixid, vanguard.mixid);
    assert_eq!(path[1].mixid, guard.mixid);
    assert_eq!(path[2].layer, 2);
}

#[test]
fn load_multi_epoch_layout() {
    let configs = load("testfiles/layout_data/bow_tie/dynamic_hybrid_steady_0.03_layout.csv");
    assert_eq!(configs.len(), 1000);
    assert_eq!(configs[0].epoch, 0);
    assert_eq!(configs[999].epoch, 999);
    assert_eq!(configs[0].layers().len(), PATH_LENGTH as usize);
}