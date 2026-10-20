// SPDX-License-Identifier: AGPL-3.0-only
// Translated from goapai `action.go` (https://github.com/akmonengine/goapai,
// commit 2e86c5e, Apache-2.0, Copyright 2024 Agoune Ramine). Modified: see
// docs/DEVIATIONS.md.

//! Actions and effects (`action.go`).

use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::fmt;
use core::ops::{Deref, DerefMut};

use crate::agent::Agent;
use crate::state::{Conditions, Numeric, State, StateKey, Value, World};

/// Arithmetic operator of an effect (Go: `arithmetic`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Arithmetic {
    /// Go: `SET`
    #[default]
    Set,
    /// Go: `ADD`
    Add,
    /// Go: `SUBSTRACT`
    Subtract,
    /// Go: `MULTIPLY`
    Multiply,
    /// Go: `DIVIDE`
    Divide,
}

impl Arithmetic {
    /// The numeric value Go prints for this operator with `%v`.
    pub const fn go_code(self) -> u8 {
        self as u8
    }
}

/// Errors from applying effects or choosing a goal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GoapError {
    /// Go: `"w does not exist"`
    StateDoesNotExist,
    /// Go: `"type does not match"`
    TypeMismatch,
    /// Go: `"operation %v not allowed on bool type"`
    OperationNotAllowedOnBool(Arithmetic),
    /// Go: `"arithmetic operation %v not allowed on string type"`
    OperationNotAllowedOnString(Arithmetic),
    /// Integer division by zero. Go panics here; see DEVIATIONS.md.
    DivisionByZero,
    /// Go: `"no goal available"`
    NoGoalAvailable,
}

impl fmt::Display for GoapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StateDoesNotExist => f.write_str("w does not exist"),
            Self::TypeMismatch => f.write_str("type does not match"),
            Self::OperationNotAllowedOnBool(op) => {
                write!(f, "operation {} not allowed on bool type", op.go_code())
            }
            Self::OperationNotAllowedOnString(op) => {
                write!(
                    f,
                    "arithmetic operation {} not allowed on string type",
                    op.go_code()
                )
            }
            Self::DivisionByZero => f.write_str("integer divide by zero"),
            Self::NoGoalAvailable => f.write_str("no goal available"),
        }
    }
}

/// One action: preconditions, effects and a cost (Go: `Action`).
#[derive(Clone, Debug, Default)]
pub struct Action {
    pub(crate) name: String,
    pub(crate) cost: f32,
    pub(crate) repeatable: bool,
    pub(crate) conditions: Conditions,
    pub(crate) effects: Effects,
}

impl Action {
    pub fn new(
        name: &str,
        cost: f32,
        repeatable: bool,
        conditions: Conditions,
        effects: Effects,
    ) -> Self {
        Self {
            name: String::from(name),
            cost,
            repeatable,
            conditions,
            effects,
        }
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_cost(&self) -> f32 {
        self.cost
    }

    pub fn is_repeatable(&self) -> bool {
        self.repeatable
    }

    pub fn get_conditions(&self) -> &Conditions {
        &self.conditions
    }

    pub fn get_effects(&self) -> &Effects {
        &self.effects
    }
}

/// Available actions (Go: `Actions`, a `[]*Action`).
#[derive(Clone, Debug, Default)]
pub struct Actions(pub Vec<Arc<Action>>);

impl Actions {
    pub fn new() -> Self {
        Self::default()
    }

    /// Go: `Actions.AddAction`.
    pub fn add_action(
        &mut self,
        name: &str,
        cost: f32,
        repeatable: bool,
        conditions: Conditions,
        effects: Effects,
    ) {
        self.0.push(Arc::new(Action::new(
            name, cost, repeatable, conditions, effects,
        )));
    }
}

impl Deref for Actions {
    type Target = Vec<Arc<Action>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Actions {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Numeric effect (Go: `Effect[T]`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Effect<T: Numeric> {
    pub key: StateKey,
    pub operator: Arithmetic,
    pub value: T,
}

impl<T: Numeric> Effect<T> {
    pub fn new(key: StateKey, operator: Arithmetic, value: T) -> Self {
        Self {
            key,
            operator,
            value,
        }
    }

    pub fn get_key(&self) -> StateKey {
        self.key
    }

    /// True if applying this effect would leave `w` unchanged. Only a `Set`
    /// can be known to be a no-op.
    pub(crate) fn check(&self, w: &World) -> bool {
        if self.operator != Arithmetic::Set {
            return false;
        }
        match w.get(self.key).and_then(|s| T::from_value(&s.value)) {
            Some(v) => v == self.value,
            None => false,
        }
    }

    pub(crate) fn apply(&self, w: &mut World) -> Result<(), GoapError> {
        let Some(state) = w.get(self.key) else {
            let v = match self.operator {
                Arithmetic::Set | Arithmetic::Add => self.value,
                Arithmetic::Subtract => self.value.negate(),
                Arithmetic::Multiply | Arithmetic::Divide => {
                    return Err(GoapError::StateDoesNotExist)
                }
            };
            w.store(State::new_value(self.key, v.into_value()));
            return Ok(());
        };
        let current = T::from_value(&state.value).ok_or(GoapError::TypeMismatch)?;
        let next = current.arith(self.operator, self.value)?;
        w.store(State::new_value(self.key, next.into_value()));
        Ok(())
    }
}

/// Boolean effect; only `Set` is allowed (Go: `EffectBool`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EffectBool {
    pub key: StateKey,
    pub value: bool,
    pub operator: Arithmetic,
}

impl EffectBool {
    pub fn get_key(&self) -> StateKey {
        self.key
    }

    pub(crate) fn check(&self, w: &World) -> bool {
        if self.operator != Arithmetic::Set {
            return false;
        }
        matches!(w.get(self.key).map(|s| &s.value), Some(Value::Bool(v)) if *v == self.value)
    }

    pub(crate) fn apply(&self, w: &mut World) -> Result<(), GoapError> {
        if self.operator != Arithmetic::Set {
            return Err(GoapError::OperationNotAllowedOnBool(self.operator));
        }
        match w.get(self.key).map(|s| &s.value) {
            None | Some(Value::Bool(_)) => {
                w.store(State::new_value(self.key, Value::Bool(self.value)));
                Ok(())
            }
            Some(_) => Err(GoapError::TypeMismatch),
        }
    }
}

/// String effect; `Set` replaces, `Add` concatenates (Go: `EffectString`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectString {
    pub key: StateKey,
    pub value: String,
    pub operator: Arithmetic,
}

impl EffectString {
    pub fn get_key(&self) -> StateKey {
        self.key
    }

