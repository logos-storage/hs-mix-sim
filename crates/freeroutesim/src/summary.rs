//! Summary and output for simulation results.
//!
//! This module only records the facts the simulation passes in and turns them into final
//! console summaries, or CSVs.

use std::fs::{File, create_dir_all};
use std::io::{BufWriter, Write};
use std::path::Path;

#[derive(Clone, Default)]
struct UserFirstCompromise {
    timestamp: u64,
    message_index: u64,
    node_compromises_before_win: Option<u64>,
}

#[derive(Default)]
struct NodeCompromiseStats {
    total: u64,
    measured_wins: u64,
}

impl NodeCompromiseStats {
    fn record(&mut self, count: Option<u64>) {
        if let Some(count) = count {
            self.total += count;
            self.measured_wins += 1;
        }
    }

    fn mean(&self) -> Option<f64> {
        (self.measured_wins > 0).then(|| self.total as f64 / self.measured_wins as f64)
    }
}

#[derive(Clone, Default)]
pub struct SimulationSummary {
    users: u32,
    total_messages: u64,
    users_with_compromised_messages: u64,
    first_compromise_timestamp: Option<u64>,
    first_compromise_message_index: Option<u64>,
    first_compromises: Vec<UserFirstCompromise>,
}

pub struct SimulationConfigSummary {
    pub days: u32,
    pub epoch_seconds: u32,
    pub topologies_loaded: usize,
    pub path_hops: usize,
    pub path_sampler_type: &'static str,
    pub user_model_type: &'static str,
    pub malicious_node_fraction: f64,
    pub malicious_bandwidth_fraction: f64,
    pub churn_rate: f64,
    pub sdlm: Option<SdlmSummary>,
}

#[derive(Clone, Copy, Debug)]
pub enum SdlmStrategy {
    Random,
    KHopsFixed { fixed_hops: usize },
    KOverW { k: usize },
    AlphaSticky { alpha: f64 },
}

#[derive(Clone, Copy, Debug)]
pub struct SdlmSummary {
    strategy: SdlmStrategy,
    session_paths: u64,
    path_hops: usize,
    active_nodes: usize,
    malicious_nodes: usize,
}

impl SdlmSummary {
    pub fn new(
        strategy: SdlmStrategy,
        session_paths: u64,
        path_hops: usize,
        active_nodes: usize,
        malicious_nodes: usize,
    ) -> Self {
        assert!(session_paths > 0, "S-DLM needs at least one session path");
        assert!(path_hops > 0, "S-DLM needs at least one path hop");
        assert!(active_nodes > 0, "S-DLM needs at least one active node");
        assert!(
            malicious_nodes <= active_nodes,
            "malicious nodes cannot exceed active nodes"
        );

        Self {
            strategy,
            session_paths,
            path_hops,
            active_nodes,
            malicious_nodes,
        }
    }

    fn strategy_name(self) -> &'static str {
        match self.strategy {
            SdlmStrategy::Random => "random",
            SdlmStrategy::KHopsFixed { .. } => "k-hf",
            SdlmStrategy::KOverW { .. } => "k-w",
            SdlmStrategy::AlphaSticky { .. } => "alpha-ss",
        }
    }

    fn beta(self) -> f64 {
        self.malicious_nodes as f64 / self.active_nodes as f64
    }

    fn approximate_probability(self) -> f64 {
        let beta = self.beta();
        let one_path_probability = pow_usize(beta, self.path_hops);

        let probability = match self.strategy {
            SdlmStrategy::Random => {
                probability_at_least_once(one_path_probability, self.session_paths)
            }
            SdlmStrategy::KHopsFixed { fixed_hops } => {
                assert!(
                    fixed_hops <= self.path_hops,
                    "fixed hops cannot exceed path hops"
                );
                let fixed_probability = pow_usize(beta, fixed_hops);
                let changing_probability = pow_usize(beta, self.path_hops - fixed_hops);
                fixed_probability
                    * probability_at_least_once(changing_probability, self.session_paths)
            }
            SdlmStrategy::KOverW { k } => {
                assert!(k > 0, "K/W needs at least one candidate per hop");
                let distinct_paths = capped_power(k, self.path_hops, self.session_paths);
                probability_at_least_once(one_path_probability, distinct_paths)
            }
            SdlmStrategy::AlphaSticky { alpha } => {
                assert!(
                    (0.0..=1.0).contains(&alpha),
                    "alpha must be between zero and one"
                );
                if one_path_probability == 1.0 {
                    1.0
                } else if self.session_paths == 1 {
                    one_path_probability
                } else {
                    let later_path_probability = (1.0 - alpha) * one_path_probability;
                    let log_no_compromise = (-one_path_probability).ln_1p()
                        + (self.session_paths - 1) as f64 * (-later_path_probability).ln_1p();
                    -log_no_compromise.exp_m1()
                }
            }
        };

        probability.clamp(0.0, 1.0)
    }
}

