//! A simple hidden-service model.
//!
//! Each simulated service receives a client request after a random `[INTERVAL_MIN, INTERVAL_MAX]`
//! (`default = [5, 15] minute`)
//! interval. For every request, it sends a burst of `[MESSAGES_MIN, MESSAGES_MAX]`
//! (`default = [1, 100]`) messages within the following `BURST_WINDOW_SECONDS` (default = 60 seconds).
use crate::usermodel::{RequestHandler, RouteEvent, UserModel, UserModelInfo};
use rand::distributions::{Distribution, Uniform};
use rand::rngs::SmallRng;
use rand::SeedableRng;

const INTERVAL_MAX: u64 = 900;
const INTERVAL_MIN: u64 = 300;
const BURST_WINDOW_SECONDS: u64 = 60;
const MESSAGES_MAX: u64 = 100;
const MESSAGES_MIN: u64 = 1;

pub struct SimpleHiddenServiceModel<'a> {
    /// timestamp of the current burst start, starting at 0.
    current_time: u64,
    /// the max value of a message timing
    limit: u64,
    rng: SmallRng,
    interval_die: Uniform<u64>,
    message_count_die: Uniform<u64>,
    burst_offset_die: Uniform<u64>,
    burst_messages: Vec<u64>,
    next_burst_message: usize,
    current_burst_id: u128,
    user_info: UserModelInfo<'a>,
}

impl<'a> UserModel<'a> for SimpleHiddenServiceModel<'a> {
    fn new(_tot_users: u32, _epoch: u32, user_info: UserModelInfo<'a>) -> Self {
        SimpleHiddenServiceModel {
            rng: SmallRng::from_entropy(),
            interval_die: Uniform::from(INTERVAL_MIN..=INTERVAL_MAX),
            message_count_die: Uniform::from(MESSAGES_MIN..=MESSAGES_MAX),
            burst_offset_die: Uniform::from(0..=BURST_WINDOW_SECONDS),
            current_time: 0,
            limit: 0,
            burst_messages: Vec::new(),
            next_burst_message: 0,
            current_burst_id: 0,
            user_info,
        }
    }

    fn set_limit(&mut self, limit: u64) {
        self.limit = limit;
    }
}

impl SimpleHiddenServiceModel<'_> {
    fn has_pending_burst_messages(&self) -> bool {
        self.next_burst_message < self.burst_messages.len()
    }

    fn start_next_burst(&mut self) {
        self.current_time += self.interval_die.sample(&mut self.rng);
        self.current_burst_id += 1;
        self.next_burst_message = 0;
        self.burst_messages.clear();

        let message_count = self.message_count_die.sample(&mut self.rng) as usize;
        self.burst_messages.reserve(message_count);
        for _ in 0..message_count {
            let offset = self.burst_offset_die.sample(&mut self.rng);
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

impl<'a> RequestHandler for SimpleHiddenServiceModel<'a> {
    type Out = RouteEvent<'a>;

    #[inline]
    fn fetch_next(&mut self) -> Option<Self::Out> {
        let next_timing = self.next_message_timing();
        match next_timing {
            currt if currt < self.limit => {
                self.user_info.update(currt, &mut self.rng);
                Some((
                    currt,
                    self.user_info.get_selected_vanguard(),
                    self.user_info.get_selected_guard(),
                    Some(self.current_burst_id),
                ))
            }
            _ => None,
        }
    }
}

