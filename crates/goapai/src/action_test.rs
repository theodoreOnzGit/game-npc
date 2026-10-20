// SPDX-License-Identifier: AGPL-3.0-only
// Translated from goapai `action_test.go` (commit 2e86c5e, Apache-2.0,
// Copyright 2024 Agoune Ramine).

use super::*;
use crate::agent::{create_agent, set_state, Goals};
use alloc::vec;

#[test]
fn actions_add_action() {
    // single action
    let mut actions = Actions::new();
    actions.add_action("test", 1.5, false, vec![], vec![]);
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].name, "test");
    assert_eq!(actions[0].cost, 1.5);
    assert!(!actions[0].repeatable);
    // multiple actions
    let mut actions = Actions::new();
    actions.add_action("action1", 1.0, true, vec![], vec![]);
    actions.add_action("action2", 2.0, false, vec![], vec![]);
    assert_eq!(actions.len(), 2);
}

#[test]
fn action_get_name() {
    let mut actions = Actions::new();
    actions.add_action("my_action", 1.0, false, vec![], vec![]);
    assert_eq!(actions[0].get_name(), "my_action");
}

#[test]
fn action_get_effects() {
    let mut actions = Actions::new();
    actions.add_action(
        "test",
        1.0,
        false,
        vec![],
        vec![Effect::new(1, Arithmetic::Set, 10_i64).into()],
    );
    assert_eq!(actions[0].get_effects().len(), 1);
}

#[test]
fn effect_check() {
    let cases = [
        (
            "match with SET operator",
            100_i64,
            100_i64,
            Arithmetic::Set,
            true,
        ),
        (
            "no match with SET operator",
            100,
            200,
            Arithmetic::Set,
            false,
        ),
        (
            "non-SET operator always false",
            100,
            100,
            Arithmetic::Add,
            false,
        ),
    ];
    for (name, state_value, effect_val, op, want) in cases {
        let mut agent = create_agent(Goals::new(), Actions::new());
        set_state(&mut agent, 1, state_value);
        let effect = Effect::new(1, op, effect_val);
        assert_eq!(effect.check(&agent.w), want, "{name}");
    }
}
