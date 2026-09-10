//! Discrete-Event hidden-service traffic model, starting at time zero.
//!

use crate::adversary::Adversary;
use crate::path_sampler::PathSampler;
use crate::usermodel::event_scheduling::Scheduler;
use crate::usermodel::{RouteEvent, UserModel, UserModelInfo};

pub enum HiddenServiceEvent {
    CheckPath,
}

pub struct HiddenServiceModel<'a, S: PathSampler, A: Adversary> {
    model_info: UserModelInfo<'a>,
    scheduler: Scheduler<HiddenServiceEvent>,
    deadline: u64,
    stopped: bool,
    path_sampler: S,
    adversary: A,
}

impl<'a, S: PathSampler, A: Adversary> HiddenServiceModel<'a, S, A> {
    /// The simulation deadline is an inclusive timestamp in seconds.
    pub fn new(
        model_info: UserModelInfo<'a>,
        path_sampler: S,
        adversary: A,
        deadline: u64,
    ) -> Self {
        let mut model = Self {
            model_info,
            scheduler: Scheduler::new(),
            deadline,
            stopped: false,
            path_sampler,
            adversary,
        };
        // Seed events here once adversary/sampler event generation is defined.
        model.schedule_event(0, HiddenServiceEvent::CheckPath);
        model
    }

    /// Add an event before the run or while processing another event.
    /// Panics if the model has stopped or the timestamp is in the past.
    pub fn schedule_event(&mut self, time: u64, event: HiddenServiceEvent) {
        assert!(
            !self.stopped,
            "cannot schedule an event after the model stops"
        );
        self.scheduler.schedule(time, event);
    }

    fn process_event(&mut self, event: HiddenServiceEvent) -> Option<RouteEvent> {
        let time = self.scheduler.current_time();
        match event {
            HiddenServiceEvent::CheckPath => {
                let (topology_index, topology) = self.model_info.topology_at(time)?;
                let path = self.path_sampler.sample_path(topology_index, topology);
                Some((time, self.adversary.wins(&path)))
            }
        }
    }
}

impl<S: PathSampler, A: Adversary> UserModel for HiddenServiceModel<'_, S, A> {
    fn fetch_next(&mut self) -> Option<RouteEvent> {
        if self.stopped {
            return None;
        }
        let result = self
            .scheduler
            .next_event(self.deadline)
            .and_then(|event| self.process_event(event));
        // Stop on compromise, exhaustion, deadline, or missing topology.
        self.stopped = result.is_none_or(|(_, adversary_won)| adversary_won);
        result
    }
}

