// SPDX-License-Identifier: AGPL-3.0-only
// Translated from goapai `effect_test.go` (commit 2e86c5e, Apache-2.0,
// Copyright 2024 Agoune Ramine).

// Table-driven cases mirror upstream's anonymous-struct tables.
#![allow(clippy::type_complexity)]

use super::*;
use crate::agent::{create_agent, set_state, Agent, Goals};
use alloc::string::ToString;

fn agent() -> Agent {
    create_agent(Goals::new(), Actions::new())
}

#[test]
fn effect_string_get_key() {
    let e = EffectString {
        key: 42,
        value: "test".into(),
        operator: Arithmetic::Set,
    };
    assert_eq!(e.get_key(), 42);
}

#[test]
fn effect_string_check() {
    let cases = [
        ("SET match", "hello", 1, "hello", true),
        ("SET no match", "hello", 1, "world", false),
        ("key not found", "hello", 99, "test", false),
    ];
    for (name, state_val, key, effect_val, want) in cases {
        let mut a = agent();
        set_state(&mut a, 1, state_val);
        let e = EffectString {
            key,
            value: effect_val.into(),
            operator: Arithmetic::Set,
        };
        assert_eq!(e.check(&a.w), want, "{name}");
    }
}

#[test]
fn effect_string_apply() {
    use Arithmetic::*;
    let cases: [(&str, Option<&str>, Arithmetic, &str, Option<&str>); 7] = [
        ("SET new state", None, Set, "hello", Some("hello")),
        ("SET existing state", Some("old"), Set, "new", Some("new")),
        (
            "ADD concatenation",
            Some("hello"),
            Add,
            " world",
            Some("hello world"),
        ),
        ("ADD new state", None, Add, "test", Some("test")),
        (
            "SUBSTRACT not allowed",
            Some("hello"),
            Subtract,
            "test",
            None,
        ),
        (
            "MULTIPLY not allowed",
            Some("hello"),
            Multiply,
            "test",
            None,
        ),
        ("DIVIDE not allowed", Some("hello"), Divide, "test", None),
    ];
    for (name, setup, operator, value, want) in cases {
        let mut a = agent();
        if let Some(v) = setup {
            set_state(&mut a, 1, v);
        }
        let e = EffectString {
            key: 1,
            value: value.into(),
            operator,
        };
        let result = e.apply(&mut a.w);
        match want {
            None => assert!(result.is_err(), "{name}: expected error"),
            Some(want) => {
                assert_eq!(result, Ok(()), "{name}");
                assert_eq!(
                    a.w.get(1).unwrap().value,
                    Value::String(want.into()),
                    "{name}"
                );
            }
        }
    }
}

#[test]
fn effect_bool_get_key() {
    let e = EffectBool {
        key: 42,
        value: true,
        operator: Arithmetic::Set,
    };
    assert_eq!(e.get_key(), 42);
}

#[test]
fn effect_bool_apply_errors() {
    let cases = [
        ("SET allowed", Arithmetic::Set, false),
        ("ADD not allowed", Arithmetic::Add, true),
        ("SUBSTRACT not allowed", Arithmetic::Subtract, true),
    ];
    for (name, operator, want_err) in cases {
        let mut a = agent();
        set_state(&mut a, 1, true);
        let e = EffectBool {
            key: 1,
            value: false,
            operator,
        };
        assert_eq!(e.apply(&mut a.w).is_err(), want_err, "{name}");
    }
}

#[test]
fn effect_apply_all_operators() {
    use Arithmetic::*;
    let cases: [(StateKey, &str, i64, Arithmetic, i64, Option<i64>); 9] = [
        (1, "SET", 100, Set, 50, Some(50)),
        (1, "ADD", 100, Add, 50, Some(150)),
        (1, "SUBSTRACT", 100, Subtract, 30, Some(70)),
        (1, "MULTIPLY", 10, Multiply, 5, Some(50)),
        (1, "DIVIDE", 100, Divide, 4, Some(25)),
        // Upstream's name says "non-existing", but the state is set to 0.
        (1, "ADD on non-existing key", 0, Add, 50, Some(50)),
        (
            1,
            "SUBSTRACT on non-existing key",
            0,
            Subtract,
            50,
            Some(-50),
        ),
        (
            99,
            "MULTIPLY on non-existing key error",
            0,
            Multiply,
            50,
            None,
        ),
        (99, "DIVIDE on non-existing key error", 0, Divide, 50, None),
    ];
    for (key, name, initial, operator, value, want) in cases {
        let mut a = agent();
        set_state(&mut a, 1, initial);
        let e = Effect::new(key, operator, value);
        let result = e.apply(&mut a.w);
        match want {
            None => assert!(result.is_err(), "{name}: expected error"),
            Some(want) => {
                assert_eq!(result, Ok(()), "{name}");
                assert_eq!(a.w.get(1).unwrap().value, Value::Int(want), "{name}");
            }
        }
    }
}

