// SPDX-License-Identifier: AGPL-3.0-only
// Translated from goapai `distance_test.go` (commit 2e86c5e, Apache-2.0,
// Copyright 2024 Agoune Ramine).

use crate::state::{
    Condition, ConditionBool, ConditionFn, ConditionInterface, ConditionString, Operator::*, State,
    Value,
};

fn s(v: Value) -> State {
    State::new_value(1, v)
}

#[test]
fn state_distance() {
    use Value::*;
    let cases: alloc::vec::Vec<(&str, Value, ConditionInterface, f32)> = alloc::vec![
        // Numeric types - EQUAL operator
        (
            "int EQUAL satisfied",
            Int(100),
            Condition::new(1, Equal, 100_i64).into(),
            0.0
        ),
        (
            "int EQUAL below target",
            Int(50),
            Condition::new(1, Equal, 100_i64).into(),
            50.0
        ),
        (
            "int EQUAL above target",
            Int(150),
            Condition::new(1, Equal, 100_i64).into(),
            50.0
        ),
        // UPPER_OR_EQUAL
        (
            "int UPPER_OR_EQUAL satisfied",
            Int(100),
            Condition::new(1, UpperOrEqual, 80_i64).into(),
            0.0
        ),
        (
            "int UPPER_OR_EQUAL below target",
            Int(50),
            Condition::new(1, UpperOrEqual, 80_i64).into(),
            30.0
        ),
        // UPPER
        (
            "int UPPER satisfied",
            Int(100),
            Condition::new(1, Upper, 80_i64).into(),
            0.0
        ),
        (
            "int UPPER at target",
            Int(80),
            Condition::new(1, Upper, 80_i64).into(),
            1.0
        ),
        // LOWER_OR_EQUAL
        (
            "int LOWER_OR_EQUAL satisfied",
            Int(50),
            Condition::new(1, LowerOrEqual, 80_i64).into(),
            0.0
        ),
        (
            "int LOWER_OR_EQUAL above target",
            Int(100),
            Condition::new(1, LowerOrEqual, 80_i64).into(),
            20.0
        ),
        // LOWER
        (
            "int LOWER satisfied",
            Int(50),
            Condition::new(1, Lower, 80_i64).into(),
            0.0
        ),
        (
            "int LOWER at target",
            Int(80),
            Condition::new(1, Lower, 80_i64).into(),
            1.0
        ),
        // NOT_EQUAL
        (
            "int NOT_EQUAL satisfied",
            Int(50),
            Condition::new(1, NotEqual, 100_i64).into(),
            0.0
        ),
        (
            "int NOT_EQUAL not satisfied",
            Int(100),
            Condition::new(1, NotEqual, 100_i64).into(),
            1.0
        ),
        // Float64, uint64, int8, uint8
        (
            "float64 EQUAL",
            Float64(75.5),
            Condition::new(1, Equal, 100.0_f64).into(),
            24.5
        ),
        (
            "uint64 UPPER_OR_EQUAL",
            Uint64(50),
            Condition::new(1, UpperOrEqual, 100_u64).into(),
            50.0
        ),
        (
            "int8 EQUAL",
            Int8(50),
            Condition::new(1, Equal, 100_i8).into(),
            50.0
        ),
        (
            "uint8 LOWER_OR_EQUAL",
            Uint8(100),
            Condition::new(1, LowerOrEqual, 80_u8).into(),
            20.0
        ),
        // Bool
        (
            "bool EQUAL satisfied",
            Bool(true),
            ConditionBool {
                key: 1,
                value: true,
                operator: Equal
            }
            .into(),
            0.0
        ),
        (
            "bool EQUAL not satisfied",
            Bool(true),
            ConditionBool {
                key: 1,
                value: false,
                operator: Equal
            }
            .into(),
            1.0
        ),
        (
            "bool NOT_EQUAL satisfied",
            Bool(true),
            ConditionBool {
                key: 1,
                value: false,
                operator: NotEqual
            }
            .into(),
            0.0
        ),
        (
            "bool NOT_EQUAL not satisfied",
            Bool(true),
            ConditionBool {
                key: 1,
                value: true,
                operator: NotEqual
            }
            .into(),
            1.0
        ),
        // String
        (
            "string EQUAL satisfied",
            String("test".into()),
            ConditionString {
                key: 1,
                value: "test".into(),
                operator: Equal
            }
            .into(),
            0.0
        ),
        (
            "string EQUAL not satisfied",
            String("test".into()),
            ConditionString {
                key: 1,
                value: "other".into(),
                operator: Equal
            }
            .into(),
            1.0
        ),
        (
            "string NOT_EQUAL satisfied",
            String("test".into()),
            ConditionString {
                key: 1,
                value: "other".into(),
                operator: NotEqual
            }
            .into(),
            0.0
        ),
        (
            "string NOT_EQUAL not satisfied",
            String("test".into()),
            ConditionString {
                key: 1,
                value: "test".into(),
                operator: NotEqual
            }
            .into(),
            1.0
        ),
        // Key mismatch
        (
            "key mismatch",
            Int(100),
            Condition::new(99, Equal, 100_i64).into(),
            0.0
        ),
    ];
    for (name, state_val, condition, want) in cases {
        assert_eq!(s(state_val).distance(&condition), want, "{name}");
    }
}

// ---- Tests added in the translation (not in upstream) ----

#[test]
fn type_mismatch_and_procedural_conditions_have_zero_distance() {
    assert_eq!(
        s(Value::Int8(5)).distance(&Condition::new(1, Equal, 100_i64).into()),
        0.0
    );
    let f: ConditionInterface = ConditionFn::new(1, |_| false).into();
    assert_eq!(s(Value::Int(5)).distance(&f), 0.0);
}

#[test]
fn large_uint64_distance_rounds_like_go() {
    // Both values convert to f64 with rounding; the f64 gap is then rounded
    // to f32. Reference value printed by Go for the same inputs.
    let d = s(Value::Uint64(u64::MAX)).distance(&Condition::new(1, Equal, 1_u64).into());
    assert_eq!(d, 1.8446744e19);
}
