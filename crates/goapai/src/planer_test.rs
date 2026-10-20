// SPDX-License-Identifier: AGPL-3.0-only
// Translated from goapai `planer_test.go` (commit 2e86c5e, Apache-2.0,
// Copyright 2024 Agoune Ramine).

use super::*;
use crate::action::{Actions, Arithmetic::*, Effect, EffectBool};
use crate::agent::{create_agent, set_sensor, set_state, GoalInterface, Goals};
use crate::state::{Condition, ConditionBool, ConditionFn, Operator::*};
use alloc::vec;

fn names(plan: &Plan) -> Vec<&str> {
    plan.iter().map(|a| a.get_name()).collect()
}

#[test]
fn plan_get_total_cost() {
    let a = |name: &str, cost| Arc::new(Action::new(name, cost, false, vec![], vec![]));
    assert_eq!(Plan(vec![]).get_total_cost(), 0.0);
    assert_eq!(Plan(vec![a("action 1", 1.0)]).get_total_cost(), 1.0);
    assert_eq!(
        Plan(vec![a("action 1", 1.0), a("action 2", 2.0)]).get_total_cost(),
        3.0
    );
}

#[test]
fn get_plan_simple_goal() {
    let mut actions = Actions::new();
    actions.add_action(
        "increment",
        1.0,
        true,
        vec![],
        vec![Effect::new(1, Add, 10_i64).into()],
    );
    let goals = Goals::new().with(
        "reach_30",
        GoalInterface::new(vec![Condition::new(1, Equal, 30_i64).into()], |_| 1.0),
    );
    let mut agent = create_agent(goals, actions);
    set_state(&mut agent, 1, 0_i64);
    let (goal_name, plan) = get_plan(&agent, 10);
    assert_eq!(goal_name, "reach_30");
    // Plan includes root node
    assert_eq!(plan.len(), 4);
}

#[test]
fn get_plan_already_at_goal() {
    let goals = Goals::new().with(
        "be_at_100",
        GoalInterface::new(vec![Condition::new(1, Equal, 100_i64).into()], |_| 1.0),
    );
    let mut agent = create_agent(goals, Actions::new());
    set_state(&mut agent, 1, 100_i64);
    let (goal_name, plan) = get_plan(&agent, 10);
    assert_eq!(goal_name, "be_at_100");
    assert_eq!(plan.len(), 1);
}

#[test]
fn get_plan_no_goals_available() {
    let goals = Goals::new().with(
        "impossible",
        GoalInterface::new(vec![Condition::new(1, Equal, 100_i64).into()], |_| 0.0),
    );
    let mut agent = create_agent(goals, Actions::new());
    set_state(&mut agent, 1, 0_i64);
    let (goal_name, plan) = get_plan(&agent, 10);
    assert_eq!(goal_name, "");
    assert_eq!(plan.len(), 0);
}

#[test]
fn get_plan_unreachable_goal() {
    let goals = Goals::new().with(
        "unreachable",
        GoalInterface::new(vec![Condition::new(1, Equal, 100_i64).into()], |_| 1.0),
    );
    let mut agent = create_agent(goals, Actions::new());
    set_state(&mut agent, 1, 0_i64);
    let (goal_name, plan) = get_plan(&agent, 10);
    assert_eq!(goal_name, "unreachable");
    assert_eq!(plan.len(), 0);
}

#[test]
fn get_plan_max_depth_exceeded() {
    let mut actions = Actions::new();
    actions.add_action(
        "increment",
        1.0,
        true,
        vec![],
        vec![Effect::new(1, Add, 1_i64).into()],
    );
    let goals = Goals::new().with(
        "reach_100",
        GoalInterface::new(vec![Condition::new(1, Equal, 100_i64).into()], |_| 1.0),
    );
    let mut agent = create_agent(goals, actions);
    set_state(&mut agent, 1, 0_i64);
    let (_, plan) = get_plan(&agent, 5);
    assert!(plan.len() <= 5);
}

