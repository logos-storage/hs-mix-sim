use crate::adversary::PathAdversary;
use crate::mixnet::Mixnet;
use crate::path_sampler::PathSampler;
use crate::usermodel::{RouteEvent, UserModel};
use rand::distributions::{Distribution, Uniform};
use rand::thread_rng;

pub(crate) const INTERVAL_MAX: u64 = 900;
pub(crate) const INTERVAL_MIN: u64 = 300;

pub struct SimpleModel<'a, S: PathSampler, A: PathAdversary> {
    mixnet: &'a Mixnet,
    /// timestamp of current time, starting at 0.
    current_time: u64,
    deadline: u64,
    die: Uniform<u64>,
    path_sampler: S,
    adversary: A,
}

impl<'a, S: PathSampler, A: PathAdversary> SimpleModel<'a, S, A> {
    pub fn new(mixnet: &'a Mixnet, path_sampler: S, adversary: A, deadline: u64) -> Self {
        Self {
            mixnet,
            current_time: 0,
            deadline,
            die: Uniform::from(INTERVAL_MIN..INTERVAL_MAX),
            path_sampler,
            adversary,
        }
    }
}

impl<S: PathSampler, A: PathAdversary> UserModel for SimpleModel<'_, S, A> {
    fn fetch_next(&mut self) -> Option<RouteEvent> {
        if self.current_time >= self.deadline {
            return None;
        }
        let next_timing = self.current_time + self.die.sample(&mut thread_rng());
        if next_timing > self.deadline {
            self.current_time = self.deadline;
            return None;
        }
        self.current_time = next_timing;
        let path = self.path_sampler.sample_path(self.mixnet);
        let adversary_won = self.adversary.wins(&path);
        Some((next_timing, adversary_won))
    }
}
