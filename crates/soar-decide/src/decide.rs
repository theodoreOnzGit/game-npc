// SPDX-License-Identifier: AGPL-3.0-only
// Translated from Soar `Core/SoarKernel/src/decision_process/decide.cpp`
// (`decide_context_slot`) (https://github.com/SoarGroup/Soar, commit
// 3fc337a, BSD, Copyright 1995-2012 Carnegie Mellon University, The Regents
// of the University of Michigan, USC/ISI). Modified: see docs/DEVIATIONS.md.

//! Deciding a context slot: one operator, or a typed impasse.

use alloc::vec::Vec;

use crate::exploration::Exploration;
use crate::preference::Slot;
use crate::rng::SoarRng;
use crate::semantics::{run, ImpasseType, Mode};

/// What the impasse is about (`^attribute` on the impasse state).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ImpasseAttribute {
    /// No usable operator proposal at all (Soar: state no-change).
    State,
    /// The operator slot: a tie, conflict or constraint failure among
    /// proposals, or no change while an operator is already selected.
    Operator,
}

/// A decision impasse (`create_new_impasse` / `update_impasse_items`).
#[derive(Clone, Debug, PartialEq)]
pub struct Impasse<V> {
    pub kind: ImpasseType,
    pub attribute: ImpasseAttribute,
    /// The candidates the impasse is over (`^item`). Empty for no-change.
    pub items: Vec<V>,
}

/// The outcome of deciding a context slot.
#[derive(Clone, Debug, PartialEq)]
pub enum Decision<V> {
    Selected(V),
    Impasse(Impasse<V>),
}

/// Decide the operator slot of a state (`decide_context_slot`).
///
/// `has_current_value` says whether the slot already holds an operator
/// (`s->wmes`). It decides the attribute of a no-change impasse: `operator`
/// if something was selected before, `state` otherwise.
pub fn decide_context_slot<V: PartialEq + Copy>(
    slot: &Slot<V>,
    has_current_value: bool,
    exploration: &Exploration,
    rng: &mut SoarRng,
    force: &mut Option<V>,
) -> Decision<V> {
    let sem = run(
        slot,
        Mode::Decide {
            exploration,
            rng,
            force,
        },
    );
    let kind = match sem.impasse {
        ImpasseType::None => match sem.candidates.as_slice() {
            [winner] => return Decision::Selected(*winner),
            // Upstream aborts with a fatal error for more than one winner;
            // preference semantics never returns that in decide mode.
            [] => ImpasseType::NoChange,
            _ => unreachable!("decide mode resolves indifference to one winner"),
        },
        other => other,
    };
    let attribute = if kind == ImpasseType::NoChange && !has_current_value {
        ImpasseAttribute::State
    } else {
        ImpasseAttribute::Operator
    };
    let items = if kind == ImpasseType::NoChange {
        Vec::new()
    } else {
        sem.candidates
    };
    Decision::Impasse(Impasse {
        kind,
        attribute,
        items,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preference::{Preference, PreferenceType as T};
    use alloc::vec;

    fn decide(slot: &Slot<u32>, current: bool) -> Decision<u32> {
        decide_context_slot(
            slot,
            current,
            &Exploration::default(),
            &mut SoarRng::new(1),
            &mut None,
        )
    }

    #[test]
    fn empty_slot_is_state_no_change() {
        let d = decide(&Slot::new(), false);
        assert_eq!(
            d,
            Decision::Impasse(Impasse {
                kind: ImpasseType::NoChange,
                attribute: ImpasseAttribute::State,
                items: vec![]
            })
        );
    }

    #[test]
    fn no_candidates_with_current_operator_is_operator_no_change() {
        let mut s = Slot::new();
        s.push(Preference::unary(T::Reject, 1u32));
        match decide(&s, true) {
            Decision::Impasse(i) => {
                assert_eq!(i.kind, ImpasseType::NoChange);
                assert_eq!(i.attribute, ImpasseAttribute::Operator);
            }
            d => panic!("{d:?}"),
        }
    }

    #[test]
    fn single_acceptable_is_selected() {
        let mut s = Slot::new();
        s.push(Preference::unary(T::Acceptable, 7u32));
        assert_eq!(decide(&s, false), Decision::Selected(7));
    }

    #[test]
    fn tie_carries_items() {
        let mut s = Slot::new();
        s.push(Preference::unary(T::Acceptable, 1u32));
        s.push(Preference::unary(T::Acceptable, 2u32));
        match decide(&s, false) {
            Decision::Impasse(i) => {
                assert_eq!(i.kind, ImpasseType::Tie);
                assert_eq!(i.attribute, ImpasseAttribute::Operator);
                assert_eq!(i.items, vec![2, 1]);
            }
            d => panic!("{d:?}"),
        }
    }
}
