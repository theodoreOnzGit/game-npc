// SPDX-License-Identifier: AGPL-3.0-only
// Translated from goapai `distance.go` (https://github.com/akmonengine/goapai,
// commit 2e86c5e, Apache-2.0, Copyright 2024 Agoune Ramine). Modified: see
// docs/DEVIATIONS.md.

//! Heuristic distance from a state to a condition (`distance.go`).

use crate::state::{ConditionInterface, Numeric, Operator, State, Value};

impl State {
    /// Distance still to cover for `condition` to hold: 0 when satisfied,
    /// the numeric gap for numeric conditions, 1 for an unmet bool or string
    /// condition. 0 when the key or type does not match, and for procedural
    /// conditions (Go: `State.Distance`).
    pub fn distance(&self, condition: &ConditionInterface) -> f32 {
        if self.key != condition.get_key() {
            return 0.0;
        }
        fn num<T: Numeric>(v: &Value, target: T, op: Operator) -> f32 {
            match T::from_value(v) {
                Some(cur) => calculate_numeric_distance(cur.to_f64(), target.to_f64(), op),
                None => 0.0,
            }
        }
        let binary = |satisfied: bool| if satisfied { 0.0 } else { 1.0 };
        match condition {
            ConditionInterface::Int8(c) => num(&self.value, c.value, c.operator),
            ConditionInterface::Int(c) => num(&self.value, c.value, c.operator),
            ConditionInterface::Uint8(c) => num(&self.value, c.value, c.operator),
            ConditionInterface::Uint64(c) => num(&self.value, c.value, c.operator),
            ConditionInterface::Float64(c) => num(&self.value, c.value, c.operator),
            ConditionInterface::Bool(c) => match (&self.value, c.operator) {
                (Value::Bool(v), Operator::Equal) => binary(*v == c.value),
                (Value::Bool(v), Operator::NotEqual) => binary(*v != c.value),
                _ => 0.0,
            },
            ConditionInterface::String(c) => match (&self.value, c.operator) {
                (Value::String(v), Operator::Equal) => binary(*v == c.value),
                (Value::String(v), Operator::NotEqual) => binary(*v != c.value),
                _ => 0.0,
            },
            ConditionInterface::Fn(_) => 0.0,
        }
    }
}

/// Go: `calculateNumericDistance`. Arithmetic in `f64`, result rounded to
/// `f32`, as upstream.
pub(crate) fn calculate_numeric_distance(current: f64, target: f64, op: Operator) -> f32 {
    match op {
        Operator::Equal => {
            if current < target {
                (target - current) as f32
            } else {
                (current - target) as f32
            }
        }
        Operator::NotEqual => {
            if current == target {
                1.0
            } else {
                0.0
            }
        }
        Operator::UpperOrEqual => {
            if current < target {
                (target - current) as f32
            } else {
                0.0
            }
        }
        Operator::Upper => {
            if current <= target {
                (target - current + 1.0) as f32
            } else {
                0.0
            }
        }
        Operator::LowerOrEqual => {
            if current > target {
                (current - target) as f32
            } else {
                0.0
            }
        }
        Operator::Lower => {
            if current >= target {
                (current - target + 1.0) as f32
            } else {
                0.0
            }
        }
    }
}

#[cfg(test)]
#[path = "distance_test.rs"]
mod distance_test;
