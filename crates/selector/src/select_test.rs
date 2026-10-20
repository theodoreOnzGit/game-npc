// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use crate::registry::{Cost, SubroutineSpec};
use crate::task::{InputKind, OutputKind, ReasoningTask as T};
use alloc::vec;

fn spec(task: T, accepts: InputKind, cost: u32) -> SubroutineSpec {
    SubroutineSpec {
        task,
        accepts,
        returns: OutputKind::Path,
        cost: Cost::Constant(cost),
        precondition: None,
    }
}

/// Graph analysis (cost 10), pathfinding (cost 10), goal planning (cost 30)
/// all accept graphs; epistemic evaluation accepts propositions.
fn registry() -> Registry {
    let mut r = Registry::new();
    r.register(spec(T::GoalPlanning, InputKind::Graph, 30))
        .unwrap();
    r.register(spec(T::Pathfinding, InputKind::Graph, 10))
        .unwrap();
    r.register(spec(T::GraphAnalysis, InputKind::Graph, 10))
        .unwrap();
    r.register(spec(T::EpistemicEvaluation, InputKind::Proposition, 1))
        .unwrap();
    r
}

fn det() -> DeterministicSelector {
    DeterministicSelector {
        registry: registry(),
    }
}

fn graph() -> ReasoningRequest {
    ReasoningRequest::new(InputKind::Graph, 3)
}

#[test]
fn cheapest_wins_and_equal_cost_goes_to_lowest_id() {
    // Graph analysis (0x03) and pathfinding (0x05) both cost 10.
    assert_eq!(det().select(&graph()), Ok(T::GraphAnalysis));
}

#[test]
fn prohibit_removes_a_candidate() {
    let mut r = graph();
    r.prohibit = vec![T::GraphAnalysis];
    assert_eq!(det().select(&r), Ok(T::Pathfinding));
    r.prohibit = vec![T::GraphAnalysis, T::Pathfinding, T::GoalPlanning];
    assert_eq!(det().select(&r), Err(SelectionError::NoApplicableRoutine));
}

#[test]
fn require_selects_an_applicable_routine_even_if_dearer() {
    let mut r = graph();
    r.require = Some(T::GoalPlanning);
    assert_eq!(det().select(&r), Ok(T::GoalPlanning));
}

#[test]
fn require_of_an_incompatible_routine_is_invalid_input() {
    let mut r = graph();
    r.require = Some(T::EpistemicEvaluation);
    assert_eq!(det().select(&r), Err(SelectionError::InvalidInput));
    r.require = Some(T::MathematicalCalculation); // not registered
    assert_eq!(det().select(&r), Err(SelectionError::InvalidInput));
}

#[test]
fn require_plus_prohibit_is_ambiguous() {
    let mut r = graph();
    r.require = Some(T::Pathfinding);
    r.prohibit = vec![T::Pathfinding];
    let t = det().select_traced(&r);
    assert_eq!(t.result, Err(SelectionError::AmbiguousSelection));
    assert_eq!(t.impasse, Some(ImpasseType::ConstraintFailure));
}

#[test]
fn caller_preference_overrides_cost() {
    let mut r = graph();
    r.prefer = vec![
        (T::GoalPlanning, T::GraphAnalysis),
        (T::GoalPlanning, T::Pathfinding),
    ];
    assert_eq!(det().select(&r), Ok(T::GoalPlanning));
}

#[test]
fn preference_cycle_is_ambiguous() {
    let mut r = graph();
    r.prohibit = vec![T::GoalPlanning];
    r.prefer = vec![
        (T::GraphAnalysis, T::Pathfinding),
        (T::Pathfinding, T::GraphAnalysis),
    ];
    let t = det().select_traced(&r);
    assert_eq!(t.result, Err(SelectionError::AmbiguousSelection));
    assert_eq!(t.impasse, Some(ImpasseType::Conflict));
}

#[test]
fn budget_excludes_expensive_routines() {
    let mut r = graph();
    r.require = Some(T::GoalPlanning);
    r.budget = Some(20);
    assert_eq!(det().select(&r), Err(SelectionError::InvalidInput));
    r.require = None;
    assert_eq!(det().select(&r), Ok(T::GraphAnalysis));
}

