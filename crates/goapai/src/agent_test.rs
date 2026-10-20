// SPDX-License-Identifier: AGPL-3.0-only
// Translated from goapai `agent_test.go` (commit 2e86c5e, Apache-2.0,
// Copyright 2024 Agoune Ramine).

use super::*;
use crate::state::{ConditionBool, Operator, Value};
use alloc::vec;

#[test]
fn create_agent_with_goal_and_action() {
    let goals = Goals::new().with(
        "test_goal",
        GoalInterface::new(
            vec![ConditionBool {
                key: 1,
                value: true,
                operator: Operator::Equal,
            }
            .into()],
            |_| 1.0,
        ),
    );
    let mut actions = Actions::new();
    actions.add_action("test_action", 1.0, false, vec![], vec![]);
    let agent = create_agent(goals, actions);
    assert_eq!(agent.actions.len(), 1);
    assert_eq!(agent.goals.len(), 1);
    // Go also checks that sensors are initialised and that world.Agent is
    // non-nil; Rust sensors are always initialised and the world holds no
    // agent pointer (DEVIATIONS.md D9).
    assert!(agent.sensors.is_empty());
}

#[test]
fn set_state_types() {
    let mut a = create_agent(Goals::new(), Actions::new());
    set_state(&mut a, 1, 42_i64);
    assert_eq!(a.w.states().len(), 1);
    assert_eq!(
        (a.w.states()[0].key, &a.w.states()[0].value),
        (1, &Value::Int(42))
    );

    let mut a = create_agent(Goals::new(), Actions::new());
    set_state(&mut a, 2, true);
    assert_eq!(
        (a.w.states()[0].key, &a.w.states()[0].value),
        (2, &Value::Bool(true))
    );

    let mut a = create_agent(Goals::new(), Actions::new());
    set_state(&mut a, 3, "test");
    assert_eq!(
        (a.w.states()[0].key, &a.w.states()[0].value),
        (3, &Value::String("test".into()))
    );

    // multiple world
    let mut a = create_agent(Goals::new(), Actions::new());
    set_state(&mut a, 1, 100_i64);
    set_state(&mut a, 2, false);
    set_state(&mut a, 3, "hello");
    assert_eq!(a.w.states().len(), 3);
}

#[test]
fn set_sensor_values() {
    struct TestEntity {
        health: i64,
    }
    // single sensor
    let mut a = create_agent(Goals::new(), Actions::new());
    set_sensor(&mut a, "entity", TestEntity { health: 100 });
    assert_eq!(a.sensors.len(), 1);
    assert_eq!(a.sensors.get::<TestEntity>("entity").unwrap().health, 100);
    // multiple sensors
    let mut a = create_agent(Goals::new(), Actions::new());
    set_sensor(&mut a, "sensor1", "value1");
    set_sensor(&mut a, "sensor2", 42_i64);
    set_sensor(&mut a, "sensor3", true);
    assert_eq!(a.sensors.len(), 3);
    assert_eq!(a.sensors.get::<&str>("sensor1"), Some(&"value1"));
    assert_eq!(a.sensors.get::<i64>("sensor2"), Some(&42));
    assert_eq!(a.sensors.get::<bool>("sensor3"), Some(&true));
}

// ---- Tests added in the translation (not in upstream) ----

#[test]
fn goals_insert_replaces_in_place() {
    let goals = Goals::new()
        .with("a", GoalInterface::new(vec![], |_| 1.0))
        .with("b", GoalInterface::new(vec![], |_| 2.0))
        .with("a", GoalInterface::new(vec![], |_| 3.0));
    let names: alloc::vec::Vec<&str> = goals.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(names, ["a", "b"]);
    let s = crate::state::Sensors::new();
    assert_eq!((goals.get("a").unwrap().priority_fn)(&s), 3.0);
}
