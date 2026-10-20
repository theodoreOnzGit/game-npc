// SPDX-License-Identifier: AGPL-3.0-only

//! ACT-R-style utility learning: a tiny, deterministic learned preference.
//!
//! ACT-R learns a utility per production from the rewards that follow it,
//! with the difference-learning rule (Anderson et al. 2004, *An integrated
//! theory of the mind*, Psychol. Rev. 111(4); reference [4] of
//! `docs/literature-review.md`):
//!
//! `U(n) = U(n−1) + α · (R(n) − U(n−1))`
//!
//! Here the "production" is a routine chosen for a kind of input, so
//! utilities are kept per `(InputKind, ReasoningTask)`. Plugged into
//! [`crate::UtilitySelector`], this gives a learned selector that:
//! - can only rank routines that are already applicable;
//! - is bit-reproducible (`+ − ×` only);
//! - needs no neural network.
//!
//! It is the cheap learned baseline that a Burn model must beat (literature
//! review §6.2; issue #4 phase 3).
//!
//! **Not modelled:** ACT-R adds logistic noise to utilities before
//! choosing. Here selection is greedy, and exact ties are broken by the
//! selector's seeded RNG, which keeps runs replayable for lockstep play.
//! Exploration can be added with the soar-decide policies if wanted.

use alloc::vec::Vec;

use crate::registry::ReasoningRequest;
use crate::select::UtilityModel;
use crate::task::{InputKind, ReasoningTask};

/// Why a learning setting was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActrError {
    /// α must be in (0, 1].
    InvalidLearningRate,
    /// Initial utilities and rewards must be finite.
    NonFinite,
}

/// Per-(input kind, routine) utilities learned from rewards.
#[derive(Clone, Debug, PartialEq)]
pub struct ActrUtility {
    alpha: f64,
    initial: f64,
    table: Vec<((InputKind, ReasoningTask), f64)>,
}

impl ActrUtility {
    /// `alpha` is the learning rate (ACT-R's default is 0.2). `initial` is
    /// the utility before any reward (ACT-R's default is 0).
    pub fn new(alpha: f64, initial: f64) -> Result<Self, ActrError> {
        if !(alpha > 0.0 && alpha <= 1.0) {
            return Err(ActrError::InvalidLearningRate);
        }
        if !initial.is_finite() {
            return Err(ActrError::NonFinite);
        }
        Ok(Self {
            alpha,
            initial,
            table: Vec::new(),
        })
    }

    /// Current utility of `task` for inputs of `kind`.
    pub fn value(&self, kind: InputKind, task: ReasoningTask) -> f64 {
        self.table
            .iter()
            .find(|(k, _)| *k == (kind, task))
            .map_or(self.initial, |(_, u)| *u)
    }

    /// Apply one reward after `task` ran on an input of `kind`.
    pub fn reward(
        &mut self,
        kind: InputKind,
        task: ReasoningTask,
        r: f64,
    ) -> Result<(), ActrError> {
        if !r.is_finite() {
            return Err(ActrError::NonFinite);
        }
        let u = self.value(kind, task);
        let next = u + self.alpha * (r - u);
        match self.table.iter_mut().find(|(k, _)| *k == (kind, task)) {
            Some(e) => e.1 = next,
            None => self.table.push(((kind, task), next)),
        }
        Ok(())
    }
}

impl UtilityModel for ActrUtility {
    fn utility(&self, task: ReasoningTask, request: &ReasoningRequest) -> f64 {
        self.value(request.input, task)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::{Cost, Registry, SubroutineSpec};
    use crate::select::{replay, SubroutineSelector, UtilitySelector};
    use crate::task::OutputKind;

    #[test]
    fn rule_matches_the_actr_equation() {
        let mut m = ActrUtility::new(0.2, 0.0).unwrap();
        m.reward(InputKind::Graph, ReasoningTask::Pathfinding, 10.0)
            .unwrap();
        assert_eq!(m.value(InputKind::Graph, ReasoningTask::Pathfinding), 2.0);
        m.reward(InputKind::Graph, ReasoningTask::Pathfinding, 10.0)
            .unwrap();
        assert_eq!(m.value(InputKind::Graph, ReasoningTask::Pathfinding), 3.6);
        // Other keys are untouched.
        assert_eq!(
            m.value(InputKind::SpatialMap, ReasoningTask::Pathfinding),
            0.0
        );
    }

    #[test]
    fn converges_to_a_constant_reward() {
        let mut m = ActrUtility::new(0.2, 0.0).unwrap();
        for _ in 0..200 {
            m.reward(InputKind::Graph, ReasoningTask::GraphAnalysis, 5.0)
                .unwrap();
        }
        assert!((m.value(InputKind::Graph, ReasoningTask::GraphAnalysis) - 5.0).abs() < 1e-12);
    }

    #[test]
    fn rejects_bad_settings() {
        assert_eq!(
            ActrUtility::new(0.0, 0.0),
            Err(ActrError::InvalidLearningRate)
        );
        assert_eq!(
            ActrUtility::new(1.5, 0.0),
            Err(ActrError::InvalidLearningRate)
        );
        assert_eq!(ActrUtility::new(0.2, f64::NAN), Err(ActrError::NonFinite));
        let mut m = ActrUtility::new(0.2, 0.0).unwrap();
        assert_eq!(
            m.reward(
                InputKind::Graph,
                ReasoningTask::GraphAnalysis,
                f64::INFINITY
            ),
            Err(ActrError::NonFinite)
        );
    }

    /// Learning shifts the choice from what the seeded tie-break picks to
    /// the routine that earns reward, and never to an inapplicable one.
    #[test]
    fn rewards_steer_the_selector() {
        let mut registry = Registry::new();
        for (task, accepts) in [
            (ReasoningTask::GraphAnalysis, InputKind::Graph),
            (ReasoningTask::Pathfinding, InputKind::Graph),
            (ReasoningTask::EpistemicEvaluation, InputKind::Proposition),
        ] {
            registry
                .register(SubroutineSpec {
                    task,
                    accepts,
                    returns: OutputKind::Path,
                    cost: Cost::Constant(1),
                    precondition: None,
                })
                .unwrap();
        }
        let req = ReasoningRequest::new(InputKind::Graph, 10);
        let mut model = ActrUtility::new(0.2, 0.0).unwrap();
        // The epistemic routine is rewarded hugely, but it cannot apply.
        for _ in 0..20 {
            model
                .reward(InputKind::Graph, ReasoningTask::EpistemicEvaluation, 100.0)
                .unwrap();
            model
                .reward(InputKind::Graph, ReasoningTask::Pathfinding, 1.0)
                .unwrap();
            model
                .reward(InputKind::Graph, ReasoningTask::GraphAnalysis, -1.0)
                .unwrap();
        }
        let s = UtilitySelector {
            registry,
            model,
            seed: 3,
        };
        let trace = s.select_traced(&req);
        assert_eq!(trace.result, Ok(ReasoningTask::Pathfinding));
        assert!(replay(&s, &trace));
    }
}
