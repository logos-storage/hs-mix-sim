use crate::params::{GUARD_HOP, GUARDS_SAMPLE_SIZE, GUARDS_SAMPLE_SIZE_EXTEND};
use crate::path_sampler::{HopBehavior, PathSampler};
use crate::path_sampler::bandwidth_random::{
    sample_bandwidth_path_with_fixed_hops, sample_weighted_unique,
};
use crate::topologygen::{MixNode, Topology};
use std::collections::HashSet;

pub struct PathSamplerWithGuards {
    hops: usize,
    current_topology_index: Option<usize>,
    guard_set: Vec<u32>,
    selected_guard: Option<u32>,
}

impl PathSamplerWithGuards {
    pub fn new(hops: usize) -> Self {
        assert!(
            GUARD_HOP < hops,
            "guard hop {GUARD_HOP} is outside a {hops}-hop path"
        );
        Self {
            hops,
            current_topology_index: None,
            guard_set: Vec::new(),
            selected_guard: None,
        }
    }

    pub(crate) fn selected_guard(&mut self, topology_index: usize, topology: &Topology) -> MixNode {
        if self.current_topology_index != Some(topology_index) {
            self.update_guard(topology);
            self.current_topology_index = Some(topology_index);
        }

        let guard_id = self
            .selected_guard
            .expect("guard sampler should have an online selected guard");
        find_active(topology, guard_id)
            .expect("selected guard should be online in the current topology")
            .clone()
    }

    pub(crate) fn guard_set(&self) -> &[u32] {
        &self.guard_set
    }

    fn update_guard(&mut self, topology: &Topology) {
        if self
            .selected_guard
            .is_some_and(|guard| find_active(topology, guard).is_some())
        {
            return;
        }

        self.selected_guard = self
            .guard_set
            .iter()
            .copied()
            .find(|guard| find_active(topology, *guard).is_some());
        if self.selected_guard.is_some() {
            return;
        }

        let sample_size = if self.guard_set.is_empty() {
            GUARDS_SAMPLE_SIZE
        } else {
            GUARDS_SAMPLE_SIZE_EXTEND
        };
        let known: HashSet<u32> = self.guard_set.iter().copied().collect();
        let additions = sample_weighted_unique(topology.guards(), sample_size, &known);
        self.guard_set
            .extend(additions.into_iter().map(|node| node.mix_id));
        self.selected_guard = self
            .guard_set
            .iter()
            .copied()
            .find(|guard| find_active(topology, *guard).is_some());

        assert!(
            self.selected_guard.is_some(),
            "current topology has no online guard available to this user"
        );
    }
}

impl PathSampler for PathSamplerWithGuards {
    fn sample_path(&mut self, topology_index: usize, topology: &Topology) -> Vec<MixNode> {
        let guard = self.selected_guard(topology_index, topology);
        sample_bandwidth_path_with_fixed_hops(topology, self.hops, &[(GUARD_HOP, guard)])
    }

    fn hop_behavior(&self, hop: usize) -> HopBehavior {
        todo!()
    }

    fn peak(&self, hop: usize, mix_node: u32) -> Vec<u32> {
        todo!()
    }

    fn hops(&self) -> usize {
        self.hops
    }

    fn sampler_type(&self) -> &'static str {
        "PathSamplerWithGuards"
    }
}

fn find_active(topology: &Topology, mix_id: u32) -> Option<&MixNode> {
    topology.active().iter().find(|node| node.mix_id == mix_id)
}