use crate::params::DEFAULT_PATH_HOPS;
use crate::path_sampler::PathSampler;
use crate::topologygen::{MixNode, Topology};
use rand::thread_rng;
use std::collections::HashSet;

pub struct BandwidthRandomPathSampler {
    hops: usize,
    current_topology_index: Option<usize>,
}

impl BandwidthRandomPathSampler {
    pub fn new(hops: usize) -> Self {
        Self {
            hops,
            current_topology_index: None,
        }
    }
}

impl Default for BandwidthRandomPathSampler {
    fn default() -> Self {
        Self::new(DEFAULT_PATH_HOPS)
    }
}

impl PathSampler for BandwidthRandomPathSampler {
    fn sample_path(&mut self, topology_index: usize, topology: &Topology) -> Vec<MixNode> {
        self.current_topology_index = Some(topology_index);
        assert!(
            self.hops <= topology.active().len(),
            "cannot sample a {}-hop path from {} active mix nodes",
            self.hops,
            topology.active().len()
        );

        let mut path = Vec::with_capacity(self.hops);
        let mut used = HashSet::with_capacity(self.hops);
        let mut rng = thread_rng();
        while path.len() < self.hops {
            let sampled = topology
                .sample_active(&mut rng)
                .expect("active topology should contain at least one node");
            if used.insert(sampled.mix_id) {
                path.push(sampled.clone());
            }
        }
        path
    }

    fn hops(&self) -> usize {
        self.hops
    }

    fn sampler_type(&self) -> &'static str {
        "BandwidthRandomPathSampler"
    }
}
