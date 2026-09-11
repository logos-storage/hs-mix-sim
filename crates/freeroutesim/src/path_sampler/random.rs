use crate::mixnet::{MixNode, Mixnet};
use crate::params::DEFAULT_PATH_HOPS;
use crate::path_sampler::PathSampler;
use rand::{Rng, thread_rng};
use std::collections::HashSet;

/// Uniform random path sampler. Mix-node bandwidth is ignored.
pub struct RandomPathSampler {
    hops: usize,
}

impl RandomPathSampler {
    pub fn new(hops: usize) -> Self {
        Self { hops }
    }
}

impl Default for RandomPathSampler {
    fn default() -> Self {
        Self::new(DEFAULT_PATH_HOPS)
    }
}

impl PathSampler for RandomPathSampler {
    fn sample_path(&mut self, mixnet: &Mixnet) -> Vec<MixNode> {
        let nodes = mixnet.nodes();
        assert!(
            self.hops <= nodes.len(),
            "cannot sample a {}-hop path from {} mix nodes",
            self.hops,
            nodes.len()
        );

        let mut rng = thread_rng();
        let mut used = HashSet::with_capacity(self.hops);
        let mut path = Vec::with_capacity(self.hops);
        while path.len() < self.hops {
            let node = &nodes[rng.gen_range(0..nodes.len())];
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
