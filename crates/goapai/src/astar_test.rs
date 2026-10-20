// SPDX-License-Identifier: AGPL-3.0-only
// Translated from goapai `astar_test.go` (commit 2e86c5e, Apache-2.0,
// Copyright 2024 Agoune Ramine).

use super::*;
use crate::action::{Arithmetic::*, Effect, EffectBool};
use crate::agent::{create_agent, set_state, Agent, Goals};
use crate::state::{Condition, ConditionBool, ConditionInterface, Operator::*, State, Value};
use alloc::vec;

fn agent() -> Agent {
    create_agent(Goals::new(), Actions::new())
}

fn goal(conditions: Vec<ConditionInterface>) -> GoalInterface {
    GoalInterface::new(conditions, |_| 1.0)
}

fn names(plan: &Plan) -> Vec<&str> {
    plan.iter().map(|a| a.get_name()).collect()
}

fn named(name: &str, repeatable: bool) -> Arc<Action> {
    Arc::new(Action::new(name, 1.0, repeatable, vec![], vec![]))
}

fn chain(actions: &[Arc<Action>]) -> Vec<Node> {
    let mut nodes = Vec::new();
    for (i, a) in actions.iter().enumerate() {
        let mut n = Node::with_cost(0.0);
        n.action = Arc::clone(a);
        n.parent = i.checked_sub(1);
        n.depth = i as u32 + 1;
        nodes.push(n);
    }
    nodes
}

#[test]
fn get_impacting_actions_filters_no_ops() {
    let mut a = agent();
    set_state(&mut a, 1, 100_i64);
    set_state(&mut a, 2, false);
    let mut actions = Actions::new();
    actions.add_action(
        "change",
        1.0,
        false,
        vec![],
        vec![Effect::new(1, Set, 200_i64).into()],
    );
    actions.add_action(
        "no_change",
        1.0,
        false,
        vec![],
        vec![
            Effect::new(1, Set, 100_i64).into(),
            EffectBool {
                key: 2,
                value: false,
                operator: Set,
            }
            .into(),
        ],
    );
    let impacting = get_impacting_actions(&a.w, &actions);
    assert_eq!(impacting.len(), 1);
    assert_eq!(impacting[0].name, "change");
}

#[test]
fn get_impacting_actions_all_impacting() {
    let mut a = agent();
    set_state(&mut a, 1, 100_i64);
    let mut actions = Actions::new();
    actions.add_action(
        "action1",
        1.0,
        false,
        vec![],
        vec![Effect::new(1, Set, 200_i64).into()],
    );
    actions.add_action(
        "action2",
        1.0,
        false,
        vec![],
        vec![Effect::new(1, Set, 50_i64).into()],
    );
    assert_eq!(get_impacting_actions(&a.w, &actions).len(), 2);
}

#[test]
fn build_plan_from_node_chain() {
    let nodes = chain(&[
        named("action1", false),
        named("action2", false),
        named("action3", false),
    ]);
    let plan = build_plan_from_node(&nodes, 2);
    assert_eq!(names(&plan), ["action1", "action2", "action3"]);
}

#[test]
fn build_plan_from_node_single() {
    let nodes = chain(&[named("action1", false)]);
    assert_eq!(build_plan_from_node(&nodes, 0).len(), 1);
}

#[test]
fn simulate_action_state_applies_effects() {
    let mut a = agent();
    set_state(&mut a, 1, 100_i64);
    let action = Action::new(
        "",
        0.0,
        false,
        vec![],
        vec![Effect::new(1, Add, 50_i64).into()],
    );
    let w = simulate_action_state(&action, &a.w).expect("simulation succeeds");
    assert_eq!(w.get(1).unwrap().value, Value::Int(150));
}

#[test]
fn simulate_action_state_no_change() {
    let mut a = agent();
    set_state(&mut a, 1, 100_i64);
    let action = Action::new(
        "",
        0.0,
        false,
        vec![],
        vec![Effect::new(1, Set, 100_i64).into()],
    );
    assert!(simulate_action_state(&action, &a.w).is_none());
}

#[test]
fn allowed_repetition_repeatable() {
    let action = named("test", true);
    let nodes = chain(&[Arc::clone(&action)]);
    assert!(allowed_repetition(&nodes, &action, 0));
}

#[test]
fn allowed_repetition_non_repeatable_not_used() {
    let nodes = chain(&[named("action2", false)]);
    assert!(allowed_repetition(&nodes, &named("action1", false), 0));
}

