use crate::config::TopologyConfig;
use crate::config::PATH_LENGTH;
use crate::config::Mixnode;
use crate::usermodel::{RequestHandler, RouteEvent, UserModel, UserModelInfo, UserModelIterator};
use chrono::{TimeZone, Utc};
use rand::prelude::*;
use rayon::prelude::*;
use std::vec::IntoIter;

/// Contains information required for running the simulation
#[derive(Default)]
pub struct Runable {
    /// The number of users we want to simulate
    users: u32,
    /// The Network config
    configs: Vec<TopologyConfig>,
    /// The number of virtual days for running the experiment
    days: u32,
    /// Does this simulation use persistent guard positions?
    use_guards: bool,
    /// Does this simulation use persistent vanguard positions?
    use_vanguards: bool,
    /// each topology lifetime -- we assume this to be unique (e.g., 1 day)
    epoch: u32,
    /// print human-readable per-route output --- default: false
    to_console: bool,
    /// print aggregate simulation summary at the end --- default: false
    summary: bool,
}

#[derive(Clone, Default)]
pub struct SimulationSummary {
    users: u32,
    total_messages: u64,
    compromised_messages: u64,
    users_with_compromise: u64,
    first_compromise_timestamp: Option<u64>,
    last_message_timestamp: Option<u64>,
}

impl SimulationSummary {
    fn for_user() -> Self {
        SimulationSummary {
            users: 1,
            ..Default::default()
        }
    }

    fn merge(mut self, other: Self) -> Self {
        self.users += other.users;
        self.total_messages += other.total_messages;
        self.compromised_messages += other.compromised_messages;
        self.users_with_compromise += other.users_with_compromise;
        self.first_compromise_timestamp = min_option(
            self.first_compromise_timestamp,
            other.first_compromise_timestamp,
        );
        self.last_message_timestamp =
            max_option(self.last_message_timestamp, other.last_message_timestamp);
        self
    }
}

fn min_option(a: Option<u64>, b: Option<u64>) -> Option<u64> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

fn max_option(a: Option<u64>, b: Option<u64>) -> Option<u64> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