impl SimulationSummary {
    /// Create the local summary used while simulating one user.
    pub fn for_user() -> Self {
        Self {
            users: 1,
            ..Default::default()
        }
    }

    #[inline]
    pub fn record_message(&mut self, message_timing: u64, adversary_won: bool) {
        self.total_messages += 1;

        // Each user's simulation stops at its first compromise.
        if adversary_won && self.first_compromises.is_empty() {
            let message_index = self.total_messages;
            self.users_with_compromised_messages = 1;
            self.first_compromise_timestamp = Some(message_timing);
            self.first_compromise_message_index = Some(message_index);
            self.first_compromises.push(UserFirstCompromise {
                timestamp: message_timing,
                message_index,
                node_compromises_before_win: None,
            });
        }
    }

    /// Attach the completed-node count to this user's recorded win. A count
    /// without a recorded win (including wins beyond the deadline) is ignored.
    pub fn record_compromises_before_win(&mut self, count: u64) {
        if let Some(compromise) = self.first_compromises.first_mut() {
            compromise.node_compromises_before_win = Some(count);
        }
    }

    fn node_compromise_stats(&self) -> NodeCompromiseStats {
        let mut stats = NodeCompromiseStats::default();
        for compromise in &self.first_compromises {
            stats.record(compromise.node_compromises_before_win);
        }
        stats
    }

    pub fn merge(mut self, other: Self) -> Self {
        self.users += other.users;
        self.total_messages += other.total_messages;
        self.users_with_compromised_messages += other.users_with_compromised_messages;
        self.first_compromise_timestamp = min_option(
            self.first_compromise_timestamp,
            other.first_compromise_timestamp,
        );
        self.first_compromise_message_index = min_option(
            self.first_compromise_message_index,
            other.first_compromise_message_index,
        );
        self.first_compromises.extend(other.first_compromises);
        self
    }

