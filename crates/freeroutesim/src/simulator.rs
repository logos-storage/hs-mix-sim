use crate::mixnet::MixnetConfig;
use crate::summary::{SdlmSummary, SimulationConfigSummary, SimulationSummary};
use crate::usermodel::{UserModel, UserModelIterator};
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::path::PathBuf;

/// Runtime state and output settings for one simulation run.
#[derive(Default)]
pub struct Simulator {
    /// The number of users we want to simulate
    users: u32,
    /// Parameters used to generate the static network.
    mixnet_config: MixnetConfig,
    /// The number of days for running the experiment
    days: u32,
    /// CSV reporting interval in seconds, independent of simulated time.
    csv_interval_seconds: u32,
    /// number of hops in sampled paths
    path_hops: usize,
    /// Optional csv output.
    csv_file_path: Option<PathBuf>,
    /// sampler type
    sampler_type: &'static str,
    /// user traffic model type
    model_type: &'static str,
    /// Adversary implementation selected for this run.
    adversary_type: &'static str,
    /// Formula-based S-DLM summary for supported anonymous-download samplers.
    sdlm: Option<SdlmSummary>,
}

impl Simulator {
    pub fn new(
        users: u32,
        mixnet_config: MixnetConfig,
        days: u32,
        csv_interval_seconds: u32,
        path_hops: usize,
        csv_file_path: Option<PathBuf>,
        sampler_type: &'static str,
        model_type: &'static str,
        adversary_type: &'static str,
        sdlm: Option<SdlmSummary>,
    ) -> Self {
        Self {
            users,
            mixnet_config,
            days,
            csv_interval_seconds,
            path_hops,
            csv_file_path,
            sampler_type,
            model_type,
            adversary_type,
            sdlm,
        }
    }

    /// Run every user's model
    /// Parallelism is over users. Each worker owns one user model and samples all
    /// of that user's messages or hidden-service checks until the first win or
    /// the time limit.
    pub fn simulate<T: UserModel + Send>(&mut self, user_models: Vec<UserModelIterator<T>>) {
        let progress = self.make_progress_bar();
        let simulation_limit = self.limit_sec();

        assert_eq!(
            user_models.len(),
            self.users as usize,
            "the number of user models must equal the configured number of users"
        );

        let mut summary = user_models
            .into_par_iter()
            .map(|mut user_model| {
                let mut user_summary = SimulationSummary::for_user();
                for (message_time, adversary_won) in &mut user_model {
                    if message_time > simulation_limit {
                        break;
                    }
                    user_summary.record_message(message_time, adversary_won);

                    if adversary_won {
                        break;
                    }
                }
                if let Some(count) = user_model.compromises_before_win() {
                    user_summary.record_compromises_before_win(count);
                }
                if let Some(progress) = &progress {
                    progress.inc(1);
                }
                user_summary
            })
            .reduce(SimulationSummary::default, SimulationSummary::merge);

        if let Some(progress) = &progress {
            progress.finish_with_message("simulation complete");
        }

        let config_summary = self.summary_config();
        summary.print_summary(&config_summary);
        if let Some(csv_file_path) = &self.csv_file_path {
            summary
                .write_timeseries_csv(&config_summary, csv_file_path)
                .expect("failed to write simulation CSV");
        }
    }

    /// helper to make progress bar
    fn make_progress_bar(&self) -> Option<ProgressBar> {
        let bar = ProgressBar::new(u64::from(self.users));
        let style = ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} users ({percent}%) eta {eta_precise}",
        )
        .unwrap_or_else(|_| ProgressStyle::default_bar())
        .progress_chars("#>-");
        bar.set_style(style);
        Some(bar)
    }

    fn summary_config(&self) -> SimulationConfigSummary {
        SimulationConfigSummary {
            days: self.days,
            csv_interval_seconds: self.csv_interval_seconds,
            mix_nodes: self.mixnet_config.mix_size,
            path_hops: self.path_hops,
            path_sampler_type: self.sampler_type,
            user_model_type: self.model_type,
            adversary_type: self.adversary_type,
            malicious_node_fraction: self.mixnet_config.malicious_node_fraction,
            malicious_bandwidth_fraction: self.mixnet_config.malicious_bandwidth_fraction,
            sdlm: self.sdlm,
        }
    }

    #[inline]
    pub fn limit_sec(&self) -> u64 {
        u64::from(self.days) * 24 * 60 * 60
    }
}
