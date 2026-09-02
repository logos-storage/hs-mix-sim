use crate::adversary::Adversary;
use crate::path_sampler::PathSampler;
use crate::usermodel::{RouteEvent, UserModel, UserModelInfo};
use rand::distributions::{Distribution, Uniform};
use rand::thread_rng;

const INTERVAL_MAX: u64 = 900;
const INTERVAL_MIN: u64 = 300;

pub struct SimpleModel<'a, S: PathSampler, A: Adversary> {
    model_info: UserModelInfo<'a>,
    /// timestamp of current time, starting at 0.
    current_time: u64,
    die: Uniform<u64>,
    path_sampler: S,
    adversary: A,
}

impl<'a, S: PathSampler, A: Adversary> SimpleModel<'a, S, A> {
    pub fn new(model_info: UserModelInfo<'a>, path_sampler: S, adversary: A) -> Self {
        Self {
            model_info,
            current_time: 0,
            die: Uniform::from(INTERVAL_MIN..INTERVAL_MAX),
            path_sampler,
            adversary,
        }
    }

    fn get_next_message_timing(&mut self) -> u64 {
        self.current_time += self.die.sample(&mut thread_rng());
        self.current_time
    }
}

impl<S: PathSampler, A: Adversary> UserModel for SimpleModel<'_, S, A> {
    fn fetch_next(&mut self) -> Option<RouteEvent> {
        let next_timing = self.get_next_message_timing();
        let (topology_index, topology) = self.model_info.topology_at(next_timing)?;
        let path = self.path_sampler.sample_path(topology_index, topology);
        let adversary_won = self.adversary.wins(&path);
        Some((next_timing, adversary_won))
    }
}
