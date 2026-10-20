// SPDX-License-Identifier: AGPL-3.0-only
// Translated from Soar `Core/SoarKernel/src/decision_process/decide.cpp`
// (`run_preference_semantics`, `run_non_context_preference_semantics`) and
// `decision_manipulation.cpp` (`select_force`)
// (https://github.com/SoarGroup/Soar, commit 3fc337a, BSD, Copyright
// 1995-2012 Carnegie Mellon University, The Regents of the University of
// Michigan, USC/ISI). Modified: see docs/DEVIATIONS.md.

//! Preference semantics: from a slot's preferences to a candidate set and an
//! impasse type.
//!
//! Stages, in Soar's order. Each of stages 2–5 ends early once at most one
//! candidate is left.
//!
//! 1. **Requires.** More than one distinct required value, or a required
//!    value that is also prohibited, is a constraint failure. Otherwise the
//!    required value wins. Rejects do not apply to requires.
//! 2. **Acceptables minus prohibits and rejects.**
//! 3. **Better / worse.** A candidate dominated by another value is marked
//!    conflicted. Marking walks the better list, then the worse list, and a
//!    value that is already conflicted still dominates. If every candidate
//!    is conflicted, the result is a conflict impasse over them.
//! 4. **Best**, then **worst**. Each filter applies only if it leaves at
//!    least one candidate.
//! 5. **Indifference.** If every candidate is unary-indifferent,
//!    numeric-indifferent, or binary-indifferent to every other candidate,
//!    the exploration policy picks one. Otherwise the result is a tie.
//!
//! Candidate lists keep Soar's order: they are built by pushing to the front
//! while walking a preference list.

use alloc::vec::Vec;

use crate::exploration::Exploration;
use crate::preference::{PreferenceType as T, Slot};
use crate::rng::SoarRng;

/// Impasse types, with Soar's numbering (`constants.h`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ImpasseType {
    None = 0,
    ConstraintFailure = 1,
    Conflict = 2,
    Tie = 3,
    NoChange = 4,
}

/// Result of preference semantics: the impasse type, and either the winner
/// (or, in read-only mode, every mutually indifferent candidate) or the
/// candidates the impasse is over.
#[derive(Clone, Debug, PartialEq)]
pub struct Semantics<V> {
    pub impasse: ImpasseType,
    pub candidates: Vec<V>,
}

// Decider flags (decide.cpp). Values matter only for equality.
const NOTHING: u8 = 0;
const CANDIDATE: u8 = 1;
const CONFLICTED: u8 = 2;
const BEST: u8 = 4;
const WORST: u8 = 5;
const UNARY_INDIFFERENT: u8 = 6;
const UNARY_INDIFFERENT_CONSTANT: u8 = 8;

/// Soar stores decider flags on the value symbols themselves. Here they live
/// in a table that is fresh for every call. Flags left over from an earlier
/// decision are never read by upstream either.
struct Flags<V> {
    entries: Vec<(V, u8)>,
}

impl<V: PartialEq + Copy> Flags<V> {
    fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    fn get(&self, v: &V) -> u8 {
        self.entries
            .iter()
            .find(|(k, _)| k == v)
            .map_or(NOTHING, |(_, f)| *f)
    }

    fn set(&mut self, v: V, flag: u8) {
        match self.entries.iter_mut().find(|(k, _)| *k == v) {
            Some(e) => e.1 = flag,
            None => self.entries.push((v, flag)),
        }
    }
}

/// How a call may resolve indifference.
pub(crate) enum Mode<'a, V> {
    /// Soar's `consistency = true`: no exploration, no forced selection, no
    /// random numbers. Mutually indifferent candidates are all returned.
    ReadOnly,
    /// A real decision.
    Decide {
        exploration: &'a Exploration,
        rng: &'a mut SoarRng,
        force: &'a mut Option<V>,
    },
}

