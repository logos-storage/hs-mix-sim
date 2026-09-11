use crate::mixnet::{MixNode, Mixnet};
use crate::params::DEFAULT_PATH_HOPS;
use crate::path_sampler::PathSampler;
use rand::thread_rng;
use std::collections::HashSet;

pub struct BandwidthRandomPathSampler {
    hops: usize,
}

impl BandwidthRandomPathSampler {
    pub fn new(hops: usize) -> Self {
        Self { hops }
    }
}

impl Default for BandwidthRandomPathSampler {
    fn default() -> Self {
        Self::new(DEFAULT_PATH_HOPS)
    }
}

impl PathSampler for BandwidthRandomPathSampler {
    fn sample_path(&mut self, mixnet: &Mixnet) -> Vec<MixNode> {
        assert!(
            self.hops <= mixnet.nodes().len(),
            "cannot sample a {}-hop path from {} mix nodes",
            self.hops,
            mixnet.nodes().len()
        );

        let mut path = Vec::with_capacity(self.hops);
        let mut used = HashSet::with_capacity(self.hops);
        let mut rng = thread_rng();
        while path.len() < self.hops {
            let sampled = mixnet
                .sample_by_bandwidth(&mut rng)
                .expect("mixnet should contain at least one node");
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
