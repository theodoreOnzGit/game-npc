// SPDX-License-Identifier: AGPL-3.0-only
//! Unit tests for preference semantics. Upstream Soar has no unit tests for
//! this code (its tests run whole agents); these were written from reading
//! `decide.cpp`. `tests/differential.rs` checks the same behaviour against
//! the real `soar` binary.

use super::*;
use crate::exploration::{Exploration, ParameterName, Policy};
use crate::preference::{Preference, PreferenceType::*};
use alloc::vec;

fn slot(prefs: &[Preference<u32>]) -> Slot<u32> {
    let mut s = Slot::new();
    for p in prefs {
        s.push(*p);
    }
    s
}

fn acc(v: u32) -> Preference<u32> {
    Preference::unary(Acceptable, v)
}

fn ro(prefs: &[Preference<u32>]) -> Semantics<u32> {
    run_preference_semantics_read_only(&slot(prefs))
}

#[test]
fn empty_slot_has_no_candidates() {
    assert_eq!(ro(&[]), done(ImpasseType::None, vec![]));
}

#[test]
fn candidates_are_in_reverse_list_order_and_deduplicated() {
    let r = ro(&[acc(1), acc(2), acc(1), acc(3)]);
    assert_eq!(r, done(ImpasseType::Tie, vec![3, 2, 1]));
}

#[test]
fn reject_and_prohibit_remove_candidates() {
    let r = ro(&[
        acc(1),
        acc(2),
        acc(3),
        Preference::unary(Reject, 1),
        Preference::unary(Prohibit, 3),
    ]);
    assert_eq!(r, done(ImpasseType::None, vec![2]));
}

#[test]
fn single_require_wins_over_everything_but_prohibit() {
    let r = ro(&[
        acc(1),
        acc(2),
        Preference::unary(Require, 2),
        Preference::unary(Reject, 2),
    ]);
    assert_eq!(r, done(ImpasseType::None, vec![2]));
    let r = ro(&[
        Preference::unary(Require, 2),
        Preference::unary(Prohibit, 2),
    ]);
    assert_eq!(r, done(ImpasseType::ConstraintFailure, vec![2]));
}

#[test]
fn two_requires_are_a_constraint_failure() {
    let r = ro(&[
        Preference::unary(Require, 1),
        Preference::unary(Require, 2),
        Preference::unary(Require, 1),
    ]);
    assert_eq!(r, done(ImpasseType::ConstraintFailure, vec![2, 1]));
}

#[test]
fn better_removes_the_dominated() {
    let r = ro(&[
        acc(1),
        acc(2),
        acc(3),
        Preference::binary(Better, 1, 3),
        Preference::unary(UnaryIndifferent, 1),
        Preference::unary(UnaryIndifferent, 2),
    ]);
    assert_eq!(r, done(ImpasseType::None, vec![2, 1]));
}

#[test]
fn worse_marks_the_value_side() {
    let r = ro(&[acc(1), acc(2), Preference::binary(Worse, 2, 1)]);
    assert_eq!(r, done(ImpasseType::None, vec![1]));
}

#[test]
fn cycle_is_a_conflict_over_both() {
    let r = ro(&[
        acc(1),
        acc(2),
        Preference::binary(Better, 1, 2),
        Preference::binary(Better, 2, 1),
    ]);
    assert_eq!(r, done(ImpasseType::Conflict, vec![2, 1]));
}

#[test]
fn conflicted_value_still_dominates() {
    // 1 > 2 marks 2 conflicted; 2 > 3 then still marks 3 (3 is a candidate).
    let r = ro(&[
        acc(1),
        acc(2),
        acc(3),
        Preference::binary(Better, 1, 2),
        Preference::binary(Better, 2, 3),
    ]);
    assert_eq!(r, done(ImpasseType::None, vec![1]));
}

#[test]
fn self_preference_and_non_candidate_referent_are_ignored() {
    let r = ro(&[
        acc(1),
        acc(2),
        Preference::binary(Better, 1, 1),
        Preference::binary(Better, 9, 2),
    ]);
    assert_eq!(r, done(ImpasseType::Tie, vec![2, 1]));
}

#[test]
fn best_filters_only_if_some_candidate_is_best() {
    let r = ro(&[acc(1), acc(2), acc(3), Preference::unary(Best, 2)]);
    assert_eq!(r, done(ImpasseType::None, vec![2]));
    let r = ro(&[acc(1), acc(2), Preference::unary(Best, 9)]);
    assert_eq!(r, done(ImpasseType::Tie, vec![2, 1]));
}

