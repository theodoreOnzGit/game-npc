// SPDX-License-Identifier: AGPL-3.0-only
// Translated from goapai `state.go` (https://github.com/akmonengine/goapai,
// commit 2e86c5e, Apache-2.0, Copyright 2024 Agoune Ramine). Modified: see
// docs/DEVIATIONS.md.

//! World state, states, sensors and conditions (`state.go`).

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::any::Any;
use core::sync::atomic::{AtomicU8, Ordering};

use crate::action::{Arithmetic, GoapError};

/// Comparison operator of a condition (Go: `operator`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Operator {
    /// Go: `EQUAL`
    #[default]
    Equal,
    /// Go: `NOT_EQUAL`
    NotEqual,
    /// Go: `LOWER_OR_EQUAL`
    LowerOrEqual,
    /// Go: `LOWER`
    Lower,
    /// Go: `UPPER_OR_EQUAL`
    UpperOrEqual,
    /// Go: `UPPER`
    Upper,
}

/// Compact 16-bit state identifier (Go: `StateKey`).
pub type StateKey = u16;

/// A state value. Each variant is one Go type parameter of `State[T]`.
///
/// Go `int` maps to `i64` (Go's `int` is 64-bit on amd64, arm64 and wasm).
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Int8(i8),
    Int(i64),
    Uint8(u8),
    Uint64(u64),
    Float64(f64),
    Bool(bool),
    String(String),
}

mod sealed {
    pub trait Sealed {}
}

/// Numeric state types (Go: `Numeric` = `int8 | int | uint8 | uint64 | float64`).
pub trait Numeric: Copy + PartialOrd + core::fmt::Debug + sealed::Sealed {
    #[doc(hidden)]
    fn from_value(v: &Value) -> Option<Self>;
    #[doc(hidden)]
    fn into_value(self) -> Value;
    #[doc(hidden)]
    fn to_f64(self) -> f64;
    /// The `uint64(v)` conversion Go's `State.Hash` mixes in.
    #[doc(hidden)]
    fn hash_bits(self) -> u64;
    /// Go unary minus (wrapping for integers).
    #[doc(hidden)]
    fn negate(self) -> Self;
    /// Go arithmetic: integers wrap; integer division by zero is an error
    /// (Go panics; see DEVIATIONS.md).
    #[doc(hidden)]
    fn arith(self, op: Arithmetic, rhs: Self) -> Result<Self, GoapError>;
}

macro_rules! impl_int {
    ($t:ty, $variant:ident) => {
        impl sealed::Sealed for $t {}
        impl Numeric for $t {
            fn from_value(v: &Value) -> Option<Self> {
                match v {
                    Value::$variant(x) => Some(*x),
                    _ => None,
                }
            }
            fn into_value(self) -> Value {
                Value::$variant(self)
            }
            fn to_f64(self) -> f64 {
                self as f64
            }
            fn hash_bits(self) -> u64 {
                // Sign-extends for signed types, as Go's uint64(v) does.
                self as u64
            }
            fn negate(self) -> Self {
                self.wrapping_neg()
            }
            fn arith(self, op: Arithmetic, rhs: Self) -> Result<Self, GoapError> {
                Ok(match op {
                    Arithmetic::Set => rhs,
                    Arithmetic::Add => self.wrapping_add(rhs),
                    Arithmetic::Subtract => self.wrapping_sub(rhs),
                    Arithmetic::Multiply => self.wrapping_mul(rhs),
                    Arithmetic::Divide => {
                        if rhs == 0 {
                            return Err(GoapError::DivisionByZero);
                        }
                        self.wrapping_div(rhs)
                    }
                })
            }
        }
        impl StateValue for $t {
            fn into_value(self) -> Value {
                Value::$variant(self)
            }
        }
    };
}

impl_int!(i8, Int8);
impl_int!(i64, Int);
impl_int!(u8, Uint8);
impl_int!(u64, Uint64);

