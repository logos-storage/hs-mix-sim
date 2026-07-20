use crate::path_sampler::PathSampler;
use crate::topologygen::MixNode;
use crate::usermodel::{RouteEvent, UserModel, UserModelInfo};
use rand::distributions::{Distribution, Uniform};
use rand::thread_rng;

const INTERVAL_MAX: u64 = 900;
const INTERVAL_MIN: u64 = 300;

pub struct SimpleModel<'a, S: PathSampler> {
    model_info: UserModelInfo<'a>,
    /// timestamp of current time, starting at 0.
    current_time: u64,
    die: Uniform<u64>,
    path_sampler: S,
}

impl<'a, S: PathSampler> SimpleModel<'a, S> {
    pub fn new(model_info: UserModelInfo<'a>, path_sampler: S) -> Self {
        Self {
            model_info,
            current_time: 0,
            die: Uniform::from(INTERVAL_MIN..INTERVAL_MAX),
            path_sampler,
        }
    }

    fn get_next_message_timing(&mut self) -> u64 {
        self.current_time += self.die.sample(&mut thread_rng());
        self.current_time
    }
}

impl<S: PathSampler> UserModel for SimpleModel<'_, S> {
    fn fetch_next(&mut self) -> Option<RouteEvent> {
        let next_timing = self.get_next_message_timing();
        let (topology_index, topology) = self.model_info.topology_at(next_timing)?;
        let path = self.path_sampler.sample_path(topology_index, topology);
        let is_malicious = Self::is_path_malicious(&path);
        Some((next_timing, is_malicious))
    }

    fn is_path_malicious(path: &[MixNode]) -> bool {
        !path.is_empty() && path.iter().all(|node| node.is_malicious)
    }
}
