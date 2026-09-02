//! Shared user-model trait
//!
//! Models implement `fetch_next` to say when the next message is sent and
//! whether their configured adversary won on its sampled path.
//!

mod download_session_model;
mod hidden_service_model;
mod simple_model;

use std::ops::{Deref, DerefMut};

pub use download_session_model::DownloadSessionModel;
pub(crate) use download_session_model::session_path_count;
pub use hidden_service_model::HiddenServiceModel;
pub use simple_model::SimpleModel;

/// time of the message
pub type MessageTime = u64;

/// A route event that gets fired whenever a packet is sent through the mix, containing:
/// - message time
/// - whether the adversary won on the sampled path
pub type RouteEvent = (MessageTime, bool);

pub trait UserModel {
    /// Return the next route event for this user, or `None` once the model has
    /// reached the simulation time limit.
    fn fetch_next(&mut self) -> Option<RouteEvent>;
}

pub struct UserModelInfo<'a> {
    topologies: &'a [crate::topologygen::Topology],
    epoch_seconds: u32,
}

impl<'a> UserModelInfo<'a> {
    pub fn new(topologies: &'a [crate::topologygen::Topology], epoch_seconds: u32) -> Self {
        Self {
            topologies,
            epoch_seconds,
        }
    }

    fn topology_at(
        &self,
        message_time: MessageTime,
    ) -> Option<(usize, &'a crate::topologygen::Topology)> {
        if self.epoch_seconds == 0 {
            return None;
        }

        let index = usize::try_from(message_time / u64::from(self.epoch_seconds)).ok()?;
        self.topologies.get(index).map(|topology| (index, topology))
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
