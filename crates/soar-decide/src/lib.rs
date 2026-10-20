// SPDX-License-Identifier: AGPL-3.0-only
// Translated from parts of the Soar kernel (https://github.com/SoarGroup/Soar,
// commit 3fc337a, BSD, Copyright 1995-2012 Carnegie Mellon University, The
// Regents of the University of Michigan, USC/ISI). Modified: see
// docs/DEVIATIONS.md.

//! # soar-decide: Soar's decision procedure in Rust
//!
//! A translation of the part of the [Soar](https://github.com/SoarGroup/Soar)
//! kernel that turns operator proposals and preferences into a single
//! choice, or into a typed impasse:
//!
//! - [`semantics`]: preference semantics: require, reject/prohibit,
//!   better/worse, best/worst, unary, binary and numeric indifference.
//! - [`decide`]: deciding a context slot (selection or tie / conflict /
//!   constraint-failure / no-change impasse).
//! - [`exploration`]: the six exploration policies (first, last,
//!   random-uniform, softmax, ε-greedy, Boltzmann) with parameter decay.
//! - [`rng`]: Soar's Mersenne Twister, so random choices match Soar's for
//!   the same seed.
//!
//! Values (operators) are any `Copy + PartialEq` type, so the caller keeps
//! its own identifiers. No production matcher is needed: preferences go in
//! directly.
//!
//! `tests/differential.rs` checks this crate against the real `soar`
//! binary. The intentional differences are in `docs/DEVIATIONS.md`.
//!
//! ```
//! use soar_decide::*;
//!
//! // Three subroutines; the planner must not run while `Pathfind` is
//! // prohibited, and the rest are scored numerically.
//! #[derive(Clone, Copy, PartialEq, Debug)]
//! enum Op { Plan, Pathfind, Compute }
//!
//! let mut slot = Slot::new();
//! for op in [Op::Plan, Op::Pathfind, Op::Compute] {
//!     slot.push(Preference::unary(PreferenceType::Acceptable, op));
//! }
//! slot.push(Preference::unary(PreferenceType::Prohibit, Op::Pathfind));
//! slot.push(Preference::numeric(Op::Plan, 0.9));
//! slot.push(Preference::numeric(Op::Compute, 0.1));
//!
//! let mut explore = Exploration::new(Policy::EpsilonGreedy);
//! explore.set_parameter(ParameterName::Epsilon, 0.0).unwrap();
//! let mut rng = SoarRng::new(42);
//!
//! let d = decide_context_slot(&slot, false, &explore, &mut rng, &mut None);
//! assert_eq!(d, Decision::Selected(Op::Plan));
//! ```
#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

pub mod decide;
pub mod exploration;
pub mod preference;
pub mod rng;
pub mod semantics;

pub use decide::{decide_context_slot, Decision, Impasse, ImpasseAttribute};
pub use exploration::{
    Exploration, ExplorationError, NumericIndifferentMode, Parameter, ParameterName, Policy,
    ReductionPolicy,
};
pub use preference::{Preference, PreferenceType, Referent, Slot, NUM_PREFERENCE_TYPES};
pub use rng::SoarRng;
pub use semantics::{
    run_non_context_preference_semantics, run_preference_semantics,
    run_preference_semantics_read_only, ImpasseType, Semantics,
};
