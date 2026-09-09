use crate::params::DEFAULT_PATH_HOPS;
use crate::path_sampler::{HopBehavior, PathSampler};
use crate::topologygen::{MixNode, Topology};
use rand::{Rng, thread_rng};
use std::collections::HashSet;

/// Uniform random path sampler. Mix-node bandwidth is ignored.
pub struct RandomPathSampler {
    hops: usize,
    current_topology_index: Option<usize>,
}

impl RandomPathSampler {
    pub fn new(hops: usize) -> Self {
        Self {
            hops,
            current_topology_index: None,
        }
    }
}

impl Default for RandomPathSampler {
    fn default() -> Self {
        Self::new(DEFAULT_PATH_HOPS)
    }
}

impl PathSampler for RandomPathSampler {
    fn sample_path(&mut self, topology_index: usize, topology: &Topology) -> Vec<MixNode> {
        self.current_topology_index = Some(topology_index);
        let active = topology.active();
        assert!(
            self.hops <= active.len(),
            "cannot sample a {}-hop path from {} active mix nodes",
            self.hops,
            active.len()
        );

        let mut rng = thread_rng();
        let mut used = HashSet::with_capacity(self.hops);
        let mut path = Vec::with_capacity(self.hops);
        while path.len() < self.hops {
            let node = &active[rng.gen_range(0..active.len())];
            if used.insert(node.mix_id) {
                path.push(node.clone());
            }
        }
        path
    }

    fn hops(&self) -> usize {
        self.hops
    }

    fn sampler_type(&self) -> &'static str {
        "RandomPathSampler"
    }
}