impl Runable {
    /// Creates a new simulation to run.
    pub fn new(users: u32, configs: Vec<TopologyConfig>, days: u32, epoch: u32) -> Self {
        Runable {
            configs,
            users,
            days,
            epoch,
            ..Default::default()
        }
    }
    /// Do we enable persistent guards for this simulation?
    pub fn with_guards(&mut self) -> &mut Self {
        self.use_guards = true;
        self
    }
    /// Do we enable persistent vanguards for this simulation?
    pub fn with_vanguards(&mut self) -> &mut Self {
        self.use_vanguards = true;
        self
    }
    /// Do we print human-readable route results?
    pub fn with_console(&mut self) -> &mut Self {
        self.to_console = true;
        self
    }
    /// Do we print an aggregate summary at the end?
    pub fn with_summary(&mut self) -> &mut Self {
        self.summary = true;
        self
    }
    /// Get a random path from the right mixnet configuration.
    ///
    /// A vanguard and guard are optionally given. They are persistent mixnet nodes chosen to
    /// change as little as possible.
    #[inline]
    pub fn sample_path<'a>(
        &'a self,
        message_timing: u64,
        rng: &mut ThreadRng,
        vanguard: Option<&'a Mixnode>,
        guard: Option<&'a Mixnode>,
    ) -> IntoIter<&'a Mixnode> {
        self.configs[(message_timing / self.epoch as u64) as usize]
            .sample_path(rng, vanguard, guard)
    }

    /// Check whether the three mixnode in path are compromised.
    /// Returns true if they are, false otherwise.
    pub fn is_path_malicious(&self, path: &[&Mixnode]) -> bool {
        let mut mal_mix = 0;
        for i in 0..PATH_LENGTH {
            if path[i as usize].is_malicious {
                mal_mix += 1;
            }
        }
        mal_mix == PATH_LENGTH
    }

    /// Format the message's sending time as a naive time
    #[inline]
    fn format_message_timing(timing: u64) -> String {
        let dt = Utc
            .timestamp_opt(timing as i64, 0)
            .single()
            .expect("message timing should be a valid timestamp");
        dt.format("%Y-%m-%d %H:%M:%S").to_string()
    }

    #[inline]
    pub fn days_to_timestamp(&self) -> u64 {
        u64::from(self.days) * 24 * 60 * 60
    }

    #[inline]
    fn log_stdout(
        &self,
        user: u32,
        strdate: &str,
        path: IntoIter<&Mixnode>,
        is_malicious: bool,
        requestid: Option<u128>,
    ) {
        if !self.to_console {
            return;
        }

        let path_ids: Vec<String> = path.map(|hop| hop.mixid.to_string()).collect();
        let path = path_ids.join(",");

        if let Some(rid) = requestid {
            println!("{strdate} {user} {rid} {path} {is_malicious}");
        } else {
            println!("{strdate} {user} {path} {is_malicious}");
        }
    }

    #[inline]
    fn log_summary(
        &self,
        summary: &mut SimulationSummary,
        message_timing: u64,
        is_malicious: bool,
    ) {
        summary.total_messages += 1;
        summary.last_message_timestamp = Some(
            summary
                .last_message_timestamp
                .map_or(message_timing, |last| last.max(message_timing)),
        );

        if is_malicious {
            summary.compromised_messages += 1;
            if summary.first_compromise_timestamp.is_none() {
                summary.users_with_compromise = 1;
                summary.first_compromise_timestamp = Some(message_timing);
            }
        }
    }

    fn print_summary(&self, summary: &SimulationSummary) {
        println!("route_summary");
        println!("users={}", summary.users);
        println!("days={}", self.days);
        println!("epoch_seconds={}", self.epoch);
        println!("topologies_loaded={}", self.configs.len());
        println!("guards_enabled={}", self.use_guards);
        println!("vanguards_enabled={}", self.use_vanguards);
        println!("total_messages={}", summary.total_messages);
        println!("compromised_messages={}", summary.compromised_messages);
        println!(
            "percentage_messages_compromised={:.6}",
            percentage(summary.compromised_messages, summary.total_messages)
        );
        println!(
            "users_with_at_least_one_compromised_message={}",
            summary.users_with_compromise
        );
        println!(
            "users_without_compromised_message={}",
            summary
                .users
                .saturating_sub(summary.users_with_compromise as u32)
        );
        println!(
            "percentage_users_with_at_least_one_compromised_message={:.6}",
            percentage(summary.users_with_compromise, u64::from(summary.users))
        );
        println!(
            "first_compromise_timestamp_seconds={}",
            optional_u64(summary.first_compromise_timestamp)
        );
        println!(
            "last_message_timestamp_seconds={}",
            optional_u64(summary.last_message_timestamp)
        );
    }

    /// Initialize user models
    pub fn init<'a, T>(&'a self) -> Vec<UserModelIterator<T>>
    where
        T: UserModel<'a>,
    {
        (0..self.users)
            .map(|user| {
                let model = T::new(
                    self.users,
                    self.epoch,
                    UserModelInfo::new(
                        user,
                        &self.configs,
                        self.epoch,
                        self.use_guards,
                        self.use_vanguards,
                    ),
                );
                UserModelIterator(model)
            })
            .collect()
    }

    /// Run the simulation -- this function should output
    /// route taken for each user each time the user requires to send
    /// a message, which depends of the user model through time.
    pub fn run<'a, T>(&'a self, mut usermodels: Vec<UserModelIterator<T>>)
    where
        T: UserModel<'a> + Send,
        T: RequestHandler<Out = RouteEvent<'a>>,
    {
        // for_each should block until they all completed
        let summary = (0..self.users)
            .into_par_iter()
            .zip(&mut usermodels)
            .map(|(user, mut usermodel)| {
                let mut rng = thread_rng();
                let mut user_summary = SimulationSummary::for_user();
                // move this in the init part?
                usermodel.set_limit(self.days_to_timestamp());
                //let userinfo = &mut userinfos[user as usize];
                for (message_timing, vanguard, guard, requestid) in &mut usermodel {
                    // do we need to update userinfo relative to the current timing?
                    let path = self.sample_path(message_timing, &mut rng, vanguard, guard);
                    let strdate = Runable::format_message_timing(message_timing);
                    // write out the path for this message_timing
                    let is_malicious = self.is_path_malicious(path.as_slice());
                    self.log_stdout(user, &strdate, path, is_malicious, requestid);
                    self.log_summary(&mut user_summary, message_timing, is_malicious);
                }
                user_summary
            })
            .reduce(SimulationSummary::default, SimulationSummary::merge);

        if self.summary {
            self.print_summary(&summary);
        }
    }
}

fn percentage(numerator: u64, denominator: u64) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64 * 100.0
    }
}

fn optional_u64(value: Option<u64>) -> String {
    value.map_or_else(|| "none".to_string(), |value| value.to_string())
}

#[test]
fn test_date_formatting() {
    let mut timing = 60 * 11;
    let mut strdate = Runable::format_message_timing(timing);
    assert_eq!(strdate, "1970-01-01 00:11:00");
    timing = timing + 1;
    strdate = Runable::format_message_timing(timing);
    assert_eq!(strdate, "1970-01-01 00:11:01");
    timing = timing + 25 * 60 * 60;
    strdate = Runable::format_message_timing(timing);
    assert_eq!(strdate, "1970-01-02 01:11:01");
}

#[test]
fn test_summary_counts_user_once() {
    let runner = Runable::default();
    let mut summary = SimulationSummary::for_user();

    runner.log_summary(&mut summary, 10, false);
    runner.log_summary(&mut summary, 20, true);
    runner.log_summary(&mut summary, 30, true);

    assert_eq!(summary.users, 1);
    assert_eq!(summary.total_messages, 3);
    assert_eq!(summary.compromised_messages, 2);
    assert_eq!(summary.users_with_compromise, 1);
    assert_eq!(summary.first_compromise_timestamp, Some(20));
    assert_eq!(summary.last_message_timestamp, Some(30));
}