impl sealed::Sealed for f64 {}
impl Numeric for f64 {
    fn from_value(v: &Value) -> Option<Self> {
        match v {
            Value::Float64(x) => Some(*x),
            _ => None,
        }
    }
    fn into_value(self) -> Value {
        Value::Float64(self)
    }
    fn to_f64(self) -> f64 {
        self
    }
    fn hash_bits(self) -> u64 {
        self.to_bits()
    }
    fn negate(self) -> Self {
        -self
    }
    fn arith(self, op: Arithmetic, rhs: Self) -> Result<Self, GoapError> {
        Ok(match op {
            Arithmetic::Set => rhs,
            Arithmetic::Add => self + rhs,
            Arithmetic::Subtract => self - rhs,
            Arithmetic::Multiply => self * rhs,
            Arithmetic::Divide => self / rhs,
        })
    }
}
impl StateValue for f64 {
    fn into_value(self) -> Value {
        Value::Float64(self)
    }
}

/// Anything storable as a state (Go: `Numeric | bool | string`).
pub trait StateValue {
    fn into_value(self) -> Value;
}
impl StateValue for bool {
    fn into_value(self) -> Value {
        Value::Bool(self)
    }
}
impl StateValue for String {
    fn into_value(self) -> Value {
        Value::String(self)
    }
}
impl StateValue for &str {
    fn into_value(self) -> Value {
        Value::String(String::from(self))
    }
}

/// One key-value pair of the world state (Go: `State[T]`).
#[derive(Clone, Debug, PartialEq)]
pub struct State {
    pub key: StateKey,
    pub value: Value,
    /// Cached hash, maintained by [`World::store`].
    hash: u64,
}

impl State {
    pub fn new(key: StateKey, value: impl StateValue) -> Self {
        Self {
            key,
            value: value.into_value(),
            hash: 0,
        }
    }

    pub fn new_value(key: StateKey, value: Value) -> Self {
        Self {
            key,
            value,
            hash: 0,
        }
    }

    pub fn get_key(&self) -> StateKey {
        self.key
    }

    pub fn get_value(&self) -> &Value {
        &self.value
    }

    pub fn get_hash(&self) -> u64 {
        self.hash
    }

    /// True if `w` holds a state at `key` with the same type and value.
    pub fn check(&self, w: &World, key: StateKey) -> bool {
        match w.get(key) {
            Some(s) => s.value == self.value,
            None => false,
        }
    }

    /// Fast multiplicative hash of key and value.
    pub fn hash(&self) -> u64 {
        const PRIME1: u64 = 11400714819323198485;
        const PRIME2: u64 = 14029467366897019727;
        let mut hash = u64::from(self.key).wrapping_mul(PRIME1);
        match &self.value {
            Value::Int8(v) => hash ^= v.hash_bits().wrapping_mul(PRIME2),
            Value::Int(v) => hash ^= v.hash_bits().wrapping_mul(PRIME2),
            Value::Uint8(v) => hash ^= v.hash_bits().wrapping_mul(PRIME2),
            Value::Uint64(v) => hash ^= v.hash_bits().wrapping_mul(PRIME2),
            Value::Float64(v) => hash ^= v.hash_bits().wrapping_mul(PRIME2),
            Value::Bool(v) => {
                if *v {
                    hash ^= PRIME2
                }
            }
            Value::String(v) => {
                for b in v.bytes() {
                    hash = hash.wrapping_mul(PRIME2) ^ u64::from(b);
                }
            }
        }
        hash
    }

    /// Store this state into `w`, replacing any state with the same key.
    pub fn store(self, w: &mut World) {
        w.store(self);
    }
}

/// The world state: an ordered list of states plus an incremental hash
/// (Go: `world`).
///
/// Unlike Go, the world does not hold a pointer to its agent; sensors are
/// passed to condition checks explicitly.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct World {
    pub(crate) states: Vec<State>,
    pub(crate) hash: u64,
}

impl World {
    pub fn new() -> Self {
        Self::default()
    }

    /// Go: `world.Check` (compares by hash).
    pub fn check(&self, other: &World) -> bool {
        self.hash == other.hash
    }

    pub fn states(&self) -> &[State] {
        &self.states
    }

    pub fn hash(&self) -> u64 {
        self.hash
    }

    /// Go: `states.GetIndex`.
    pub fn get_index(&self, key: StateKey) -> Option<usize> {
        self.states.iter().position(|s| s.key == key)
    }

    pub fn get(&self, key: StateKey) -> Option<&State> {
        self.get_index(key).map(|k| &self.states[k])
    }