    pub fn print_summary(&self, config_summary: &SimulationConfigSummary) {
        println!("simulation_summary");
        println!("users={}", self.users);
        println!("days={}", config_summary.days);
        println!("epoch_seconds={}", config_summary.epoch_seconds);
        println!("topologies_loaded={}", config_summary.topologies_loaded);
        println!("path_hops={}", config_summary.path_hops);
        println!("path_sampler_type={}", config_summary.path_sampler_type);
        println!("user_model_type={}", config_summary.user_model_type);
        println!(
            "malicious_node_fraction={:.6}",
            config_summary.malicious_node_fraction
        );
        println!(
            "malicious_bandwidth_fraction={:.6}",
            config_summary.malicious_bandwidth_fraction
        );
        println!("churn_rate={:.6}", config_summary.churn_rate);
        if let Some(sdlm) = config_summary.sdlm {
            println!("session_paths={}", sdlm.session_paths);
            println!("formula_active_nodes={}", sdlm.active_nodes);
            println!("formula_malicious_nodes={}", sdlm.malicious_nodes);
            println!("formula_beta={:.8}", sdlm.beta());
            println!("s_dlm_strategy={}", sdlm.strategy_name());
            match sdlm.strategy {
                SdlmStrategy::Random => {}
                SdlmStrategy::KHopsFixed { fixed_hops } => {
                    println!("s_dlm_fixed_hops={fixed_hops}");
                }
                SdlmStrategy::KOverW { k } => println!("s_dlm_k={k}"),
                SdlmStrategy::AlphaSticky { alpha } => println!("s_dlm_alpha={alpha:.8}"),
            }
            let approximate_s_dlm = sdlm.approximate_probability();
            println!("approximate_s_dlm_probability={:.10}", approximate_s_dlm);
            println!(
                "approximate_s_dlm_percentage={:.8}",
                approximate_s_dlm * 100.0
            );
        }
        let hidden_service = config_summary.user_model_type == "HiddenServiceModel";
        let (total_label, winners_label, nonwinners_label, count_label) = if hidden_service {
            (
                "processed_checks",
                "users_with_identified_services",
                "users_without_identified_services",
                "fewest_checks_to_service_identification",
            )
        } else {
            (
                "total_messages",
                "users_with_compromised_messages",
                "users_without_compromised_messages",
                "fewest_messages_to_first_compromise",
            )
        };
        println!("{total_label}={}", self.total_messages);
        println!("{winners_label}={}", self.users_with_compromised_messages);
        println!(
            "{nonwinners_label}={}",
            u64::from(self.users).saturating_sub(self.users_with_compromised_messages)
        );
        println!(
            "percentage_users_compromised={:.6}",
            percentage(self.users_with_compromised_messages, u64::from(self.users))
        );
        println!(
            "first_compromise_timestamp_seconds={}",
            optional_u64(self.first_compromise_timestamp)
        );
        println!(
            "{count_label}={}",
            optional_u64(self.first_compromise_message_index)
        );
        let stats = self.node_compromise_stats();
        if hidden_service || stats.measured_wins > 0 {
            println!("total_node_compromises_before_win={}", stats.total);
            println!("wins_with_compromise_counts={}", stats.measured_wins);
            println!(
                "mean_node_compromises_before_win={}",
                stats
                    .mean()
                    .map_or_else(|| "none".to_owned(), |mean| format!("{mean:.6}"))
            );
        }
    }

    /// Write cumulative win curves. The existing `message_count` curve counts
    /// processed checks for hidden-service models. Node-compromise columns
    /// include only winners with a recorded count; missing means are empty.
    pub fn write_timeseries_csv(
        &mut self,
        config_summary: &SimulationConfigSummary,
        csv_file_path: &Path,
    ) -> std::io::Result<()> {
        if let Some(parent) = csv_file_path.parent()
            && !parent.as_os_str().is_empty()
        {
            create_dir_all(parent)?;
        }

        self.first_compromises
            .sort_unstable_by_key(|compromise| compromise.timestamp);

        let file = File::create(csv_file_path)?;
        let mut writer = BufWriter::new(file);
        writeln!(
            writer,
            "curve,x,day,users_with_compromise,total_users,cumulative_probability,cumulative_percentage,cumulative_node_compromises_before_win,wins_with_compromise_counts,mean_node_compromises_before_win"
        )?;

        let duration = u64::from(config_summary.days) * 24 * 60 * 60;
        let interval = u64::from(config_summary.epoch_seconds.max(1));
        let mut timestamp = 0;
        let mut compromised = 0usize;
        let mut node_stats = NodeCompromiseStats::default();

        loop {
            while compromised < self.first_compromises.len()
                && self.first_compromises[compromised].timestamp <= timestamp
            {
                node_stats.record(self.first_compromises[compromised].node_compromises_before_win);
                compromised += 1;
            }

            writeln!(
                writer,
                "time_seconds,{},{:.6},{},{},{:.8},{:.6},{},{},{}",
                timestamp,
                timestamp as f64 / 86_400.0,
                compromised,
                self.users,
                probability(compromised as u64, u64::from(self.users)),
                percentage(compromised as u64, u64::from(self.users)),
                node_stats.total,
                node_stats.measured_wins,
                node_stats
                    .mean()
                    .map(|mean| format!("{mean:.6}"))
                    .unwrap_or_default()
            )?;

            if timestamp >= duration {
                break;
            }
            timestamp = timestamp.saturating_add(interval).min(duration);
        }

        let mut message_compromises: Vec<&UserFirstCompromise> =
            self.first_compromises.iter().collect();
        message_compromises.sort_unstable_by_key(|compromise| compromise.message_index);

        writeln!(
            writer,
            "message_count,0,,0,{},{:.8},{:.6},0,0,",
            self.users,
            probability(0, u64::from(self.users)),
            percentage(0, u64::from(self.users))
        )?;

        let mut compromised = 0usize;
        let mut node_stats = NodeCompromiseStats::default();
        while compromised < message_compromises.len() {
            let message_index = message_compromises[compromised].message_index;
            while compromised < message_compromises.len()
                && message_compromises[compromised].message_index <= message_index
            {
                node_stats.record(message_compromises[compromised].node_compromises_before_win);
                compromised += 1;
            }

            writeln!(
                writer,
                "message_count,{},,{},{},{:.8},{:.6},{},{},{}",
                message_index,
                compromised,
                self.users,
                probability(compromised as u64, u64::from(self.users)),
                percentage(compromised as u64, u64::from(self.users)),
                node_stats.total,
                node_stats.measured_wins,
                node_stats
                    .mean()
                    .map(|mean| format!("{mean:.6}"))
                    .unwrap_or_default()
            )?;
        }

        writer.flush()
    }
}