#[test]
fn allowed_repetition_non_repeatable_already_used() {
    let action = named("test", false);
    let nodes = chain(&[Arc::clone(&action), named("other", false)]);
    assert!(!allowed_repetition(&nodes, &action, 1));
}

fn missing(state1: i64, state2: bool) -> usize {
    let mut a = agent();
    set_state(&mut a, 1, state1);
    set_state(&mut a, 2, state2);
    let g = goal(vec![
        Condition::new(1, Equal, 100_i64).into(),
        ConditionBool {
            key: 2,
            value: true,
            operator: Equal,
        }
        .into(),
    ]);
    count_missing_goal(&g, &a.w, &a.sensors)
}

#[test]
fn count_missing_goal_all_met() {
    assert_eq!(missing(100, true), 0);
}

#[test]
fn count_missing_goal_one_missing() {
    assert_eq!(missing(100, false), 1);
}

#[test]
fn count_missing_goal_all_missing() {
    assert_eq!(missing(0, false), 2);
}

#[test]
fn compute_heuristic_unmet() {
    let mut current = agent();
    set_state(&mut current, 1, 50_i64);
    let g = goal(vec![Condition::new(1, Equal, 100_i64).into()]);
    assert!(compute_heuristic(&g, &current.w, &current.sensors) > 0.0);
}

#[test]
fn compute_heuristic_goal_met() {
    let mut current = agent();
    set_state(&mut current, 1, 100_i64);
    let g = goal(vec![Condition::new(1, Equal, 100_i64).into()]);
    assert_eq!(compute_heuristic(&g, &current.w, &current.sensors), 0.0);
}

fn run(a: &Agent, g: &GoalInterface, actions: &Actions, depth: u16) -> Plan {
    astar(&a.w, g, actions, depth, &a.sensors)
}

#[test]
fn astar_simple_goal() {
    let mut a = agent();
    set_state(&mut a, 1, 0_i64);
    let mut actions = Actions::new();
    actions.add_action(
        "increment",
        1.0,
        true,
        vec![],
        vec![Effect::new(1, Add, 10_i64).into()],
    );
    let g = goal(vec![Condition::new(1, Equal, 30_i64).into()]);
    // Plan includes the root node with empty action
    assert_eq!(run(&a, &g, &actions, 10).len(), 4);
}

#[test]
fn astar_unreachable_goal() {
    let mut a = agent();
    set_state(&mut a, 1, 0_i64);
    let g = goal(vec![Condition::new(1, Equal, 100_i64).into()]);
    assert_eq!(run(&a, &g, &Actions::new(), 10).len(), 0);
}

#[test]
fn astar_max_depth() {
    let mut a = agent();
    set_state(&mut a, 1, 0_i64);
    let mut actions = Actions::new();
    actions.add_action(
        "increment",
        1.0,
        true,
        vec![],
        vec![Effect::new(1, Add, 1_i64).into()],
    );
    let g = goal(vec![Condition::new(1, Equal, 100_i64).into()]);
    assert!(run(&a, &g, &actions, 5).len() <= 5);
}

#[test]
fn astar_already_at_goal() {
    let mut a = agent();
    set_state(&mut a, 1, 100_i64);
    let g = goal(vec![Condition::new(1, Equal, 100_i64).into()]);
    // Plan includes root node with empty action when already at goal
    assert_eq!(run(&a, &g, &Actions::new(), 10).len(), 1);
}

#[test]
fn astar_prefer_lower_cost() {
    let mut a = agent();
    set_state(&mut a, 1, 0_i64);
    let mut actions = Actions::new();
    actions.add_action(
        "expensive",
        10.0,
        false,
        vec![],
        vec![Effect::new(1, Set, 100_i64).into()],
    );
    actions.add_action(
        "cheap",
        1.0,
        false,
        vec![],
        vec![Effect::new(1, Set, 100_i64).into()],
    );
    let g = goal(vec![Condition::new(1, Equal, 100_i64).into()]);
    let plan = run(&a, &g, &actions, 10);
    assert_eq!(names(&plan), ["", "cheap"]);
}

