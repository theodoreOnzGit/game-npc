//! # game-npc — NPC intelligence engine core
//!
//! A deterministic decision core for NPC agency, built around a finite,
//! hierarchical motivation taxonomy (GitHub issue #1).
//!
//! The pipeline follows the architecture in the specification:
//!
//! ```text
//! World event + personality
//!   -> deterministic appraisal          (appraisal)
//!   -> character state                  (state)
//!   -> motivation model                 (model: deterministic baseline today,
//!                                        Burn FP32 network later)
//!   -> motivation priority vector       (vector)
//!   -> symbolic goal generator          (goal)
//!   -> deterministic planner + validator(action)
//!   -> world state transition           (world)
//!   -> decision trace, replayable       (trace)
//! ```
//!
//! Motivation, goal, action, emotion, personality and resource are distinct
//! types. A motivation model only ever produces a [`MotivationVector`]; it has
//! no access to the world. Only an [`action::ValidatedAction`], which can be
//! constructed solely by [`action::validate`], may change [`world::WorldState`].
//!
//! The crate is `no_std` and allocation-free, so the same data model compiles
//! for native targets and `wasm32-unknown-unknown`.
#![no_std]
#![forbid(unsafe_code)]

#[cfg(feature = "std")]
extern crate std;

pub mod action;
pub mod appraisal;
pub mod goal;
pub mod model;
pub mod motivation;
pub mod personality;
pub mod state;
pub mod trace;
pub mod vector;
pub mod world;

pub use action::{validate, Action, ActionRejection, ValidatedAction};
pub use appraisal::{appraise, WorldEvent};
pub use goal::{generate_goal, Goal};
pub use model::{DeterministicModel, MotivationModel, ScoringParams};
pub use motivation::{Domain, Motivation, MOTIVATION_COUNT};
pub use personality::Personality;
pub use state::{CharacterState, EmotionState, Perception};
pub use trace::{decide, replay, DecisionTrace, ReplayError};
pub use vector::MotivationVector;
pub use world::WorldState;

/// Clamp a value to the closed unit interval. NaN maps to 0 so a corrupt
/// input can never propagate through scoring.
#[inline]
pub(crate) fn unit(x: f32) -> f32 {
    if x.is_nan() {
        0.0
    } else {
        x.clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::unit;

    #[test]
    fn unit_clamps_and_rejects_nan() {
        assert_eq!(unit(-1.0), 0.0);
        assert_eq!(unit(2.0), 1.0);
        assert_eq!(unit(0.25), 0.25);
        assert_eq!(unit(f32::NAN), 0.0);
    }
}