    /// Insert or replace the state at `state.key`, keeping the world hash
    /// consistent (Go: `State.Store`).
    pub fn store(&mut self, mut state: State) {
        state.hash = state.hash();
        match self.get_index(state.key) {
            Some(k) => {
                self.hash ^= self.states[k].hash ^ state.hash;
                self.states[k] = state;
            }
            None => {
                self.hash ^= state.hash;
                self.states.push(state);
            }
        }
    }
}

/// External data for priority functions and procedural conditions
/// (Go: `Sensor`).
pub type Sensor = Arc<dyn Any + Send + Sync>;

/// Named sensors (Go: `Sensors`, a `map[string]Sensor`).
#[derive(Clone, Default)]
pub struct Sensors {
    map: BTreeMap<String, Sensor>,
}

impl core::fmt::Debug for Sensors {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_set().entries(self.map.keys()).finish()
    }
}

impl Sensors {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set<T: Any + Send + Sync>(&mut self, name: &str, value: T) {
        self.map.insert(String::from(name), Arc::new(value));
    }

    /// Go: `Sensors.GetSensor` (Go returns `nil` when missing).
    pub fn get_sensor(&self, name: &str) -> Option<&Sensor> {
        self.map.get(name)
    }

    /// Typed access: the sensor downcast to `T`.
    pub fn get<T: Any>(&self, name: &str) -> Option<&T> {
        self.map.get(name).and_then(|s| s.downcast_ref::<T>())
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

/// Numeric condition (Go: `Condition[T]`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Condition<T: Numeric> {
    pub key: StateKey,
    pub value: T,
    pub operator: Operator,
}

impl<T: Numeric> Condition<T> {
    pub fn new(key: StateKey, operator: Operator, value: T) -> Self {
        Self {
            key,
            value,
            operator,
        }
    }

    pub fn get_key(&self) -> StateKey {
        self.key
    }

    pub fn check(&self, w: &World) -> bool {
        let Some(v) = w.get(self.key).and_then(|s| T::from_value(&s.value)) else {
            return false;
        };
        match self.operator {
            Operator::Equal => v == self.value,
            Operator::NotEqual => v != self.value,
            Operator::LowerOrEqual => v <= self.value,
            Operator::Lower => v < self.value,
            Operator::UpperOrEqual => v >= self.value,
            Operator::Upper => v > self.value,
        }
    }
}

/// Boolean condition; only `Equal` and `NotEqual` can succeed
/// (Go: `ConditionBool`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConditionBool {
    pub key: StateKey,
    pub value: bool,
    pub operator: Operator,
}

impl ConditionBool {
    pub fn get_key(&self) -> StateKey {
        self.key
    }

    pub fn check(&self, w: &World) -> bool {
        match w.get(self.key).map(|s| &s.value) {
            Some(Value::Bool(v)) => match self.operator {
                Operator::Equal => *v == self.value,
                Operator::NotEqual => *v != self.value,
                _ => false,
            },
            _ => false,
        }
    }
}

/// String condition; only `Equal` and `NotEqual` can succeed
/// (Go: `ConditionString`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConditionString {
    pub key: StateKey,
    pub value: String,
    pub operator: Operator,
}

impl ConditionString {
    pub fn get_key(&self) -> StateKey {
        self.key
    }

    pub fn check(&self, w: &World) -> bool {
        match w.get(self.key).map(|s| &s.value) {
            Some(Value::String(v)) => match self.operator {
                Operator::Equal => *v == self.value,
                Operator::NotEqual => *v != self.value,
                _ => false,
            },
            _ => false,
        }
    }
}

/// Function type of a procedural condition.
pub type CheckFn = Arc<dyn Fn(&Sensors) -> bool + Send + Sync>;

const UNRESOLVED: u8 = 0;
const RESOLVED_FALSE: u8 = 1;
const RESOLVED_TRUE: u8 = 2;

/// Procedural condition evaluated against sensors (Go: `ConditionFn`).
///
/// The result is cached on first evaluation and the cache is reset at the
/// start of every planning request ([`crate::get_plan`]), so it is resolved
/// once per request. Share one condition between several actions or goals
/// by cloning the [`ConditionInterface`]; clones share the cache.
pub struct ConditionFn {
    pub key: StateKey,
    check_fn: CheckFn,
    cache: AtomicU8,
}

