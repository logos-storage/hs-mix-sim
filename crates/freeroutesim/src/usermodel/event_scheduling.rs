//! Synchronous event scheduling for the hidden-service model.

use std::cmp::Ordering;
use std::collections::BinaryHeap;

struct ScheduledEvent<E> {
    time: u64,
    sequence: u64,
    event: E,
}

// BinaryHeap is a max-heap: reverse both keys for earliest-time-first,
// then insertion order. Payloads do not need to implement Ord.
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

#[cfg(test)]
mod tests {
    use super::Scheduler;

    #[test]
    fn jumps_to_events_in_timestamp_then_insertion_order() {
        // Deliberately has no Eq or Ord implementation.
        struct Event(&'static str);
        let mut scheduler = Scheduler::new();
        assert_eq!(scheduler.current_time(), 0);
        scheduler.schedule(900, Event("last"));
        scheduler.schedule(7, Event("first"));
        scheduler.schedule(7, Event("second"));
        assert_eq!(scheduler.next_event(900).unwrap().0, "first");
        assert_eq!(scheduler.current_time(), 7);
        // Follow-up events at this time run after events already queued there.
        scheduler.schedule(7, Event("third"));
        for expected in ["second", "third", "last"] {
            assert_eq!(scheduler.next_event(900).unwrap().0, expected);
        }
        assert_eq!(scheduler.current_time(), 900);
        assert!(scheduler.next_event(900).is_none());
        assert_eq!(scheduler.current_time(), 900);
    }

    #[test]
    fn deadline_is_inclusive_and_does_not_consume_later_events() {
        let mut scheduler = Scheduler::new();
        assert_eq!(scheduler.next_event(0), None);
        assert_eq!(scheduler.current_time(), 0);
        scheduler.schedule(0, "initial");
        scheduler.schedule(10, "deadline");
        scheduler.schedule(u64::MAX, "future");
        assert_eq!(scheduler.next_event(0), Some("initial"));
        assert_eq!(scheduler.next_event(10), Some("deadline"));
        assert_eq!(scheduler.next_event(10), None);
        assert_eq!(scheduler.current_time(), 10);
        assert_eq!(scheduler.next_event(u64::MAX), Some("future"));
        assert_eq!(scheduler.current_time(), u64::MAX);
    }

    #[test]
    #[should_panic(expected = "cannot schedule an event in the past")]
    fn rejects_events_in_the_past() {
        let mut scheduler = Scheduler::new();
        scheduler.schedule(10, ());
        scheduler.next_event(10);
        scheduler.schedule(9, ());
    }

    #[test]
    #[should_panic(expected = "event sequence number exhausted")]
    fn sequence_never_wraps() {
        let mut scheduler = Scheduler::new();
        scheduler.next_sequence = u64::MAX;
        scheduler.schedule(0, ());
    }
}
