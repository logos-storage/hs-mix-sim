//! hidden-service traffic model.
//!
//! Time moves every 100 sec, the model checks with if the adversary wins
//! The adversary wins depending on the implemented model
//! each adversary model defines it own view of the path sampler, therefore,
//! the path sampler is supplied to the adversary.

use crate::adversary::Adversary;
use crate::path_sampler::PathSampler;
use crate::usermodel::{RouteEvent, UserModel, UserModelInfo};

const TIME_TICK: u64 = 100;

pub struct HiddenServiceModel<'a, S: PathSampler, A: Adversary> {
    model_info: UserModelInfo<'a>,
    /// Start time of the current burst.
    current_time: u64,
    path_sampler: S,
    adversary: A,
}

impl<'a, S: PathSampler, A: Adversary> HiddenServiceModel<'a, S, A> {
    pub fn new(model_info: UserModelInfo<'a>, path_sampler: S, adversary: A) -> Self {
        Self {
            model_info,
            current_time: 0,
            path_sampler,
            adversary,
        }
    }

    fn next_message_timing(&mut self) -> u64 {
        self.current_time += TIME_TICK;
        self.current_time
    }
}

impl<S: PathSampler, A: Adversary> UserModel for HiddenServiceModel<'_, S, A> {
    fn fetch_next(&mut self) -> Option<RouteEvent> {
        let next_timing = self.next_message_timing();
        let (topology_index, topology) = self.model_info.topology_at(next_timing)?;
        let path = self.path_sampler.sample_path(topology_index, topology);
        let adversary_won = self.adversary.wins(&path);
        Some((next_timing, adversary_won))
    }
}
