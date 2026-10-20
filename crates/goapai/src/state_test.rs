// SPDX-License-Identifier: AGPL-3.0-only
// Translated from goapai `state_test.go` (commit 2e86c5e, Apache-2.0,
// Copyright 2024 Agoune Ramine).

use super::*;
use crate::action::Actions;
use crate::agent::{create_agent, set_sensor, set_state, Goals};

fn agent() -> crate::Agent {
    create_agent(Goals::new(), Actions::new())
}

// TestState_Operations
#[test]
fn state_get_key() {
    let state = State::new(42, 100_i64);
    assert_eq!(state.get_key(), 42);
}

#[test]
fn state_get_value() {
    let state = State::new(1, 100_i64);
    assert_eq!(state.get_value(), &Value::Int(100));
}

#[test]
fn state_check_match() {
    let mut a = agent();
    set_state(&mut a, 1, 100_i64);
    assert!(State::new(1, 100_i64).check(&a.w, 1));
}

#[test]
fn state_check_no_match() {
    let mut a = agent();
    set_state(&mut a, 1, 100_i64);
    assert!(!State::new(1, 200_i64).check(&a.w, 1));
}

#[test]
fn state_check_key_not_found() {
    let a = agent();
    assert!(!State::new(99, 100_i64).check(&a.w, 99));
}

#[test]
fn condition_operators() {
    use Operator::*;
    let cases: [(&str, i64, i64, Operator, bool); 16] = [
        ("equal match", 100, 100, Equal, true),
        ("equal no match", 100, 99, Equal, false),
        ("not equal match", 100, 99, NotEqual, true),
        ("not equal no match", 100, 100, NotEqual, false),
        ("lower true", 50, 100, Lower, true),
        ("lower false equal", 50, 50, Lower, false),
        ("lower false greater", 100, 50, Lower, false),
        ("lower or equal true less", 50, 100, LowerOrEqual, true),
        ("lower or equal true equal", 50, 50, LowerOrEqual, true),
        ("lower or equal false", 100, 50, LowerOrEqual, false),
        ("upper true", 100, 50, Upper, true),
        ("upper false equal", 100, 100, Upper, false),
        ("upper false less", 50, 100, Upper, false),
        ("upper or equal true greater", 100, 50, UpperOrEqual, true),
        ("upper or equal true equal", 100, 100, UpperOrEqual, true),
        ("upper or equal false", 50, 100, UpperOrEqual, false),
    ];
    for (name, state_val, cond_val, op, want) in cases {
        let mut a = agent();
        set_state(&mut a, 1, state_val);
        let c = Condition::new(1, op, cond_val);
        assert_eq!(c.check(&a.w), want, "{name}");
    }
}

#[test]
fn condition_key_not_found() {
    let a = agent();
    assert!(!Condition::new(99, Operator::Equal, 100_i64).check(&a.w));
}

#[test]
fn condition_bool() {
    use Operator::*;
    let cases = [
        ("equal true match", true, true, Equal, true),
        ("equal false match", false, false, Equal, true),
        ("equal no match", true, false, Equal, false),
        ("not equal match", true, false, NotEqual, true),
        ("not equal no match", true, true, NotEqual, false),
    ];
    for (name, state_val, cond_val, operator, want) in cases {
        let mut a = agent();
        set_state(&mut a, 1, state_val);
        let c = ConditionBool {
            key: 1,
            value: cond_val,
            operator,
        };
        assert_eq!(c.check(&a.w), want, "{name}");
    }
    // key not found
    let a = agent();
    let c = ConditionBool {
        key: 99,
        value: true,
        operator: Equal,
    };
    assert!(!c.check(&a.w));
}

#[test]
fn condition_string() {
    use Operator::*;
    let cases = [
        ("equal match", "test", "test", Equal, true),
        ("equal no match", "test", "other", Equal, false),
        ("not equal match", "test", "other", NotEqual, true),
        ("not equal no match", "test", "test", NotEqual, false),
    ];
    for (name, state_val, cond_val, operator, want) in cases {
        let mut a = agent();
        set_state(&mut a, 1, state_val);
        let c = ConditionString {
            key: 1,
            value: cond_val.into(),
            operator,
        };
        assert_eq!(c.check(&a.w), want, "{name}");
    }
}

#[test]
fn condition_fn() {
    let cases = [
        ("above threshold", 100_i64, 50_i64, true),
        ("below threshold", 30, 50, false),
        ("at threshold", 50, 50, false),
    ];
    for (name, sensor_val, threshold, want) in cases {
        let mut a = agent();
        set_sensor(&mut a, "value", sensor_val);
        let c = ConditionFn::new(1, move |s: &Sensors| {
            *s.get::<i64>("value").unwrap() > threshold
        });
        assert_eq!(c.check(&a.sensors), want, "{name}");
        // Test caching
        assert!(c.resolved(), "{name}: expected resolved");
        assert_eq!(c.check(&a.sensors), want, "{name}: cached result");
    }
}

#[test]
fn conditions_check() {
    // all match
    let mut a = agent();
    set_state(&mut a, 1, 100_i64);
    set_state(&mut a, 2, true);
    let all: Conditions = alloc::vec![
        Condition::new(1, Operator::Equal, 100_i64).into(),
        ConditionBool {
            key: 2,
            value: true,
            operator: Operator::Equal
        }
        .into(),
    ];
    assert!(check_conditions(&all, &a.w, &a.sensors));
    // one fails
    let one_fails: Conditions = alloc::vec![
        Condition::new(1, Operator::Equal, 100_i64).into(),
        ConditionBool {
            key: 2,
            value: false,
            operator: Operator::Equal
        }
        .into(),
    ];
    assert!(!check_conditions(&one_fails, &a.w, &a.sensors));
    // empty conditions
    let empty = agent();
    assert!(check_conditions(&[], &empty.w, &empty.sensors));
}

