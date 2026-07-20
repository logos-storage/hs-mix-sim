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
    pub fn record_message(&mut self, message_timing: u64, is_malicious: bool) {
        self.total_messages += 1;

        // Each user's simulation stops at its first compromise. 
        if is_malicious && self.first_compromises.is_empty() {
            let message_index = self.total_messages;
            self.users_with_compromised_messages = 1;
            self.first_compromise_timestamp = Some(message_timing);
            self.first_compromise_message_index = Some(message_index);
            self.first_compromises.push(UserFirstCompromise {
                timestamp: message_timing,
                message_index,
            });
        }
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
        println!("total_messages={}", self.total_messages);
        println!(
            "users_with_compromised_messages={}",
            self.users_with_compromised_messages
        );
        println!(
            "users_without_compromised_messages={}",
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
            "fewest_messages_to_first_compromise={}",
            optional_u64(self.first_compromise_message_index)
        );
    }

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
            "curve,x,day,users_with_compromise,total_users,cumulative_probability,cumulative_percentage"
        )?;

        let duration = u64::from(config_summary.days) * 24 * 60 * 60;
        let interval = u64::from(config_summary.epoch_seconds.max(1));
        let mut timestamp = 0;
        let mut compromised = 0usize;

        loop {
            while compromised < self.first_compromises.len()
                && self.first_compromises[compromised].timestamp <= timestamp
            {
                compromised += 1;
            }

            writeln!(
                writer,
                "time_seconds,{},{:.6},{},{},{:.8},{:.6}",
                timestamp,
                timestamp as f64 / 86_400.0,
                compromised,
                self.users,
                probability(compromised as u64, u64::from(self.users)),
                percentage(compromised as u64, u64::from(self.users))
            )?;

            if timestamp >= duration {
                break;
            }
            timestamp = timestamp.saturating_add(interval).min(duration);
        }

        let mut message_indices: Vec<u64> = self
            .first_compromises
            .iter()
            .map(|compromise| compromise.message_index)
            .collect();
        message_indices.sort_unstable();

        writeln!(
            writer,
            "message_count,0,,0,{},{:.8},{:.6}",
            self.users,
            probability(0, u64::from(self.users)),
            percentage(0, u64::from(self.users))
        )?;

        let mut compromised = 0usize;
        while compromised < message_indices.len() {
            let message_index = message_indices[compromised];
            while compromised < message_indices.len()
                && message_indices[compromised] <= message_index
            {
                compromised += 1;
            }

            writeln!(
                writer,
                "message_count,{},,{},{},{:.8},{:.6}",
                message_index,
                compromised,
                self.users,
                probability(compromised as u64, u64::from(self.users)),
                percentage(compromised as u64, u64::from(self.users))
            )?;
        }

        Ok(())
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
