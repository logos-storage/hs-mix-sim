//! Synchronous event scheduling for the hidden-service model.

use std::cmp::Ordering;
use std::collections::BinaryHeap;

struct ScheduledEvent<E> {
    time: u64,
    sequence: u64,
    event: E,
}

impl<E> Ord for ScheduledEvent<E> {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .time
            .cmp(&self.time)
            .then_with(|| other.sequence.cmp(&self.sequence))
    }
}

impl<E> PartialOrd for ScheduledEvent<E> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<E> PartialEq for ScheduledEvent<E> {
    fn eq(&self, other: &Self) -> bool {
        self.time == other.time && self.sequence == other.sequence
    }
}

impl<E> Eq for ScheduledEvent<E> {}

pub struct Scheduler<E> {
    current_time: u64,
    events: BinaryHeap<ScheduledEvent<E>>,
    next_sequence: u64,
}

impl<E> Scheduler<E> {
    pub fn new() -> Self {
        Self {
            current_time: 0,
            events: BinaryHeap::new(),
            next_sequence: 0,
        }
    }

    /// Current simulated time in seconds, initially zero.
    pub fn current_time(&self) -> u64 {
        self.current_time
    }

    /// Schedule at an absolute timestamp in seconds. Events at the current
    /// time are allowed; scheduling in the past or exhausting sequence IDs panics.
    pub fn schedule(&mut self, time: u64, event: E) {
        assert!(
            time >= self.current_time,
            "cannot schedule an event in the past"
        );
        let sequence = self.next_sequence;
        self.next_sequence = sequence
            .checked_add(1)
            .expect("event sequence number exhausted");
        self.events.push(ScheduledEvent {
            time,
            sequence,
            event,
        });
    }

    /// Pop the earliest event and jump directly to its time. The deadline is
    /// inclusive. An empty queue or an event beyond it leaves time and queue unchanged.
    pub fn next_event(&mut self, deadline: u64) -> Option<E> {
        if self.events.peek()?.time > deadline {
            return None;
        }
        let scheduled = self.events.pop()?;
        self.current_time = scheduled.time;
        Some(scheduled.event)
    }
}