#[test]
fn sensors_get_sensor() {
    let mut sensors = Sensors::new();
    sensors.set("test", 42_i64);
    assert_eq!(sensors.get::<i64>("test"), Some(&42));
    assert!(sensors.get_sensor("missing").is_none());
}

// Upstream's test values include 3.14 (not meant as PI).
#[allow(clippy::approx_constant)]
#[test]
fn state_hash() {
    let cases: [(&str, State, State, bool); 11] = [
        (
            "same int world",
            State::new(1, 100_i64),
            State::new(1, 100_i64),
            true,
        ),
        (
            "different int values",
            State::new(1, 100_i64),
            State::new(1, 200_i64),
            false,
        ),
        (
            "different keys",
            State::new(1, 100_i64),
            State::new(2, 100_i64),
            false,
        ),
        (
            "same bool world",
            State::new(1, true),
            State::new(1, true),
            true,
        ),
        (
            "different bool values",
            State::new(1, true),
            State::new(1, false),
            false,
        ),
        (
            "same string world",
            State::new(1, "test"),
            State::new(1, "test"),
            true,
        ),
        (
            "different string values",
            State::new(1, "test"),
            State::new(1, "other"),
            false,
        ),
        (
            "same float64 world",
            State::new(1, 3.14_f64),
            State::new(1, 3.14_f64),
            true,
        ),
        (
            "different float64 values",
            State::new(1, 3.14_f64),
            State::new(1, 2.71_f64),
            false,
        ),
        (
            "same uint64 world",
            State::new(1, 12345_u64),
            State::new(1, 12345_u64),
            true,
        ),
        (
            "different uint64 values",
            State::new(1, 12345_u64),
            State::new(1, 54321_u64),
            false,
        ),
    ];
    for (name, s1, s2, want_same) in cases {
        assert_eq!(s1.hash() == s2.hash(), want_same, "{name}");
    }
}

// ---- Tests added in the translation (not in upstream) ----

/// The hash constants and mixing reproduce Go's values exactly; reference
/// values printed by the Go implementation at commit 2e86c5e.
#[test]
fn hash_matches_go_reference_values() {
    assert_eq!(State::new(1, 100_i64).hash(), GO_HASH_INT_1_100);
    assert_eq!(State::new(7, -3_i8).hash(), GO_HASH_INT8_7_NEG3);
    assert_eq!(State::new(2, "abc").hash(), GO_HASH_STR_2_ABC);
    assert_eq!(State::new(3, true).hash(), GO_HASH_BOOL_3_TRUE);
    assert_eq!(State::new(4, 1.5_f64).hash(), GO_HASH_F64_4_1_5);
}

const GO_HASH_INT_1_100: u64 = 10663246675681842889;
const GO_HASH_INT8_7_NEG3: u64 = 16457174875346852480;
const GO_HASH_STR_2_ABC: u64 = 13368940366949268758;
const GO_HASH_BOOL_3_TRUE: u64 = 1735226235737055088;
const GO_HASH_F64_4_1_5: u64 = 2113849475589599316;

/// The incremental world hash always equals the XOR of its states' hashes,
/// including for states added by effects on new keys (upstream skipped
/// those; DEVIATIONS.md D1).
#[test]
fn world_hash_is_xor_of_states() {
    let mut w = World::new();
    w.store(State::new(1, 5_i64));
    w.store(State::new(2, "x"));
    w.store(State::new(1, 6_i64));
    let xor = w.states().iter().fold(0, |h, s| h ^ s.hash());
    assert_eq!(w.hash(), xor);
    assert_eq!(w.states().len(), 2);
}

#[test]
fn set_state_updates_existing_key() {
    let mut a = agent();
    set_state(&mut a, 1, 1_i64);
    set_state(&mut a, 1, 2_i64);
    assert_eq!(a.w.states().len(), 1);
    assert_eq!(a.w.get(1).unwrap().value, Value::Int(2));
}

#[test]
fn condition_type_must_match_state_type() {
    let mut a = agent();
    set_state(&mut a, 1, 100_i8);
    assert!(!Condition::new(1, Operator::Equal, 100_i64).check(&a.w));
    assert!(Condition::new(1, Operator::Equal, 100_i8).check(&a.w));
}

#[test]
fn bool_and_string_reject_ordering_operators() {
    let mut a = agent();
    set_state(&mut a, 1, true);
    set_state(&mut a, 2, "b");
    for op in [
        Operator::Lower,
        Operator::LowerOrEqual,
        Operator::Upper,
        Operator::UpperOrEqual,
    ] {
        assert!(!ConditionBool {
            key: 1,
            value: true,
            operator: op
        }
        .check(&a.w));
        assert!(!ConditionString {
            key: 2,
            value: "b".into(),
            operator: op
        }
        .check(&a.w));
    }
}

#[test]
fn nan_states_never_compare_equal() {
    let mut a = agent();
    set_state(&mut a, 1, f64::NAN);
    assert!(!Condition::new(1, Operator::Equal, f64::NAN).check(&a.w));
    assert!(Condition::new(1, Operator::NotEqual, f64::NAN).check(&a.w));
}
