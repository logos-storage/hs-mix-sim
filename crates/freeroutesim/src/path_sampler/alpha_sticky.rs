use crate::mixnet::{MixNode, Mixnet};
use crate::path_sampler::PathSampler;
use rand::Rng;
use rand::thread_rng;
use std::collections::HashSet;

/// α-Sticky Selection (α-SS) path sampler.
///
/// The first path is new. Each later selection uniformly reuses one distinct
/// path from the leading set with probability `alpha`; otherwise it introduces
/// a uniformly sampled path that has not previously appeared in this session.
pub struct AlphaStickyPathSampler {
    hops: usize,
    alpha: f64,
    leading_paths: Vec<Vec<usize>>,
    used_paths: HashSet<Vec<usize>>,
}

impl AlphaStickyPathSampler {
    pub fn new(hops: usize, alpha: f64) -> Self {
        assert!(hops > 0, "path length must be greater than zero");
        assert!(
            (0.0..=1.0).contains(&alpha),
            "alpha must be between 0 and 1"
        );

        Self {
            hops,
            alpha,
            leading_paths: Vec::new(),
            used_paths: HashSet::new(),
        }
    }

    fn sample_new_path(&mut self, total_nodes: usize) -> Vec<usize> {
        assert!(
            (self.used_paths.len() as u128) < path_space_size(total_nodes, self.hops),
            "alpha-sticky sampler exhausted the available path space"
        );

        let mut rng = thread_rng();
        loop {
            let mut used_nodes = HashSet::with_capacity(self.hops);
            let mut path = Vec::with_capacity(self.hops);
            while path.len() < self.hops {
                let node_index = rng.gen_range(0..total_nodes);
                if used_nodes.insert(node_index) {
                    path.push(node_index);
                }
            }

            if self.used_paths.insert(path.clone()) {
                self.leading_paths.push(path.clone());
                return path;
            }
        }
    }
}

impl PathSampler for AlphaStickyPathSampler {
    fn sample_path(&mut self, mixnet: &Mixnet) -> Vec<MixNode> {
        assert!(
            self.hops <= mixnet.nodes().len(),
            "cannot sample a {}-hop path from {} mix nodes",
            self.hops,
            mixnet.nodes().len()
        );

        let mut rng = thread_rng();
        let selected = if !self.leading_paths.is_empty() && rng.gen_bool(self.alpha) {
            let path_index = rng.gen_range(0..self.leading_paths.len());
            self.leading_paths[path_index].clone()
        } else {
            self.sample_new_path(mixnet.nodes().len())
        };

        selected
            .into_iter()
            .map(|node_index| mixnet.nodes()[node_index].clone())
            .collect()
    }

    fn hops(&self) -> usize {
        self.hops
    }

    fn sampler_type(&self) -> &'static str {
        "AlphaStickyPathSampler"
    }
}

fn path_space_size(total_nodes: usize, hops: usize) -> u128 {
    (0..hops).fold(1_u128, |paths, hop| {
        paths.saturating_mul((total_nodes - hop) as u128)
    })
}