#[test]
fn astar_respect_conditions() {
    let mut a = agent();
    set_state(&mut a, 1, 0_i64);
    set_state(&mut a, 2, false);
    let mut actions = Actions::new();
    actions.add_action(
        "conditional",
        1.0,
        false,
        vec![ConditionBool {
            key: 2,
            value: true,
            operator: Equal,
        }
        .into()],
        vec![Effect::new(1, Set, 100_i64).into()],
    );
    actions.add_action(
        "enabler",
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
    let g = goal(vec![Condition::new(1, Equal, 100_i64).into()]);
    assert_eq!(
        names(&run(&a, &g, &actions, 10)),
        ["", "enabler", "conditional"]
    );
}

#[test]
fn astar_non_repeatable_actions() {
    let mut a = agent();
    set_state(&mut a, 1, 0_i64);
    let mut actions = Actions::new();
    actions.add_action(
        "increment",
        1.0,
        false,
        vec![],
        vec![Effect::new(1, Add, 10_i64).into()],
    );
    let g = goal(vec![Condition::new(1, Equal, 30_i64).into()]);
    // Non-repeatable action can only be used once, so goal is unreachable
    assert_eq!(run(&a, &g, &actions, 10).len(), 0);
}

#[test]
fn astar_data_cloning() {
    let mut a = agent();
    set_state(&mut a, 1, 100_i64);
    let original = a.w.clone();
    let mut actions = Actions::new();
    actions.add_action(
        "modify",
        1.0,
        false,
        vec![],
        vec![Effect::new(1, Set, 200_i64).into()],
    );
    let g = goal(vec![Condition::new(1, Equal, 200_i64).into()]);
    let _ = run(&a, &g, &actions, 10);
    // Original state should not be modified
    assert_eq!(a.w, original);
}

// ---- Tests added in the translation (not in upstream) ----

/// Cycles: two actions that undo each other must not loop forever; the
/// search is bounded by depth.
#[test]
fn astar_cycle_terminates() {
    let mut a = agent();
    set_state(&mut a, 1, false);
    set_state(&mut a, 2, 0_i64);
    let mut actions = Actions::new();
    actions.add_action(
        "on",
        1.0,
        true,
        vec![],
        vec![EffectBool {
            key: 1,
            value: true,
            operator: Set,
        }
        .into()],
    );
    actions.add_action(
        "off",
        1.0,
        true,
        vec![],
        vec![EffectBool {
            key: 1,
            value: false,
            operator: Set,
        }
        .into()],
    );
    let g = goal(vec![Condition::new(2, Equal, 1_i64).into()]);
    assert_eq!(run(&a, &g, &actions, 8).len(), 0);
}

/// Conflicting goal conditions can never both hold.
#[test]
fn astar_conflicting_conditions() {
    let mut a = agent();
    set_state(&mut a, 1, 0_i64);
    let mut actions = Actions::new();
    actions.add_action(
        "inc",
        1.0,
        true,
        vec![],
        vec![Effect::new(1, Add, 1_i64).into()],
    );
    let g = goal(vec![
        Condition::new(1, Upper, 3_i64).into(),
        Condition::new(1, Lower, 2_i64).into(),
    ]);
    assert_eq!(run(&a, &g, &actions, 6).len(), 0);
}

/// Exactly `max_depth` actions are allowed; one more is not.
#[test]
fn astar_depth_limit_is_inclusive() {
    let mut a = agent();
    set_state(&mut a, 1, 0_i64);
    let mut actions = Actions::new();
    actions.add_action(
        "inc",
        1.0,
        true,
        vec![],
        vec![Effect::new(1, Add, 1_i64).into()],
    );
    let g = goal(vec![Condition::new(1, Equal, 3_i64).into()]);
    assert_eq!(run(&a, &g, &actions, 3).len(), 4);
    assert_eq!(run(&a, &g, &actions, 2).len(), 0);
}

/// Worlds that differ only by a key added by an effect are distinct nodes.
/// Upstream did not hash such states (DEVIATIONS.md D1) and could merge
/// them.
#[test]
fn worlds_differing_by_new_key_are_distinct() {
    let mut w1 = World::new();
    w1.store(State::new(1, 0_i64));
    let mut w2 = w1.clone();
    apply_effects(&[Effect::new(2, Add, 5_i64).into()], &mut w2).unwrap();
    assert!(!w1.check(&w2));
}

/// Procedural conditions gate actions using sensors.
#[test]
fn astar_with_condition_fn() {
    let mut a = agent();
    set_state(&mut a, 1, false);
    crate::agent::set_sensor(&mut a, "has_axe", true);
    let mut actions = Actions::new();
    actions.add_action(
        "chop",
        1.0,
        false,
        vec![crate::state::ConditionFn::new(9, |s| *s.get::<bool>("has_axe").unwrap()).into()],
        vec![EffectBool {
            key: 1,
            value: true,
            operator: Set,
        }
        .into()],
    );
    let g = goal(vec![ConditionBool {
        key: 1,
        value: true,
        operator: Equal,
    }
    .into()]);
    assert_eq!(names(&run(&a, &g, &actions, 3)), ["", "chop"]);
}