#[test]
fn effect_get_key() {
    assert_eq!(Effect::new(42, Arithmetic::Set, 100_i64).get_key(), 42);
}

// ---- Tests added in the translation (not in upstream) ----

#[test]
fn effects_on_missing_keys() {
    let mut w = World::new();
    Effect::new(1, Arithmetic::Add, 5_i64)
        .apply(&mut w)
        .unwrap();
    Effect::new(2, Arithmetic::Subtract, 5_u8)
        .apply(&mut w)
        .unwrap();
    assert_eq!(w.get(1).unwrap().value, Value::Int(5));
    // Go negates unsigned values with wrap-around: -5 as uint8 is 251.
    assert_eq!(w.get(2).unwrap().value, Value::Uint8(251));
    assert_eq!(
        Effect::new(3, Arithmetic::Multiply, 2_i64).apply(&mut w),
        Err(GoapError::StateDoesNotExist)
    );
}

#[test]
fn integer_arithmetic_wraps_like_go() {
    let mut w = World::new();
    w.store(State::new(1, 120_i8));
    Effect::new(1, Arithmetic::Add, 10_i8)
        .apply(&mut w)
        .unwrap();
    assert_eq!(w.get(1).unwrap().value, Value::Int8(-126));
    w.store(State::new(2, i8::MIN));
    Effect::new(2, Arithmetic::Divide, -1_i8)
        .apply(&mut w)
        .unwrap();
    assert_eq!(w.get(2).unwrap().value, Value::Int8(i8::MIN));
}

#[test]
fn integer_division_by_zero_is_an_error() {
    let mut w = World::new();
    w.store(State::new(1, 10_i64));
    assert_eq!(
        Effect::new(1, Arithmetic::Divide, 0_i64).apply(&mut w),
        Err(GoapError::DivisionByZero)
    );
    // Floats follow IEEE-754, as in Go.
    w.store(State::new(2, 1.0_f64));
    Effect::new(2, Arithmetic::Divide, 0.0_f64)
        .apply(&mut w)
        .unwrap();
    assert_eq!(w.get(2).unwrap().value, Value::Float64(f64::INFINITY));
}

#[test]
fn type_mismatch_is_an_error() {
    let mut w = World::new();
    w.store(State::new(1, 10_i64));
    assert_eq!(
        Effect::new(1, Arithmetic::Add, 1_u8).apply(&mut w),
        Err(GoapError::TypeMismatch)
    );
    assert_eq!(
        EffectBool {
            key: 1,
            value: true,
            operator: Arithmetic::Set
        }
        .apply(&mut w),
        Err(GoapError::TypeMismatch)
    );
    assert_eq!(
        EffectString {
            key: 1,
            value: "x".into(),
            operator: Arithmetic::Set
        }
        .apply(&mut w),
        Err(GoapError::TypeMismatch)
    );
}

#[test]
fn string_add_is_never_a_no_op() {
    // Upstream treated ADD "x" on "x" as already satisfied (DEVIATIONS.md D6).
    let mut w = World::new();
    w.store(State::new(1, "x"));
    let e = EffectString {
        key: 1,
        value: "x".into(),
        operator: Arithmetic::Add,
    };
    assert!(!e.check(&w));
}

#[test]
fn effects_keep_world_hash_consistent() {
    let mut w = World::new();
    let effects: Effects = alloc::vec![
        Effect::new(1, Arithmetic::Add, 3_i64).into(),
        EffectBool {
            key: 2,
            value: true,
            operator: Arithmetic::Set
        }
        .into(),
        EffectString {
            key: 3,
            value: "a".into(),
            operator: Arithmetic::Add
        }
        .into(),
    ];
    apply_effects(&effects, &mut w).unwrap();
    apply_effects(&effects, &mut w).unwrap();
    let xor = w.states().iter().fold(0, |h, s| h ^ s.hash());
    assert_eq!(w.hash(), xor);
}

#[test]
fn error_messages_match_upstream() {
    assert_eq!(GoapError::StateDoesNotExist.to_string(), "w does not exist");
    assert_eq!(GoapError::TypeMismatch.to_string(), "type does not match");
    assert_eq!(
        GoapError::OperationNotAllowedOnBool(Arithmetic::Add).to_string(),
        "operation 1 not allowed on bool type"
    );
    assert_eq!(
        GoapError::OperationNotAllowedOnString(Arithmetic::Subtract).to_string(),
        "arithmetic operation 2 not allowed on string type"
    );
    assert_eq!(GoapError::NoGoalAvailable.to_string(), "no goal available");
}
