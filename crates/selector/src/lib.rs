// SPDX-License-Identifier: AGPL-3.0-only

//! # selector: one reasoning subroutine per request
//!
//! The issue #4 subroutine selector, built on Soar's decision procedure
//! (`soar-decide`). A higher-level reasoner sends a structured
//! [`ReasoningRequest`]. The selector checks which registered subroutines
//! are applicable, ranks them through Soar preferences, and returns exactly
//! one [`ReasoningTask`] or a typed [`SelectionError`]. It never runs the
//! subroutine: execution goes through the typed [`Subroutine`] interface,
//! implemented outside this crate.
//!
//! - Deterministic by default ([`DeterministicSelector`]).
//! - Learned preferences plug in through [`UtilityModel`]
//!   ([`UtilitySelector`]). They can only rank routines that symbolic
//!   applicability already admitted.
//! - Every selection can be traced and replayed ([`SelectionTrace`],
//!   [`replay`]).
//!
//! ```
//! use selector::*;
//!
//! let mut registry = Registry::new();
//! registry.register(SubroutineSpec {
//!     task: ReasoningTask::GraphAnalysis,
//!     accepts: InputKind::Graph,
//!     returns: OutputKind::Path,
//!     cost: Cost::Linear { base: 5, per_item: 3 },
//!     precondition: None,
//! }).unwrap();
//! registry.register(SubroutineSpec {
//!     task: ReasoningTask::Pathfinding,
//!     accepts: InputKind::Graph,
//!     returns: OutputKind::Path,
//!     cost: Cost::Linear { base: 20, per_item: 1 },
//!     precondition: None,
//! }).unwrap();
//!
//! let selector = DeterministicSelector { registry };
//! // Small graph: graph analysis is cheaper.
//! let small = ReasoningRequest::new(InputKind::Graph, 4);
//! assert_eq!(selector.select(&small), Ok(ReasoningTask::GraphAnalysis));
//! // Large graph: pathfinding is cheaper.
//! let large = ReasoningRequest::new(InputKind::Graph, 100);
//! assert_eq!(selector.select(&large), Ok(ReasoningTask::Pathfinding));
//! // Nothing accepts a proposition.
//! let prop = ReasoningRequest::new(InputKind::Proposition, 1);
//! assert_eq!(selector.select(&prop), Err(SelectionError::NoApplicableRoutine));
//! ```
#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;

pub mod actr;
pub mod execute;
pub mod registry;
pub mod select;
pub mod task;

pub use actr::{ActrError, ActrUtility};
pub use execute::{execute, ExecutionError, Limits, Subroutine};
pub use registry::{
    Applicability, Cost, Precondition, ReasoningRequest, Registry, RegistryError, SubroutineSpec,
};
pub use select::{
    replay, DeterministicSelector, SelectionError, SelectionTrace, SubroutineSelector,
    UtilityModel, UtilitySelector,
};
pub use task::{InputKind, OutputKind, ReasoningTask};