#[test]
fn get_prioritized_goal_name_single_goal() {
    let goals = Goals::new().with("goal1", GoalInterface::new(vec![], |_| 1.0));
    let agent = create_agent(goals, Actions::new());
    assert_eq!(agent.get_prioritized_goal_name(), Ok("goal1".into()));
}

#[test]
fn get_prioritized_goal_name_multiple_goals() {
    let goals = Goals::new()
        .with("low_priority", GoalInterface::new(vec![], |_| 0.5))
        .with("high_priority", GoalInterface::new(vec![], |_| 2.0))
        .with("medium_priority", GoalInterface::new(vec![], |_| 1.0));
    let agent = create_agent(goals, Actions::new());
    assert_eq!(
        agent.get_prioritized_goal_name(),
        Ok("high_priority".into())
    );
}

#[test]
fn get_prioritized_goal_name_zero_priority() {
    let goals = Goals::new().with("inactive", GoalInterface::new(vec![], |_| 0.0));
    let agent = create_agent(goals, Actions::new());
    assert_eq!(
        agent.get_prioritized_goal_name(),
        Err(GoapError::NoGoalAvailable)
    );
}

#[test]
fn get_prioritized_goal_name_using_sensors() {
    struct Entity {
        health: i64,
    }
    let goals = Goals::new()
        .with(
            "heal",
            GoalInterface::new(vec![], |s| {
                if s.get::<Entity>("entity").unwrap().health < 50 {
                    2.0
                } else {
                    0.1
                }
            }),
        )
        .with("explore", GoalInterface::new(vec![], |_| 1.0));
    let mut agent = create_agent(goals, Actions::new());
    set_sensor(&mut agent, "entity", Entity { health: 20 });
    assert_eq!(agent.get_prioritized_goal_name(), Ok("heal".into()));
}

#[test]
fn get_plan_complex_scenario() {
    let mut actions = Actions::new();
    // Need wood to make fire
    actions.add_action(
        "get_wood",
        2.0,
        false,
        vec![],
        vec![EffectBool {
            key: 1,
            value: true,
            operator: Set,
        }
        .into()],
    );
    // Need matches to make fire
    actions.add_action(
        "get_matches",
        1.0,
        false,
        vec![],
        vec![EffectBool {
            key: 2,
            value: true,
            operator: Set,
        }
        .into()],
    );
    // Make fire requires both wood and matches
    actions.add_action(
        "make_fire",
        1.0,
        false,
        vec![
            ConditionBool {
                key: 1,
                value: true,
                operator: Equal,
            }
            .into(),
            ConditionBool {
                key: 2,
                value: true,
                operator: Equal,
            }
            .into(),
        ],
        vec![EffectBool {
            key: 3,
            value: true,
            operator: Set,
        }
        .into()],
    );
    let goals = Goals::new().with(
        "stay_warm",
        GoalInterface::new(
            vec![ConditionBool {
                key: 3,
                value: true,
                operator: Equal,
            }
            .into()],
            |_| 1.0,
        ),
    );
    let mut agent = create_agent(goals, actions);
    set_state(&mut agent, 1, false);
    set_state(&mut agent, 2, false);
    set_state(&mut agent, 3, false);
    let (goal_name, plan) = get_plan(&agent, 10);
    assert_eq!(goal_name, "stay_warm");
    // Plan includes root + 3 actions
    assert_eq!(plan.len(), 4);
    assert!(plan[1].name == "get_matches" || plan[1].name == "get_wood");
    assert_eq!(plan[3].name, "make_fire");
    // Verify total cost (root has cost 0)
    assert_eq!(plan.get_total_cost(), 4.0);
}

// ---- Tests added in the translation (not in upstream) ----

