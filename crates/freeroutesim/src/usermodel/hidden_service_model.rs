//! Discrete-event hidden-service model, starting at time zero.

use crate::adversary::{Adversary, CompromiseEvent};
use crate::mixnet::Mixnet;
use crate::time_based_path_sampler::TimeBasedPathSampler;
use crate::usermodel::event_scheduling::Scheduler;
use crate::usermodel::{RouteEvent, UserModel};

enum HiddenServiceEvent<E> {
    Sampler(E),
    Adversary(CompromiseEvent),
}

pub struct HiddenServiceModel<'a, S: TimeBasedPathSampler, A: Adversary> {
    mixnet: &'a Mixnet,
    scheduler: Scheduler<HiddenServiceEvent<S::Event>>,
    deadline: u64,
    stopped: bool,
    initialized: bool,
    compromises_at_win: Option<u64>,
    path_sampler: S,
    adversary: A,
}

impl<'a, S: TimeBasedPathSampler, A: Adversary> HiddenServiceModel<'a, S, A> {
    /// Accept a sampler initialized at time zero. The first `fetch_next` call
    /// seeds its events, walks the initial paths, and schedules compromise attempts.
    /// The simulation deadline is an inclusive timestamp in seconds.
    pub fn new(mixnet: &'a Mixnet, path_sampler: S, adversary: A, deadline: u64) -> Self {
        Self {
            mixnet,
            scheduler: Scheduler::new(),
            deadline,
            stopped: false,
            initialized: false,
            compromises_at_win: None,
            path_sampler,
            adversary,
        }
    }

    fn schedule_sampler_events(&mut self) {
        for (time, event) in self.path_sampler.next_events(self.scheduler.current_time()) {
            self.scheduler
                .schedule(time, HiddenServiceEvent::Sampler(event));
        }
    }

    fn check_win(&mut self) -> RouteEvent {
        let time = self.scheduler.current_time();
        let won = self.adversary.wins(time, &self.path_sampler);
        for (time, event) in self.adversary.next_events(time) {
            self.scheduler
                .schedule(time, HiddenServiceEvent::Adversary(event));
        }
        if won {
            self.compromises_at_win = Some(self.adversary.compromises_done());
            self.stopped = true;
        }
        (time, won)
    }

    fn process_event(&mut self, event: HiddenServiceEvent<S::Event>) -> RouteEvent {
        let time = self.scheduler.current_time();
        match event {
            HiddenServiceEvent::Sampler(event) => {
                self.path_sampler.handle_event(time, event, self.mixnet);
            }
            HiddenServiceEvent::Adversary(event) => self.adversary.handle_event(time, event),
        }
        self.schedule_sampler_events();
        self.check_win()
    }
}

impl<S: TimeBasedPathSampler, A: Adversary> UserModel for HiddenServiceModel<'_, S, A> {
    fn fetch_next(&mut self) -> Option<RouteEvent> {
        // 1. Once this user's model finishes, every later fetch returns None.
        if self.stopped {
            return None;
        }

        // 2. On the first fetch, seed sampler events and check for a win at time 0.
        // check_win also schedules any newly discovered compromise attempts.
        if !self.initialized {
            self.initialized = true;
            self.schedule_sampler_events();
            return Some(self.check_win());
        }

        // 3. Take the earliest event within the inclusive deadline and jump to
        // its timestamp. No event is consumed if the queue is empty or too late.
        // process_event calls its owner, queues new events, and checks for a win.
        let result = self
            .scheduler
            .next_event(self.deadline)
            .map(|event| self.process_event(event));
        // 4. Stop permanently on exhaustion or deadline.
        // A win already sets stopped inside check_win, but still returns its result.
        if result.is_none() {
            self.stopped = true;
        }

        // 5. Report the check's timestamp and win outcome, or None if finished.
        result
    }

    fn compromises_before_win(&self) -> Option<u64> {
        self.compromises_at_win
    }
}
