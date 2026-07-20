//! A simple user model -- It samples messages within a [5, 15min] interval.
//!
//! Currently does not send the message to any simulated user in particular, and it is one message
//! at a time.
use crate::usermodel::{RequestHandler, RouteEvent, UserModel, UserModelInfo};
use rand::distributions::{Distribution, Uniform};
use rand::rngs::SmallRng;
use rand::SeedableRng;

const INTERVAL_MAX: u64 = 900;
const INTERVAL_MIN: u64 = 300;

pub struct SimpleSynchronousModel<'a> {
    /// timestamp of current time, starting at 0.
    current_time: u64,
    /// the max value of a timing message
    limit: u64,
    rng: SmallRng,
    die: Uniform<u64>,
    user_info: UserModelInfo<'a>,
}

/// This simple model uniformly samples a new message to send in the next [300 ... 900] second
/// interval.
impl<'a> UserModel<'a> for SimpleSynchronousModel<'a> {
    fn new(_tot_users: u32, _epoch: u32, user_info: UserModelInfo<'a>) -> Self {
        SimpleSynchronousModel {
            rng: SmallRng::from_entropy(),
            die: Uniform::from(INTERVAL_MIN..INTERVAL_MAX),
            current_time: 0,
            limit: 0,
            user_info,
        }
    }

    fn set_limit(&mut self, limit: u64) {
        self.limit = limit;
    }
}

impl SimpleSynchronousModel<'_> {
    /// We simply increase the current time with the sampled value.
    fn get_next_message_timing(&mut self) -> u64 {
        self.current_time += self.die.sample(&mut self.rng);
        self.current_time
    }
}

impl<'a> RequestHandler for SimpleSynchronousModel<'a> {
    type Out = RouteEvent<'a>;

    #[inline]
    fn fetch_next(&mut self) -> Option<Self::Out> {
        let next_timing = self.get_next_message_timing();
        match next_timing {
            currt if currt < self.limit => {
                self.user_info.update(currt, &mut self.rng);
                Some((
                    currt,
                    self.user_info.get_selected_vanguard(),
                    self.user_info.get_selected_guard(),
                    None,
                ))
            }
            _ => None,
        }
    }
}
