//! hidden-service traffic model.
//!
//! A request arrives every 5--15 minutes and produces 1--100 messages spread
//! over the following minute. Each message samples its own route.

use crate::path_sampler::PathSampler;
use crate::topologygen::MixNode;
use crate::usermodel::{RouteEvent, UserModel, UserModelInfo};
use rand::distributions::{Distribution, Uniform};
use rand::thread_rng;

const INTERVAL_MAX: u64 = 900;
const INTERVAL_MIN: u64 = 300;
const BURST_WINDOW_SECONDS: u64 = 60;
const MESSAGES_MAX: u64 = 100;
const MESSAGES_MIN: u64 = 1;

pub struct HiddenServiceModel<'a, S: PathSampler> {
    model_info: UserModelInfo<'a>,
    /// Start time of the current burst.
    current_time: u64,
    interval_die: Uniform<u64>,
    message_count_die: Uniform<u64>,
    burst_offset_die: Uniform<u64>,
    burst_messages: Vec<u64>,
    next_burst_message: usize,
    path_sampler: S,
}

impl<'a, S: PathSampler> HiddenServiceModel<'a, S> {
    pub fn new(model_info: UserModelInfo<'a>, path_sampler: S) -> Self {
        Self {
            model_info,
            current_time: 0,
            interval_die: Uniform::from(INTERVAL_MIN..=INTERVAL_MAX),
            message_count_die: Uniform::from(MESSAGES_MIN..=MESSAGES_MAX),
            burst_offset_die: Uniform::from(0..=BURST_WINDOW_SECONDS),
            burst_messages: Vec::new(),
            next_burst_message: 0,
            path_sampler,
        }
    }

    fn has_pending_burst_messages(&self) -> bool {
        self.next_burst_message < self.burst_messages.len()
    }

    fn start_next_burst(&mut self) {
        let mut rng = thread_rng();
        self.current_time += self.interval_die.sample(&mut rng);
        self.next_burst_message = 0;
        self.burst_messages.clear();

        let message_count = self.message_count_die.sample(&mut rng) as usize;
        self.burst_messages.reserve(message_count);
        for _ in 0..message_count {
            let offset = self.burst_offset_die.sample(&mut rng);
            self.burst_messages.push(self.current_time + offset);
        }
        self.burst_messages.sort_unstable();
    }

    fn next_message_timing(&mut self) -> u64 {
        if !self.has_pending_burst_messages() {
            self.start_next_burst();
        }

        let message_timing = self.burst_messages[self.next_burst_message];
        self.next_burst_message += 1;
        message_timing
    }
}

impl<S: PathSampler> UserModel for HiddenServiceModel<'_, S> {
    fn fetch_next(&mut self) -> Option<RouteEvent> {
        let next_timing = self.next_message_timing();
        let (topology_index, topology) = self.model_info.topology_at(next_timing)?;
        let path = self.path_sampler.sample_path(topology_index, topology);
        let adversary_won = Self::adversary_wins(&path);
        Some((next_timing, adversary_won))
    }

    fn adversary_wins(path: &[MixNode]) -> bool {
        !path.is_empty() && path.iter().all(|node| node.is_malicious)
    }
}
