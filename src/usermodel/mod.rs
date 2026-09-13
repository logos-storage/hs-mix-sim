//! Shared user-model trait
//!
//! Models implement `fetch_next` to report the next message or hidden-service
//! check and whether their configured adversary won.
//!

mod download_session_model;
mod event_scheduling;
mod hidden_service_model;
mod simple_model;

use std::ops::{Deref, DerefMut};

pub use download_session_model::DownloadSessionModel;
pub(crate) use download_session_model::session_path_count;
pub use hidden_service_model::HiddenServiceModel;
pub use simple_model::SimpleModel;
pub(crate) use simple_model::{INTERVAL_MAX, INTERVAL_MIN};

/// Simulated time in seconds of a message or hidden-service check.
pub type MessageTime = u64;

/// A processed message or hidden-service check, containing:
/// - simulated time in seconds
/// - whether the adversary won
///
/// Hidden-service checks include initialization and scheduled sampler or adversary
/// events; they do not represent packets sent through the mix.
pub type RouteEvent = (MessageTime, bool);

pub trait UserModel {
    /// Return the next route event for this user, or `None` once the model has
    /// reached the simulation time limit, exhausted its events, or stopped.
    fn fetch_next(&mut self) -> Option<RouteEvent>;

    /// Number of completed node compromises when the adversary won, if tracked.
    /// Initially malicious nodes do not count. Return `None` before a win or
    /// when the model does not track this statistic; an all-Sybil win is `Some(0)`.
    fn compromises_before_win(&self) -> Option<u64> {
        None
    }
}

/// Wrapper that gives every user model a common Iterator implementation.
pub struct UserModelIterator<T>(pub T);

impl<T> Deref for UserModelIterator<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for UserModelIterator<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> Iterator for UserModelIterator<T>
where
    T: UserModel,
{
    type Item = RouteEvent;

    fn next(&mut self) -> Option<Self::Item> {
        self.fetch_next()
    }
}