fn min_option(left: Option<u64>, right: Option<u64>) -> Option<u64> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.min(right)),
        (Some(value), None) | (None, Some(value)) => Some(value),
        (None, None) => None,
    }
}

fn percentage(numerator: u64, denominator: u64) -> f64 {
    probability(numerator, denominator) * 100.0
}

fn probability(numerator: u64, denominator: u64) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

fn optional_u64(value: Option<u64>) -> String {
    value.map_or_else(|| "none".to_owned(), |value| value.to_string())
}

fn probability_at_least_once(single_probability: f64, trials: u64) -> f64 {
    if trials == 0 || single_probability == 0.0 {
        return 0.0;
    }

    -((trials as f64) * (-single_probability).ln_1p()).exp_m1()
}

fn pow_usize(mut base: f64, mut exponent: usize) -> f64 {
    let mut result = 1.0;
    while exponent > 0 {
        if exponent % 2 == 1 {
            result *= base;
        }
        base *= base;
        exponent /= 2;
    }
    result
}

fn capped_power(base: usize, exponent: usize, cap: u64) -> u64 {
    if cap == 0 {
        return 0;
    }

    let base = u64::try_from(base).unwrap_or(cap);
    let mut result = 1_u64;
    for _ in 0..exponent {
        result = match result.checked_mul(base) {
            Some(value) if value < cap => value,
            _ => return cap,
        };
    }
    result.min(cap)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn winning_user(timestamp: u64, checks: u64, node_count: Option<u64>) -> SimulationSummary {
        let mut summary = SimulationSummary::for_user();
        for _ in 1..checks {
            summary.record_message(0, false);
        }
        summary.record_message(timestamp, true);
        if let Some(count) = node_count {
            summary.record_compromises_before_win(count);
        }
        summary
    }

    fn config(days: u32, epoch_seconds: u32) -> SimulationConfigSummary {
        SimulationConfigSummary {
            days,
            epoch_seconds,
            topologies_loaded: 1,
            path_hops: 3,
            path_sampler_type: "FixedPathSampler",
            user_model_type: "HiddenServiceModel",
            malicious_node_fraction: 0.0,
            malicious_bandwidth_fraction: 0.0,
            churn_rate: 0.0,
            sdlm: None,
        }
    }

    fn csv_rows(
        summary: &mut SimulationSummary,
        config: &SimulationConfigSummary,
    ) -> Vec<Vec<String>> {
        static NEXT_FILE: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "freeroutesim-summary-{}-{}.csv",
            std::process::id(),
            NEXT_FILE.fetch_add(1, Ordering::Relaxed)
        ));
        summary.write_timeseries_csv(config, &path).unwrap();
        let csv = std::fs::read_to_string(&path).unwrap();
        std::fs::remove_file(path).unwrap();
        csv.lines()
            .map(|line| line.split(',').map(str::to_owned).collect())
            .collect()
    }

    #[test]
    fn node_compromise_mean_includes_zero_and_excludes_unknown_and_nonwinners() {
        let mut nonwinner = SimulationSummary::for_user();
        nonwinner.record_message(100, false);
        nonwinner.record_compromises_before_win(999);

        let summary = SimulationSummary::default()
            .merge(winning_user(0, 1, Some(0)))
            .merge(winning_user(50, 2, Some(4)))
            .merge(winning_user(100, 3, None))
            .merge(nonwinner);

        assert_eq!(summary.users, 4);
        assert_eq!(summary.users_with_compromised_messages, 3);
        assert_eq!(summary.first_compromises.len(), 3);
        assert_eq!(summary.total_messages, 7);
        let stats = summary.node_compromise_stats();
        assert_eq!(stats.total, 4);
        assert_eq!(stats.measured_wins, 2);
        assert_eq!(stats.mean(), Some(2.0));
    }

    #[test]
    fn csv_accumulates_counts_at_inclusive_time_and_check_boundaries() {
        let mut summary = winning_user(86_400, 3, Some(2))
            .merge(winning_user(40_000, 2, None))
            .merge(winning_user(0, 1, Some(0)))
            .merge(winning_user(40_000, 2, Some(4)))
            .merge(SimulationSummary::for_user());
        let rows = csv_rows(&mut summary, &config(1, 40_000));

        assert_eq!(rows.len(), 9);
        assert!(rows.iter().all(|row| row.len() == 10));
        assert_eq!(
            &rows[0][7..],
            [
                "cumulative_node_compromises_before_win",
                "wins_with_compromise_counts",
                "mean_node_compromises_before_win"
            ]
        );
        // An all-Sybil win at time zero has a measured count of zero.
        assert_eq!(&rows[1][..5], ["time_seconds", "0", "0.000000", "1", "5"]);
        assert_eq!(&rows[1][7..], ["0", "1", "0.000000"]);
        // Tied wins are accumulated together; an unknown count adds no denominator.
        assert_eq!(&rows[2][1..4], ["40000", "0.462963", "3"]);
        assert_eq!(&rows[2][7..], ["4", "2", "2.000000"]);
        assert_eq!(&rows[3][1..4], ["80000", "0.925926", "3"]);
        assert_eq!(&rows[3][7..], ["4", "2", "2.000000"]);
        // Include the deadline even when it is not a multiple of the interval.
        assert_eq!(&rows[4][1..4], ["86400", "1.000000", "4"]);
        assert_eq!(&rows[4][7..], ["6", "3", "2.000000"]);

        assert_eq!(&rows[5][..5], ["message_count", "0", "", "0", "5"]);
        assert_eq!(&rows[5][7..], ["0", "0", ""]);
        assert_eq!(&rows[6][1..4], ["1", "", "1"]);
        assert_eq!(&rows[6][7..], ["0", "1", "0.000000"]);
        assert_eq!(&rows[7][1..4], ["2", "", "3"]);
        assert_eq!(&rows[7][7..], ["4", "2", "2.000000"]);
        assert_eq!(&rows[8][1..4], ["3", "", "4"]);
        assert_eq!(&rows[8][7..], ["6", "3", "2.000000"]);
    }

    #[test]
    fn csv_leaves_unmeasured_mean_empty_with_zero_duration_and_interval() {
        let mut summary = winning_user(0, 1, None);
        assert_eq!(summary.node_compromise_stats().mean(), None);
        let rows = csv_rows(&mut summary, &config(0, 0));
        assert_eq!(rows.len(), 4);
        assert_eq!(&rows[1][..5], ["time_seconds", "0", "0.000000", "1", "1"]);
        assert!(rows[1..].iter().all(|row| row[7..] == ["0", "0", ""]));
    }
}
