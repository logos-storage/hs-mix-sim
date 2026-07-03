/**
* Simple trait definition for any concrete user model
*/
use crate::config::TopologyConfig;
use crate::config::{GUARDS_LAYER, GUARDS_SAMPLE_SIZE, GUARDS_SAMPLE_SIZE_EXTEND};
use crate::config::{VANGUARDS_LAYER, VANGUARDS_SAMPLE_SIZE, VANGUARDS_SAMPLE_SIZE_EXTEND};
use crate::config::Mixnode;
use rand::prelude::*;
use std::ops::{Deref, DerefMut};

pub trait RequestHandler {
    type Out;

    fn fetch_next(&mut self) -> Option<Self::Out>;
}

pub type RouteEvent<'a> = (u64, Option<&'a Mixnode>, Option<&'a Mixnode>, Option<u128>);

pub trait UserModel<'a> {
    fn new(tot_users: u32, epoch: u32, user_info: UserModelInfo<'a>) -> Self;
    fn set_limit(&mut self, limit: u64);
}

// Wrapper that gives every user model a common Iterator implementation.
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

impl<'a, T> Iterator for UserModelIterator<T>
where
    T: UserModel<'a> + RequestHandler<Out = RouteEvent<'a>>,
{
    type Item = RouteEvent<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.fetch_next()
    }
}

pub struct UserModelInfo<'a> {
    /// Mixnet topology
    topos: &'a [TopologyConfig],
    /// Vanguards information
    vanguards: Option<Vec<&'a Mixnode>>,
    /// The vanguard we're currently using
    selected_vanguard: Option<&'a Mixnode>,
    /// Guards information
    guards: Option<Vec<&'a Mixnode>>,
    /// The guard we're currently using
    selected_guard: Option<&'a Mixnode>,
    /// epoch length in seconds
    epoch: u32,
    curr_idx: usize,
}

impl<'a> UserModelInfo<'a> {
    pub fn new(
        _userid: u32,
        topos: &'a [TopologyConfig],
        epoch: u32,
        use_guards: bool,
        use_vanguards: bool,
    ) -> Self {
        let mut rng = rand::thread_rng();
        let mut vanguards: Option<Vec<&'a Mixnode>> = None;
        let mut selected_vanguard: Option<&'a Mixnode> = None;
        let mut guards: Option<Vec<&'a Mixnode>> = None;
        let mut selected_guard: Option<&'a Mixnode> = None;
        if use_vanguards {
            vanguards = Some(
                topos[0]
                    .sample_layer(VANGUARDS_LAYER, VANGUARDS_SAMPLE_SIZE, &mut rng)
                    .collect(),
            );
            selected_vanguard = Some(vanguards.as_ref().unwrap()[0]);
        }
        if use_guards {
            guards = Some(
                topos[0]
                    .sample_layer(GUARDS_LAYER, GUARDS_SAMPLE_SIZE, &mut rng)
                    .collect(),
            );
            selected_guard = Some(guards.as_ref().unwrap()[0]);
        }
        UserModelInfo {
            topos,
            vanguards,
            selected_vanguard,
            guards,
            selected_guard,
            epoch,
            curr_idx: 0,
        }
    }

    #[inline]
    pub fn get_selected_vanguard(&self) -> Option<&'a Mixnode> {
        self.selected_vanguard
    }

    #[inline]
    pub fn get_selected_guard(&self) -> Option<&'a Mixnode> {
        self.selected_guard
    }

    /// Potentially changes this user's persistent vanguard and guard.
    #[inline]
    pub fn update<R: Rng + ?Sized>(&mut self, message_timing: u64, rng: &mut R) {
        let idx = (message_timing / self.epoch as u64) as usize;
        if idx > self.curr_idx {
            self.curr_idx = idx;
            update_persistent_selection(
                self.topos,
                self.curr_idx,
                VANGUARDS_LAYER,
                VANGUARDS_SAMPLE_SIZE_EXTEND,
                &mut self.vanguards,
                &mut self.selected_vanguard,
                rng,
            );
            update_persistent_selection(
                self.topos,
                self.curr_idx,
                GUARDS_LAYER,
                GUARDS_SAMPLE_SIZE_EXTEND,
                &mut self.guards,
                &mut self.selected_guard,
                rng,
            );
        }
    }
}

fn update_persistent_selection<'a, R: Rng + ?Sized>(
    topos: &'a [TopologyConfig],
    topoidx: usize,
    layer: usize,
    extend_size: usize,
    candidates: &mut Option<Vec<&'a Mixnode>>,
    selected: &mut Option<&'a Mixnode>,
    rng: &mut R,
) {
    let Some(candidates) = candidates else {
        return;
    };

    if let Some(candidate) = candidates
        .iter()
        .find(|candidate| topos[topoidx].has_mix_in_layer(layer, candidate.mixid))
    {
        *selected = Some(*candidate);
        return;
    }

    let candidate_idx = candidates.len();
    candidates.extend(topos[topoidx].sample_layer(layer, extend_size, rng));
    if candidates.len() <= candidate_idx {
        panic!(
            "Did the persistent candidate set properly extend? len: {}",
            candidates.len()
        );
    }
    *selected = Some(candidates[candidate_idx]);
}

#[test]
fn user_model_info_can_use_guards_without_vanguards() {
    let configs = crate::config::load("testfiles/single_layout/1000_137_Random_BP_layout.csv");
    let user_info = UserModelInfo::new(0, &configs, 86401, true, false);

    assert!(user_info.get_selected_guard().is_some());
    assert!(user_info.get_selected_vanguard().is_none());
}

#[test]
fn user_model_info_can_use_guards_and_vanguards() {
    let configs = crate::config::load("testfiles/single_layout/1000_137_Random_BP_layout.csv");
    let user_info = UserModelInfo::new(0, &configs, 86401, true, true);

    assert!(user_info.get_selected_guard().is_some());
    assert!(user_info.get_selected_vanguard().is_some());
}

#[test]
fn user_model_info_can_disable_guards_and_vanguards() {
    let configs = crate::config::load("testfiles/single_layout/1000_137_Random_BP_layout.csv");
    let user_info = UserModelInfo::new(0, &configs, 86401, false, false);

    assert!(user_info.get_selected_guard().is_none());
    assert!(user_info.get_selected_vanguard().is_none());
}