    pub(crate) fn check(&self, w: &World) -> bool {
        if self.operator != Arithmetic::Set {
            return false;
        }
        matches!(w.get(self.key).map(|s| &s.value), Some(Value::String(v)) if *v == self.value)
    }

    pub(crate) fn apply(&self, w: &mut World) -> Result<(), GoapError> {
        if !matches!(self.operator, Arithmetic::Set | Arithmetic::Add) {
            return Err(GoapError::OperationNotAllowedOnString(self.operator));
        }
        let next = match w.get(self.key).map(|s| &s.value) {
            None => self.value.clone(),
            Some(Value::String(v)) => match self.operator {
                Arithmetic::Add => {
                    let mut s = v.clone();
                    s.push_str(&self.value);
                    s
                }
                _ => self.value.clone(),
            },
            Some(_) => return Err(GoapError::TypeMismatch),
        };
        w.store(State::new_value(self.key, Value::String(next)));
        Ok(())
    }
}

/// Procedural effect on the agent (Go: `EffectFn`). Declared but unused
/// upstream; kept for API traceability.
pub type EffectFn = fn(&mut Agent);

/// Any effect (Go: `EffectInterface`).
#[derive(Clone, Debug, PartialEq)]
pub enum EffectInterface {
    Int8(Effect<i8>),
    Int(Effect<i64>),
    Uint8(Effect<u8>),
    Uint64(Effect<u64>),
    Float64(Effect<f64>),
    Bool(EffectBool),
    String(EffectString),
}

impl EffectInterface {
    pub fn get_key(&self) -> StateKey {
        match self {
            Self::Int8(e) => e.key,
            Self::Int(e) => e.key,
            Self::Uint8(e) => e.key,
            Self::Uint64(e) => e.key,
            Self::Float64(e) => e.key,
            Self::Bool(e) => e.key,
            Self::String(e) => e.key,
        }
    }

    pub(crate) fn check(&self, w: &World) -> bool {
        match self {
            Self::Int8(e) => e.check(w),
            Self::Int(e) => e.check(w),
            Self::Uint8(e) => e.check(w),
            Self::Uint64(e) => e.check(w),
            Self::Float64(e) => e.check(w),
            Self::Bool(e) => e.check(w),
            Self::String(e) => e.check(w),
        }
    }

    pub(crate) fn apply(&self, w: &mut World) -> Result<(), GoapError> {
        match self {
            Self::Int8(e) => e.apply(w),
            Self::Int(e) => e.apply(w),
            Self::Uint8(e) => e.apply(w),
            Self::Uint64(e) => e.apply(w),
            Self::Float64(e) => e.apply(w),
            Self::Bool(e) => e.apply(w),
            Self::String(e) => e.apply(w),
        }
    }
}

macro_rules! effect_from {
    ($t:ty, $variant:ident) => {
        impl From<Effect<$t>> for EffectInterface {
            fn from(e: Effect<$t>) -> Self {
                Self::$variant(e)
            }
        }
    };
}
effect_from!(i8, Int8);
effect_from!(i64, Int);
effect_from!(u8, Uint8);
effect_from!(u64, Uint64);
effect_from!(f64, Float64);

impl From<EffectBool> for EffectInterface {
    fn from(e: EffectBool) -> Self {
        Self::Bool(e)
    }
}
impl From<EffectString> for EffectInterface {
    fn from(e: EffectString) -> Self {
        Self::String(e)
    }
}

/// An action's effects, applied in order (Go: `Effects`).
pub type Effects = Vec<EffectInterface>;

/// True if every effect is already satisfied in `w`, i.e. the action would
/// change nothing (Go: `Effects.satisfyStates`). True for no effects.
pub(crate) fn satisfy_states(effects: &[EffectInterface], w: &World) -> bool {
    effects.iter().all(|e| e.check(w))
}

/// Apply effects in order, stopping at the first error (Go: `Effects.apply`).
pub(crate) fn apply_effects(effects: &[EffectInterface], w: &mut World) -> Result<(), GoapError> {
    effects.iter().try_for_each(|e| e.apply(w))
}

#[cfg(test)]
#[path = "action_test.rs"]
mod action_test;

#[cfg(test)]
#[path = "effect_test.rs"]
mod effect_test;