impl core::fmt::Debug for ConditionFn {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ConditionFn")
            .field("key", &self.key)
            .field("cache", &self.cache.load(Ordering::Relaxed))
            .finish()
    }
}

impl ConditionFn {
    pub fn new(key: StateKey, check_fn: impl Fn(&Sensors) -> bool + Send + Sync + 'static) -> Self {
        Self {
            key,
            check_fn: Arc::new(check_fn),
            cache: AtomicU8::new(UNRESOLVED),
        }
    }

    pub fn get_key(&self) -> StateKey {
        self.key
    }

    pub fn check(&self, sensors: &Sensors) -> bool {
        match self.cache.load(Ordering::Relaxed) {
            RESOLVED_TRUE => true,
            RESOLVED_FALSE => false,
            _ => {
                let valid = (self.check_fn)(sensors);
                let v = if valid { RESOLVED_TRUE } else { RESOLVED_FALSE };
                self.cache.store(v, Ordering::Relaxed);
                valid
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn resolved(&self) -> bool {
        self.cache.load(Ordering::Relaxed) != UNRESOLVED
    }

    pub(crate) fn reset(&self) {
        self.cache.store(UNRESOLVED, Ordering::Relaxed);
    }
}

/// Any condition (Go: `ConditionInterface`).
#[derive(Clone, Debug)]
pub enum ConditionInterface {
    Int8(Condition<i8>),
    Int(Condition<i64>),
    Uint8(Condition<u8>),
    Uint64(Condition<u64>),
    Float64(Condition<f64>),
    Bool(ConditionBool),
    String(ConditionString),
    Fn(Arc<ConditionFn>),
}

impl ConditionInterface {
    pub fn get_key(&self) -> StateKey {
        match self {
            Self::Int8(c) => c.key,
            Self::Int(c) => c.key,
            Self::Uint8(c) => c.key,
            Self::Uint64(c) => c.key,
            Self::Float64(c) => c.key,
            Self::Bool(c) => c.key,
            Self::String(c) => c.key,
            Self::Fn(c) => c.key,
        }
    }

    pub fn check(&self, w: &World, sensors: &Sensors) -> bool {
        match self {
            Self::Int8(c) => c.check(w),
            Self::Int(c) => c.check(w),
            Self::Uint8(c) => c.check(w),
            Self::Uint64(c) => c.check(w),
            Self::Float64(c) => c.check(w),
            Self::Bool(c) => c.check(w),
            Self::String(c) => c.check(w),
            Self::Fn(c) => c.check(sensors),
        }
    }

    pub(crate) fn reset_cache(&self) {
        if let Self::Fn(c) = self {
            c.reset();
        }
    }
}

macro_rules! condition_from {
    ($t:ty, $variant:ident) => {
        impl From<Condition<$t>> for ConditionInterface {
            fn from(c: Condition<$t>) -> Self {
                Self::$variant(c)
            }
        }
    };
}
condition_from!(i8, Int8);
condition_from!(i64, Int);
condition_from!(u8, Uint8);
condition_from!(u64, Uint64);
condition_from!(f64, Float64);

impl From<ConditionBool> for ConditionInterface {
    fn from(c: ConditionBool) -> Self {
        Self::Bool(c)
    }
}
impl From<ConditionString> for ConditionInterface {
    fn from(c: ConditionString) -> Self {
        Self::String(c)
    }
}
impl From<ConditionFn> for ConditionInterface {
    fn from(c: ConditionFn) -> Self {
        Self::Fn(Arc::new(c))
    }
}
impl From<Arc<ConditionFn>> for ConditionInterface {
    fn from(c: Arc<ConditionFn>) -> Self {
        Self::Fn(c)
    }
}

/// All of these must hold (Go: `Conditions`).
pub type Conditions = Vec<ConditionInterface>;

/// Go: `Conditions.Check`. True for an empty list.
pub fn check_conditions(conditions: &[ConditionInterface], w: &World, sensors: &Sensors) -> bool {
    conditions.iter().all(|c| c.check(w, sensors))
}

#[cfg(test)]
#[path = "state_test.rs"]
mod state_test;
