use crate::params::{GUARD_HOP, VANGUARDS_SAMPLE_SIZE, VANGUARDS_SAMPLE_SIZE_EXTEND};
use crate::path_sampler::PathSampler;
use crate::path_sampler::bandwidth_random::{
    sample_bandwidth_path_with_fixed_hops, sample_weighted_unique,
};
use crate::path_sampler::guard::PathSamplerWithGuards;
use crate::topologygen::{MixNode, MixNodeTag, Topology};
use std::collections::HashSet;

#[derive(Default)]
struct VanguardSelection {
    candidates: Vec<u32>,
    selected: Option<u32>,
}

pub struct PathSamplerWithVanguards {
    hops: usize,
    guard_sampler: PathSamplerWithGuards,
    current_topology_index: Option<usize>,
    vanguard_hops: Vec<usize>,
    vanguard_sets: Vec<VanguardSelection>,
}

impl PathSamplerWithVanguards {
    pub fn new(hops: usize, vanguards: usize) -> Self {
        assert!(vanguards > 0, "vanguard mode needs at least one vanguard");
        assert!(
            hops > 1 && vanguards < hops - 1,
            "the number of vanguards ({vanguards}) must be less than hops - 1 ({})",
            hops.saturating_sub(1)
        );

        let vanguard_hops = (0..hops)
            .filter(|hop| *hop != GUARD_HOP)
            .take(vanguards)
            .collect();
        let vanguard_sets = (0..vanguards)
            .map(|_| VanguardSelection::default())
            .collect();

        Self {
            hops,
            guard_sampler: PathSamplerWithGuards::new(hops),
            current_topology_index: None,
            vanguard_hops,
            vanguard_sets,
        }
    }

    fn update_vanguards(&mut self, topology: &Topology, guard_id: u32) {
        let mut unavailable: HashSet<u32> =
            self.guard_sampler.guard_set().iter().copied().collect();
        for selection in &self.vanguard_sets {
            unavailable.extend(selection.candidates.iter().copied());
        }

        let mut selected_for_path = HashSet::from([guard_id]);
        for selection in &mut self.vanguard_sets {
            if !selection.selected.is_some_and(|vanguard| {
                !selected_for_path.contains(&vanguard) && find_active(topology, vanguard).is_some()
            }) {
                selection.selected = selection.candidates.iter().copied().find(|vanguard| {
                    !selected_for_path.contains(vanguard)
                        && find_active(topology, *vanguard).is_some()
                });
            }

            if selection.selected.is_none() {
                let sample_size = if selection.candidates.is_empty() {
                    VANGUARDS_SAMPLE_SIZE
                } else {
                    VANGUARDS_SAMPLE_SIZE_EXTEND
                };
                let eligible = topology
                    .active()
                    .iter()
                    .filter(|node| !node.has_tag(MixNodeTag::Guard));
                let additions = sample_weighted_unique(eligible, sample_size, &unavailable);
                for addition in additions {
                    unavailable.insert(addition.mix_id);
                    selection.candidates.push(addition.mix_id);
                }
                selection.selected = selection.candidates.iter().copied().find(|vanguard| {
                    !selected_for_path.contains(vanguard)
                        && find_active(topology, *vanguard).is_some()
                });
            }

            let selected = selection
                .selected
                .expect("current topology has no online vanguard available to this user");
            selected_for_path.insert(selected);
        }
    }
}

impl PathSampler for PathSamplerWithVanguards {
    fn sample_path(&mut self, topology_index: usize, topology: &Topology) -> Vec<MixNode> {
        let guard = self.guard_sampler.selected_guard(topology_index, topology);
        if self.current_topology_index != Some(topology_index) {
            self.update_vanguards(topology, guard.mix_id);
            self.current_topology_index = Some(topology_index);
        }

        let mut fixed_hops = Vec::with_capacity(self.vanguard_sets.len() + 1);
        fixed_hops.push((GUARD_HOP, guard));
        for (hop, selection) in self.vanguard_hops.iter().zip(&self.vanguard_sets) {
            let vanguard = find_active(
                topology,
                selection
                    .selected
                    .expect("vanguard sampler should have a selected vanguard"),
            )
            .expect("selected vanguard should be online in the current topology")
            .clone();
            fixed_hops.push((*hop, vanguard));
        }

        sample_bandwidth_path_with_fixed_hops(topology, self.hops, &fixed_hops)
    }

    fn hops(&self) -> usize {
        self.hops
    }

    fn sampler_type(&self) -> &'static str {
        "PathSamplerWithVanguards"
    }
}

fn find_active(topology: &Topology, mix_id: u32) -> Option<&MixNode> {
    topology.active().iter().find(|node| node.mix_id == mix_id)
}