#[test]
fn no_routine_for_unhandled_input() {
    let r = ReasoningRequest::new(InputKind::SpatialMap, 1);
    let t = det().select_traced(&r);
    assert_eq!(t.result, Err(SelectionError::NoApplicableRoutine));
    assert!(t
        .considered
        .iter()
        .all(|(_, a)| *a == Applicability::WrongInput));
}

/// A model that loves epistemic evaluation and has a tie at the top
/// otherwise.
struct Model;
impl UtilityModel for Model {
    fn utility(&self, t: T, _: &ReasoningRequest) -> f64 {
        match t {
            T::EpistemicEvaluation => 1e9,
            T::GoalPlanning | T::Pathfinding => 0.8,
            _ => 0.1,
        }
    }
}

fn util(seed: u32) -> UtilitySelector<Model> {
    UtilitySelector {
        registry: registry(),
        model: Model,
        seed,
    }
}

#[test]
fn utility_cannot_select_an_incompatible_routine() {
    // Epistemic evaluation has huge utility but does not accept graphs.
    let t = util(1).select_traced(&graph());
    assert!(matches!(t.result, Ok(T::GoalPlanning) | Ok(T::Pathfinding)));
    assert!(t
        .utilities
        .iter()
        .all(|(task, _)| *task != T::EpistemicEvaluation));
}

#[test]
fn utility_ties_are_broken_by_seed_reproducibly() {
    let mut seen = vec![];
    for seed in 0..40 {
        let a = util(seed).select(&graph());
        assert_eq!(a, util(seed).select(&graph()), "seed {seed}");
        seen.push(a.unwrap());
    }
    assert!(seen.contains(&T::GoalPlanning) && seen.contains(&T::Pathfinding));
    assert!(!seen.contains(&T::GraphAnalysis));
}

#[test]
fn non_finite_utility_is_invalid_input() {
    struct Nan;
    impl UtilityModel for Nan {
        fn utility(&self, _: T, _: &ReasoningRequest) -> f64 {
            f64::NAN
        }
    }
    let s = UtilitySelector {
        registry: registry(),
        model: Nan,
        seed: 0,
    };
    assert_eq!(s.select(&graph()), Err(SelectionError::InvalidInput));
}

#[test]
fn traces_replay() {
    let mut r = graph();
    r.prefer = vec![(T::Pathfinding, T::GraphAnalysis)];
    let t = det().select_traced(&r);
    assert_eq!(t.result, Ok(T::Pathfinding));
    assert!(replay(&det(), &t));
    let u = util(7).select_traced(&r);
    assert!(replay(&util(7), &u));
    // A different model or seed does not replay the same trace.
    let other = (0..40)
        .map(util)
        .find(|s| s.select(&r) != u.result)
        .expect("some seed differs");
    assert!(!replay(&other, &u));
}

/// Property check over many random requests: whatever comes out is
/// applicable, not prohibited, equals `require` when one is given, and
/// replays.
#[test]
fn selected_routine_is_always_admissible() {
    let mut state: u64 = 0x5e1ec7;
    let mut next = |k: u64| {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (state >> 33) % k
    };
    let kinds = [
        InputKind::Graph,
        InputKind::Proposition,
        InputKind::SpatialMap,
    ];
    let mut selected = 0;
    for _ in 0..3000 {
        let mut r = ReasoningRequest::new(kinds[next(3) as usize], next(50) as u32);
        if next(3) == 0 {
            r.budget = Some(next(40) as u32);
        }
        for t in T::ALL {
            if next(5) == 0 {
                r.prohibit.push(t);
            }
        }
        if next(6) == 0 {
            r.require = Some(T::ALL[next(5) as usize]);
        }
        for _ in 0..next(3) {
            r.prefer
                .push((T::ALL[next(5) as usize], T::ALL[next(5) as usize]));
        }
        for trace in [
            det().select_traced(&r),
            util(next(1000) as u32).select_traced(&r),
        ] {
            if let Ok(t) = trace.result {
                selected += 1;
                let app = trace.considered.iter().find(|(x, _)| *x == t).unwrap().1;
                assert_eq!(app, Applicability::Applicable, "{r:?}");
                assert!(!r.prohibit.contains(&t), "{r:?}");
                if let Some(req) = r.require {
                    assert_eq!(t, req, "{r:?}");
                }
            }
        }
        assert!(replay(&det(), &det().select_traced(&r)));
    }
    assert!(selected > 1000, "only {selected} selections");
}
