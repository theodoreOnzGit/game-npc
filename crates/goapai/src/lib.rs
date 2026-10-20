// SPDX-License-Identifier: AGPL-3.0-only
// Translated from goapai (https://github.com/akmonengine/goapai, commit
// 2e86c5e, Apache-2.0, Copyright 2024 Agoune Ramine). Modified: see
// docs/DEVIATIONS.md.

//! # goapai: microlithic GOAP in Rust
//!
//! A translation of the Go library
//! [goapai](https://github.com/akmonengine/goapai). An agent has goals with
//! priority functions and actions with preconditions, effects and costs.
//! [`get_plan`] picks the highest-priority goal and runs a forward A* over
//! world states to find the cheapest sequence of actions that achieves it.
//!
//! The planning algorithm and its tie-breaking order follow upstream, and
//! `tests/differential.rs` checks plans against the Go implementation. The
//! deliberate differences (bug fixes and determinism) are listed in
//! `docs/DEVIATIONS.md`, and the Go-to-Rust mapping in
//! `docs/TRACEABILITY.md`.
//!
//! ```
//! use goapai::*;
//!
//! const HUNGRY: StateKey = 0;
//! const HAS_APPLE: StateKey = 1;
//!
//! let mut actions = Actions::new();
//! actions.add_action("fetch apple", 2.0, true, vec![], vec![
//!     EffectBool { key: HAS_APPLE, value: true, operator: Arithmetic::Set }.into(),
//! ]);
//! actions.add_action("eat", 1.0, true, vec![
//!     ConditionBool { key: HAS_APPLE, value: true, operator: Operator::Equal }.into(),
//!     ConditionBool { key: HUNGRY, value: true, operator: Operator::Equal }.into(),
//! ], vec![
//!     EffectBool { key: HUNGRY, value: false, operator: Arithmetic::Set }.into(),
//!     EffectBool { key: HAS_APPLE, value: false, operator: Arithmetic::Set }.into(),
//! ]);
//!
//! let goals = Goals::new().with("eat", GoalInterface::new(
//!     vec![ConditionBool { key: HUNGRY, value: false, operator: Operator::Equal }.into()],
//!     |sensors| if *sensors.get::<bool>("hungry").unwrap() { 1.0 } else { 0.0 },
//! ));
//!
//! let mut agent = create_agent(goals, actions);
//! set_sensor(&mut agent, "hungry", true);
//! set_state(&mut agent, HUNGRY, true);
//! set_state(&mut agent, HAS_APPLE, false);
//!
//! let (goal, plan) = get_plan(&agent, 10);
//! assert_eq!(goal, "eat");
//! let names: Vec<&str> = plan.steps().iter().map(|a| a.get_name()).collect();
//! assert_eq!(names, ["fetch apple", "eat"]);
//! assert_eq!(plan.get_total_cost(), 3.0);
//! ```
#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

pub mod action;
pub mod agent;
pub(crate) mod astar;
pub mod distance;
pub(crate) mod heap;
pub mod planer;
pub mod state;

pub use action::{
    Action, Actions, Arithmetic, Effect, EffectBool, EffectFn, EffectInterface, EffectString,
    Effects, GoapError,
};
pub use agent::{
    create_agent, set_sensor, set_state, Agent, GoalInterface, GoalName, GoalPriorityFn, Goals,
};
pub use planer::{get_plan, try_get_plan, Plan};
pub use state::{
    check_conditions, CheckFn, Condition, ConditionBool, ConditionFn, ConditionInterface,
    ConditionString, Conditions, Numeric, Operator, Sensor, Sensors, State, StateKey, StateValue,
    Value, World,
};
