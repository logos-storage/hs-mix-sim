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
    max_messages_per_user: u64,
    first_compromises: Vec<UserFirstCompromise>,
}

pub struct SimulationConfigSummary {
    pub days: u32,
    pub csv_interval_seconds: u32,
    pub mix_nodes: usize,
    pub malicious_nodes: usize,
    pub path_hops: usize,
    pub path_sampler_type: &'static str,
    pub user_model_type: &'static str,
    pub adversary_type: &'static str,
    pub malicious_node_fraction: f64,
    pub sdlm: Option<SdlmSummary>,
    /// Only parameters relevant to the selected model and sampler.
    pub parameters: Vec<(&'static str, String)>,
    pub download_size_bytes: Option<u64>,
}

#[derive(Clone, Copy, Debug)]
pub enum SdlmStrategy {
    Random,
    KHopsFixed { fixed_hops: usize },
    KOverW { k: usize, fixed_hops: usize },
    AlphaSticky { alpha: f64 },
}

#[derive(Clone, Copy, Debug)]
pub struct SdlmSummary {
    strategy: SdlmStrategy,
    session_paths: u64,
    path_hops: usize,
    total_nodes: usize,
    malicious_nodes: usize,
}

impl SdlmSummary {
    pub fn new(
        strategy: SdlmStrategy,
        session_paths: u64,
        path_hops: usize,
        total_nodes: usize,
        malicious_nodes: usize,
    ) -> Self {
        assert!(session_paths > 0, "S-DLM needs at least one session path");
        assert!(path_hops > 0, "S-DLM needs at least one path hop");
        assert!(total_nodes > 0, "S-DLM needs at least one mix node");
        assert!(
            malicious_nodes <= total_nodes,
            "malicious nodes cannot exceed total nodes"
        );

        Self {
            strategy,
            session_paths,
            path_hops,
            total_nodes,
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
        self.malicious_nodes as f64 / self.total_nodes as f64
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
            SdlmStrategy::KOverW { k, fixed_hops } => {
                assert!(k > 0 && fixed_hops <= self.path_hops);
                assert!(
                    fixed_hops
                        .checked_mul(k)
                        .is_some_and(|n| n <= self.total_nodes)
                );
                // Spec approximation formula
                let fixed_probability = pow_usize(1.0 - pow_usize(1.0 - beta, k), fixed_hops);
                let random_probability = pow_usize(beta, self.path_hops - fixed_hops);
                fixed_probability
                    * probability_at_least_once(random_probability, self.session_paths)
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
        self.max_messages_per_user = self.total_messages;

        // Each user's simulation stops at its first compromise.
        if adversary_won && self.first_compromises.is_empty() {
            let message_index = self.total_messages;
            self.users_with_compromised_messages = 1;
            self.first_compromise_timestamp = Some(message_timing);

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
        self.max_messages_per_user = self.max_messages_per_user.max(other.max_messages_per_user);
        self.first_compromises.extend(other.first_compromises);
        self
    }

    pub fn print_summary(&self, config_summary: &SimulationConfigSummary) {
        println!("simulation_summary");
        println!("users={}", self.users);
        if config_summary.sdlm.is_none() {
            println!("days={}", config_summary.days);
        }
        println!("mix_nodes={}", config_summary.mix_nodes);
        println!("path_hops={}", config_summary.path_hops);
        println!("path_sampler_type={}", config_summary.path_sampler_type);
        println!("user_model_type={}", config_summary.user_model_type);
        println!("adversary_type={}", config_summary.adversary_type);
        println!(
            "malicious_node_fraction={:.6}",
            config_summary.malicious_node_fraction
        );
        println!("malicious_nodes={}", config_summary.malicious_nodes);
        for (name, value) in &config_summary.parameters {
            println!("{name}={value}");
        }
        if let Some(size) = config_summary.download_size_bytes {
            println!("download_size_bytes={size}");
        }
        if let Some(sdlm) = config_summary.sdlm {
            println!("session_paths={}", sdlm.session_paths);
            println!("formula_total_nodes={}", sdlm.total_nodes);
            println!("formula_malicious_nodes={}", sdlm.malicious_nodes);
            println!("formula_beta={:.8}", sdlm.beta());
            println!("s_dlm_strategy={}", sdlm.strategy_name());
            match sdlm.strategy {
                SdlmStrategy::Random => {}
                SdlmStrategy::KHopsFixed { fixed_hops } => {
                    println!("s_dlm_fixed_hops={fixed_hops}");
                }
                SdlmStrategy::KOverW { k, fixed_hops } => {
                    println!("s_dlm_k={k}");
                    println!("s_dlm_fixed_hops={fixed_hops}");
                }
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
        let (winners_label, nonwinners_label) = if hidden_service {
            (
                "users_with_identified_services",
                "users_without_identified_services",
            )
        } else {
            (
                "users_with_compromised_messages",
                "users_without_compromised_messages",
            )
        };
        println!("{winners_label}={}", self.users_with_compromised_messages);
        println!(
            "{nonwinners_label}={}",
            u64::from(self.users).saturating_sub(self.users_with_compromised_messages)
        );
        println!(
            "percentage_users_compromised={:.6}",
            percentage(self.users_with_compromised_messages, u64::from(self.users))
        );
        if config_summary.sdlm.is_none() {
            println!(
                "first_compromise_timestamp_seconds={}",
                optional_u64(self.first_compromise_timestamp)
            );
        }
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

    /// Export model-specific plot data with its configuration on every row.
    pub fn write_csv(
        &mut self,
        config: &SimulationConfigSummary,
        path: &Path,
    ) -> std::io::Result<()> {
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            create_dir_all(parent)?;
        }
        let mut writer = BufWriter::new(File::create(path)?);
        self.write_csv_to(config, &mut writer)?;
        writer.flush()?;
        Ok(())
    }

    fn write_csv_to(
        &mut self,
        config: &SimulationConfigSummary,
        writer: &mut impl Write,
    ) -> std::io::Result<()> {
        let hidden_service = config.user_model_type == "HiddenServiceModel";
        let mut metadata = vec![
            ("model", config.user_model_type.to_owned()),
            ("sampler", config.path_sampler_type.to_owned()),
            ("hops", config.path_hops.to_string()),
            ("adversary", config.adversary_type.to_owned()),
            ("users", self.users.to_string()),
            ("mix_nodes", config.mix_nodes.to_string()),
            ("malicious_nodes", config.malicious_nodes.to_string()),
            (
                "malicious_node_fraction",
                config.malicious_node_fraction.to_string(),
            ),
        ];
        if config.sdlm.is_none() {
            metadata.push((
                "duration_seconds",
                (u64::from(config.days) * 86_400).to_string(),
            ));
        }
        metadata.extend(config.parameters.iter().cloned());
        let data_headers = if config.sdlm.is_some() {
            vec![
                "download_size_bytes",
                "packet_count",
                "compromised_users",
                "simulated_s_dlm",
                "formula_s_dlm",
            ]
        } else {
            let mut headers = vec!["curve", "x", "compromised_users", "cumulative_probability"];
            if hidden_service {
                headers.push("mean_node_compromises_before_win");
            }
            headers
        };
        let mut headers: Vec<String> = metadata
            .iter()
            .map(|(name, _)| (*name).to_owned())
            .collect();
        headers.extend(data_headers.iter().map(|name| (*name).to_owned()));
        write_csv_row(writer, &headers)?;
        let metadata_values: Vec<String> = metadata.into_iter().map(|(_, value)| value).collect();
        let mut row = |data: Vec<String>| {
            let mut values = metadata_values.clone();
            values.extend(data);
            write_csv_row(writer, &values)
        };

        if let Some(sdlm) = config.sdlm {
            return row(vec![
                config
                    .download_size_bytes
                    .expect("download size is required for S-DLM")
                    .to_string(),
                sdlm.session_paths.to_string(),
                self.users_with_compromised_messages.to_string(),
                format!(
                    "{:.10}",
                    probability(self.users_with_compromised_messages, u64::from(self.users))
                ),
                format!("{:.10}", sdlm.approximate_probability()),
            ]);
        }

        // Count each user's first win once, retaining nonwinners in the denominator.
        self.first_compromises
            .sort_unstable_by_key(|win| win.timestamp);
        let duration = u64::from(config.days) * 86_400;
        let interval = u64::from(config.csv_interval_seconds.max(1));
        let mut timestamp = 0;
        let mut compromised = 0;
        let mut node_stats = NodeCompromiseStats::default();
        loop {
            while compromised < self.first_compromises.len()
                && self.first_compromises[compromised].timestamp <= timestamp
            {
                node_stats.record(self.first_compromises[compromised].node_compromises_before_win);
                compromised += 1;
            }
            let mut data = vec![
                "time_seconds".to_owned(),
                timestamp.to_string(),
                compromised.to_string(),
                format!(
                    "{:.10}",
                    probability(compromised as u64, u64::from(self.users))
                ),
            ];
            if hidden_service {
                data.push(
                    node_stats
                        .mean()
                        .map(|mean| format!("{mean:.6}"))
                        .unwrap_or_default(),
                );
            }
            row(data)?;
            if timestamp == duration {
                break;
            }
            timestamp = timestamp.saturating_add(interval).min(duration);
        }
        if hidden_service {
            return Ok(());
        }

        // Hidden-service events are checks, not messages; this curve is simple-only.
        self.first_compromises
            .sort_unstable_by_key(|win| win.message_index);
        row(vec![
            "message_count".to_owned(),
            "0".to_owned(),
            "0".to_owned(),
            "0.0000000000".to_owned(),
        ])?;
        let mut compromised = 0;
        let mut last_message = 0;
        while compromised < self.first_compromises.len() {
            last_message = self.first_compromises[compromised].message_index;
            while compromised < self.first_compromises.len()
                && self.first_compromises[compromised].message_index <= last_message
            {
                compromised += 1;
            }
            row(vec![
                "message_count".to_owned(),
                last_message.to_string(),
                compromised.to_string(),
                format!(
                    "{:.10}",
                    probability(compromised as u64, u64::from(self.users))
                ),
            ])?;
        }
        // Retain the observed endpoint even if the entire curve has zero wins.
        if self.max_messages_per_user > last_message {
            row(vec![
                "message_count".to_owned(),
                self.max_messages_per_user.to_string(),
                compromised.to_string(),
                format!(
                    "{:.10}",
                    probability(compromised as u64, u64::from(self.users))
                ),
            ])?;
        }
        Ok(())
    }
}

fn write_csv_row(writer: &mut impl Write, values: &[String]) -> std::io::Result<()> {
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            write!(writer, ",")?;
        }
        if value.contains([',', '\"', '\n', '\r']) {
            write!(writer, "\"{}\"", value.replace('\"', "\"\""))?;
        } else {
            write!(writer, "{value}")?;
        }
    }
    writeln!(writer)
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