/// Read-only preference semantics. This is what Soar's `preferences`
/// command runs to list candidates and their selection probabilities
/// (`consistency = true`).
pub fn run_preference_semantics_read_only<V: PartialEq + Copy>(slot: &Slot<V>) -> Semantics<V> {
    run(slot, Mode::ReadOnly)
}

/// Preference semantics for a real decision (`run_preference_semantics`
/// with `consistency = false`). Indifferent candidates are resolved by
/// `exploration`, drawing from `rng`. If `force` holds a value that has an
/// acceptable preference, that value is selected outright (Soar's
/// `decide select`) and `force` is cleared.
pub fn run_preference_semantics<V: PartialEq + Copy>(
    slot: &Slot<V>,
    exploration: &Exploration,
    rng: &mut SoarRng,
    force: &mut Option<V>,
) -> Semantics<V> {
    run(
        slot,
        Mode::Decide {
            exploration,
            rng,
            force,
        },
    )
}

fn done<V>(impasse: ImpasseType, candidates: Vec<V>) -> Semantics<V> {
    Semantics {
        impasse,
        candidates,
    }
}

pub(crate) fn run<V: PartialEq + Copy>(slot: &Slot<V>, mode: Mode<'_, V>) -> Semantics<V> {
    if slot.is_empty() {
        return done(ImpasseType::None, Vec::new());
    }

    // Forced selection (`select_force`): the first acceptable preference for
    // the forced value, ignoring every other preference.
    let mut mode = mode;
    if let Mode::Decide { force, .. } = &mut mode {
        if let Some(f) = **force {
            if let Some(p) = slot.list(T::Acceptable).iter().find(|p| p.value == f) {
                **force = None;
                return done(ImpasseType::None, alloc::vec![p.value]);
            }
        }
    }

    let mut flags = Flags::new();

    // === Requires ===
    if !slot.list(T::Require).is_empty() {
        for p in slot.list(T::Require) {
            flags.set(p.value, NOTHING);
        }
        let mut candidates = Vec::new();
        for p in slot.list(T::Require) {
            if flags.get(&p.value) == NOTHING {
                candidates.insert(0, p.value);
                flags.set(p.value, CANDIDATE);
            }
        }
        if candidates.len() > 1 {
            return done(ImpasseType::ConstraintFailure, candidates);
        }
        let value = candidates[0];
        if slot.list(T::Prohibit).iter().any(|p| p.value == value) {
            return done(ImpasseType::ConstraintFailure, candidates);
        }
        return done(ImpasseType::None, candidates);
    }

    // === Acceptables, prohibits, rejects ===
    for p in slot.list(T::Acceptable) {
        flags.set(p.value, CANDIDATE);
    }
    for p in slot.list(T::Prohibit).iter().chain(slot.list(T::Reject)) {
        flags.set(p.value, NOTHING);
    }
    let mut candidates = Vec::new();
    for p in slot.list(T::Acceptable) {
        if flags.get(&p.value) == CANDIDATE {
            candidates.insert(0, p.value);
            flags.set(p.value, NOTHING);
        }
    }
    if candidates.len() <= 1 {
        return done(ImpasseType::None, candidates);
    }

    // === Better / worse ===
    let better = slot.list(T::Better);
    let worse = slot.list(T::Worse);
    if !better.is_empty() || !worse.is_empty() {
        for p in better.iter().chain(worse) {
            flags.set(p.value, NOTHING);
            if let Some(r) = p.referent.value() {
                flags.set(*r, NOTHING);
            }
        }
        for c in &candidates {
            flags.set(*c, CANDIDATE);
        }
        // A pair is considered only if both sides are flagged (candidate or
        // conflicted) and at least one is still a candidate.
        let dominates = |flags: &Flags<V>, j: &V, k: &V| {
            let (fj, fk) = (flags.get(j), flags.get(k));
            fj != NOTHING && fk != NOTHING && (fj == CANDIDATE || fk == CANDIDATE)
        };
        for p in better {
            let Some(k) = p.referent.value() else {
                continue;
            };
            if p.value == *k {
                continue;
            }
            if dominates(&flags, &p.value, k) {
                flags.set(*k, CONFLICTED);
            }
        }
        for p in worse {
            let Some(k) = p.referent.value() else {
                continue;
            };
            if p.value == *k {
                continue;
            }
            if dominates(&flags, &p.value, k) {
                flags.set(p.value, CONFLICTED);
            }
        }
        if !candidates.iter().any(|c| flags.get(c) == CANDIDATE) {
            candidates.retain(|c| flags.get(c) == CONFLICTED);
            return done(ImpasseType::Conflict, candidates);
        }
        candidates.retain(|c| flags.get(c) != CONFLICTED);
    }
    if candidates.len() <= 1 {
        return done(ImpasseType::None, candidates);
    }

    // === Bests ===
    if !slot.list(T::Best).is_empty() {
        for c in &candidates {
            flags.set(*c, NOTHING);
        }
        for p in slot.list(T::Best) {
            flags.set(p.value, BEST);
        }
        if candidates.iter().any(|c| flags.get(c) == BEST) {
            candidates.retain(|c| flags.get(c) == BEST);
        }
    }
    if candidates.len() <= 1 {
        return done(ImpasseType::None, candidates);
    }

    // === Worsts ===
    if !slot.list(T::Worst).is_empty() {
        for c in &candidates {
            flags.set(*c, NOTHING);
        }
        for p in slot.list(T::Worst) {
            flags.set(p.value, WORST);
        }
        if candidates.iter().any(|c| flags.get(c) != WORST) {
            candidates.retain(|c| flags.get(c) != WORST);
        }
    }
    if candidates.len() <= 1 {
        return done(ImpasseType::None, candidates);
    }

    // === Indifferents ===
    for c in &candidates {
        flags.set(*c, NOTHING);
    }
    for p in slot.list(T::UnaryIndifferent) {
        flags.set(p.value, UNARY_INDIFFERENT);
    }
    for p in slot.list(T::NumericIndifferent) {
        flags.set(p.value, UNARY_INDIFFERENT_CONSTANT);
    }
    let binary = slot.list(T::BinaryIndifferent);
    let all_indifferent = candidates.iter().enumerate().all(|(ci, cand)| {
        let f = flags.get(cand);
        if f == UNARY_INDIFFERENT || f == UNARY_INDIFFERENT_CONSTANT {
            return true;
        }
        candidates.iter().enumerate().all(|(pi, other)| {
            pi == ci
                || binary.iter().any(|p2| {
                    let r = p2.referent.value();
                    (p2.value == *cand && r == Some(other))
                        || (p2.value == *other && r == Some(cand))
                })
        })
    });

    if !all_indifferent {
        return done(ImpasseType::Tie, candidates);
    }
    match mode {
        Mode::ReadOnly => done(ImpasseType::None, candidates),
        Mode::Decide {
            exploration, rng, ..
        } => {
            let chosen = exploration.choose_according_to_policy(slot, &candidates, rng);
            done(ImpasseType::None, alloc::vec![candidates[chosen]])
        }
    }
}

/// Candidates for a non-context slot: acceptables minus rejects (not
/// prohibits). No impasse is possible (`run_non_context_preference_semantics`).
pub fn run_non_context_preference_semantics<V: PartialEq + Copy>(slot: &Slot<V>) -> Vec<V> {
    let mut flags = Flags::new();
    for p in slot.list(T::Acceptable) {
        flags.set(p.value, CANDIDATE);
    }
    for p in slot.list(T::Reject) {
        flags.set(p.value, NOTHING);
    }
    let mut candidates = Vec::new();
    for p in slot.list(T::Acceptable) {
        if flags.get(&p.value) == CANDIDATE {
            candidates.insert(0, p.value);
            flags.set(p.value, NOTHING);
        }
    }
    candidates
}

#[cfg(test)]
#[path = "semantics_test.rs"]
mod semantics_test;
