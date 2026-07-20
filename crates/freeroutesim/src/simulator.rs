use crate::summary::{SimulationConfigSummary, SimulationSummary};
use crate::topologygen::TopologyGenerator;
use crate::usermodel::{UserModel, UserModelIterator};
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::path::PathBuf;

/// Runtime state and output settings for one simulation run.
#[derive(Default)]
pub struct Simulator {
    /// The number of users we want to simulate
    users: u32,
    /// The Network topology generator
    topology_generator: TopologyGenerator,
    /// The number of days for running the experiment
    days: u32,
    /// each topology lifetime 
    epoch_time: u32,
    /// number of hops in sampled paths
    path_hops: usize,
    /// Optional csv output.
    csv_file_path: Option<PathBuf>,
    /// sampler type
    sampler_type: &'static str,
    /// user traffic model type
    model_type: &'static str,
}

impl Simulator {
    pub fn new(
        users: u32,
        topology_generator: TopologyGenerator,
        days: u32,
        epoch_time: u32,
        path_hops: usize,
        csv_file_path: Option<PathBuf>,
        sampler_type: &'static str,
        model_type: &'static str,
    ) -> Self {
        Self {
            users,
            topology_generator,
            days,
            epoch_time,
            path_hops,
            csv_file_path,
            sampler_type,
            model_type,
        }
    }

    /// Run every user's model
    /// Parallelism is over users. Each worker owns one user model and samples all
    /// of that user's messages until first compromise or the time limit.
    pub fn simulate<T: UserModel + Send>(&mut self, mut user_models: Vec<UserModelIterator<T>>) {
        let progress = self.make_progress_bar();
        let simulation_limit = self.limit_sec();

        assert_eq!(
            user_models.len(),
            self.users as usize,
            "the number of user models must equal the configured number of users"
        );

        let mut summary = (0..self.users)
            .into_par_iter()
            .zip(&mut user_models)
            .map(|(_user, mut user_models)| {
                let mut user_summary = SimulationSummary::for_user();
                for (message_time, is_malicious) in &mut user_models {
                    if message_time > simulation_limit {
                        break;
                    }
                    user_summary.record_message(message_time, is_malicious);

                    if is_malicious {
                        break;
                    }
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
            epoch_seconds: self.epoch_time,
            topologies_loaded: self.topology_generator.config.epochs as usize,
            path_hops: self.path_hops,
            path_sampler_type: self.sampler_type,
            user_model_type: self.model_type,
            malicious_node_fraction: self.topology_generator.config.malicious_node_fraction,
            malicious_bandwidth_fraction: self
                .topology_generator
                .config
                .malicious_bandwidth_fraction,
            churn_rate: self.topology_generator.config.churn_rate,
        }
    }

    #[inline]
    pub fn limit_sec(&self) -> u64 {
        u64::from(self.days) * 24 * 60 * 60
    }
}
