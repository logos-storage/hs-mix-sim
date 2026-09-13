use crate::summary::{SimulationConfigSummary, SimulationSummary};
use crate::usermodel::{UserModel, UserModelIterator};
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::path::PathBuf;

/// Runtime state and output settings for one simulation run.
pub struct Simulator {
    users: u32,
    config: SimulationConfigSummary,
    csv_file_path: Option<PathBuf>,
}

impl Simulator {
    pub fn new(
        users: u32,
        config: SimulationConfigSummary,
        csv_file_path: Option<PathBuf>,
    ) -> Self {
        Self {
            users,
            config,
            csv_file_path,
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

        summary.print_summary(&self.config);
        if let Some(csv_file_path) = &self.csv_file_path {
            summary
                .write_csv(&self.config, csv_file_path)
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

    #[inline]
    pub fn limit_sec(&self) -> u64 {
        u64::from(self.config.days) * 24 * 60 * 60
    }
}