#[test]
fn worst_filters_only_if_something_survives() {
    let r = ro(&[acc(1), acc(2), Preference::unary(Worst, 2)]);
    assert_eq!(r, done(ImpasseType::None, vec![1]));
    let r = ro(&[
        acc(1),
        acc(2),
        Preference::unary(Worst, 1),
        Preference::unary(Worst, 2),
    ]);
    assert_eq!(r, done(ImpasseType::Tie, vec![2, 1]));
}

#[test]
fn indifference_needs_every_pair_covered() {
    // Unary on all: indifferent.
    let r = ro(&[
        acc(1),
        acc(2),
        Preference::unary(UnaryIndifferent, 1),
        Preference::unary(UnaryIndifferent, 2),
    ]);
    assert_eq!(r, done(ImpasseType::None, vec![2, 1]));
    // Numeric counts as unary.
    let r = ro(&[
        acc(1),
        acc(2),
        Preference::numeric(1, 0.5),
        Preference::numeric(2, 0.0),
    ]);
    assert_eq!(r, done(ImpasseType::None, vec![2, 1]));
    // Binary in either direction covers a pair.
    let r = ro(&[acc(1), acc(2), Preference::binary(BinaryIndifferent, 2, 1)]);
    assert_eq!(r, done(ImpasseType::None, vec![2, 1]));
    // A unary-indifferent candidate does not cover the pairs of the others:
    // 2 has no binary preference with 3, so this is a tie.
    let r = ro(&[
        acc(1),
        acc(2),
        acc(3),
        Preference::binary(BinaryIndifferent, 1, 2),
        Preference::unary(UnaryIndifferent, 3),
    ]);
    assert_eq!(r, done(ImpasseType::Tie, vec![3, 2, 1]));
    // One uncovered pair is a tie.
    let r = ro(&[
        acc(1),
        acc(2),
        acc(3),
        Preference::binary(BinaryIndifferent, 1, 2),
    ]);
    assert_eq!(r, done(ImpasseType::Tie, vec![3, 2, 1]));
}

#[test]
fn decide_mode_resolves_indifference_with_the_policy() {
    let s = slot(&[
        acc(1),
        acc(2),
        acc(3),
        Preference::unary(UnaryIndifferent, 1),
        Preference::unary(UnaryIndifferent, 2),
        Preference::unary(UnaryIndifferent, 3),
    ]);
    let mut rng = SoarRng::new(1);
    let first = Exploration::new(Policy::First);
    let last = Exploration::new(Policy::Last);
    assert_eq!(
        run_preference_semantics(&s, &first, &mut rng, &mut None).candidates,
        vec![3]
    );
    assert_eq!(
        run_preference_semantics(&s, &last, &mut rng, &mut None).candidates,
        vec![1]
    );
}

#[test]
fn forced_selection_bypasses_preferences_and_is_consumed() {
    let s = slot(&[
        acc(1),
        acc(2),
        Preference::unary(Reject, 2),
        Preference::unary(Require, 1),
    ]);
    let e = Exploration::default();
    let mut rng = SoarRng::new(1);
    let mut force = Some(2);
    let r = run_preference_semantics(&s, &e, &mut rng, &mut force);
    assert_eq!(r, done(ImpasseType::None, vec![2]));
    assert_eq!(force, None);
    // Not acceptable: ignored and kept for later.
    let mut force = Some(9);
    let r = run_preference_semantics(&s, &e, &mut rng, &mut force);
    assert_eq!(r.candidates, vec![1]);
    assert_eq!(force, Some(9));
}

#[test]
fn read_only_mode_draws_no_random_numbers() {
    let s = slot(&[
        acc(1),
        acc(2),
        Preference::numeric(1, 0.5),
        Preference::numeric(2, 0.5),
    ]);
    let _ = run_preference_semantics_read_only(&s);
    // The same seed still gives the same next decision.
    let mut e = Exploration::new(Policy::EpsilonGreedy);
    e.set_parameter(ParameterName::Epsilon, 1.0).unwrap();
    let a = run_preference_semantics(&s, &e, &mut SoarRng::new(5), &mut None);
    let b = run_preference_semantics(&s, &e, &mut SoarRng::new(5), &mut None);
    assert_eq!(a, b);
}

#[test]
fn non_context_semantics_ignores_prohibits() {
    let s = slot(&[
        acc(1),
        acc(2),
        acc(3),
        Preference::unary(Reject, 1),
        Preference::unary(Prohibit, 2),
    ]);
    assert_eq!(run_non_context_preference_semantics(&s), vec![3, 2]);
}