/// Priority ties go to the goal inserted first (DEVIATIONS.md D5; Go's map
/// order made this random).
#[test]
fn goal_priority_ties_are_deterministic() {
    let goals = Goals::new()
        .with("b", GoalInterface::new(vec![], |_| 1.0))
        .with("a", GoalInterface::new(vec![], |_| 1.0));
    let agent = create_agent(goals, Actions::new());
    for _ in 0..10 {
        assert_eq!(agent.get_prioritized_goal_name(), Ok("b".into()));
    }
}

#[test]
fn nan_priority_is_never_selected() {
    let goals = Goals::new()
        .with("nan", GoalInterface::new(vec![], |_| f32::NAN))
        .with("one", GoalInterface::new(vec![], |_| 1.0));
    let agent = create_agent(goals, Actions::new());
    assert_eq!(agent.get_prioritized_goal_name(), Ok("one".into()));
}

#[test]
fn try_get_plan_reports_no_goal() {
    let agent = create_agent(Goals::new(), Actions::new());
    assert_eq!(
        try_get_plan(&agent, 3).unwrap_err(),
        GoapError::NoGoalAvailable
    );
}

/// get_plan does not change the agent (DEVIATIONS.md D2).
#[test]
fn get_plan_leaves_agent_unchanged() {
    let mut actions = Actions::new();
    actions.add_action(
        "inc",
        1.0,
        true,
        vec![],
        vec![Effect::new(1, Add, 1_i64).into()],
    );
    let goals = Goals::new().with(
        "g",
        GoalInterface::new(vec![Condition::new(1, Equal, 2_i64).into()], |_| 1.0),
    );
    let mut agent = create_agent(goals, actions);
    set_state(&mut agent, 1, 0_i64);
    let before = agent.w.clone();
    let first = names(&get_plan(&agent, 5).1).join(",");
    assert_eq!(agent.w, before);
    let second = names(&get_plan(&agent, 5).1).join(",");
    assert_eq!(first, second);
}

/// Procedural conditions are re-evaluated on every planning request
/// (DEVIATIONS.md D3; upstream cached them forever).
#[test]
fn condition_fn_is_resolved_once_per_request() {
    use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    static ALLOWED: AtomicBool = AtomicBool::new(false);
    static CALLS: AtomicUsize = AtomicUsize::new(0);
    let mut actions = Actions::new();
    actions.add_action(
        "act",
        1.0,
        true,
        vec![ConditionFn::new(9, |_| {
            CALLS.fetch_add(1, Ordering::Relaxed);
            ALLOWED.load(Ordering::Relaxed)
        })
        .into()],
        vec![Effect::new(1, Add, 1_i64).into()],
    );
    let goals = Goals::new().with(
        "g",
        GoalInterface::new(vec![Condition::new(1, Equal, 2_i64).into()], |_| 1.0),
    );
    let mut agent = create_agent(goals, actions);
    set_state(&mut agent, 1, 0_i64);

    assert_eq!(get_plan(&agent, 5).1.len(), 0);
    assert_eq!(CALLS.load(Ordering::Relaxed), 1);
    ALLOWED.store(true, Ordering::Relaxed);
    assert_eq!(names(&get_plan(&agent, 5).1), ["", "act", "act"]);
    assert_eq!(CALLS.load(Ordering::Relaxed), 2);
}

#[test]
fn steps_skip_the_root_action() {
    let mut actions = Actions::new();
    actions.add_action(
        "inc",
        1.0,
        true,
        vec![],
        vec![Effect::new(1, Add, 1_i64).into()],
    );
    let goals = Goals::new().with(
        "g",
        GoalInterface::new(vec![Condition::new(1, Equal, 1_i64).into()], |_| 1.0),
    );
    let mut agent = create_agent(goals, actions);
    set_state(&mut agent, 1, 0_i64);
    let (_, plan) = get_plan(&agent, 5);
    assert_eq!(plan.steps().len(), 1);
    assert_eq!(plan.steps()[0].get_name(), "inc");
    assert!(Plan::default().steps().is_empty());
}
