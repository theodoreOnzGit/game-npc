// SPDX-License-Identifier: AGPL-3.0-only

//! The typed subroutine registry and the requests it is matched against.

use alloc::vec::Vec;

use crate::task::{InputKind, OutputKind, ReasoningTask};

/// Estimated cost of running a subroutine, in abstract work units, as a
/// function of request size.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cost {
    Constant(u32),
    /// `base + per_item * size`, saturating.
    Linear {
        base: u32,
        per_item: u32,
    },
}

impl Cost {
    pub fn estimate(self, size: u32) -> u32 {
        match self {
            Cost::Constant(c) => c,
            Cost::Linear { base, per_item } => base.saturating_add(per_item.saturating_mul(size)),
        }
    }
}

/// A precondition on the request beyond its input kind. A plain function
/// pointer, so specs stay `Copy` and comparable.
pub type Precondition = fn(&ReasoningRequest) -> bool;

/// One registered subroutine: what it accepts, what it returns, what it
/// costs, and when it applies.
#[derive(Clone, Copy, Debug)]
pub struct SubroutineSpec {
    pub task: ReasoningTask,
    pub accepts: InputKind,
    pub returns: OutputKind,
    pub cost: Cost,
    pub precondition: Option<Precondition>,
}

/// The available subroutines (at most one spec per task).
#[derive(Clone, Debug, Default)]
pub struct Registry {
    specs: Vec<SubroutineSpec>,
}

/// Why a registration was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegistryError {
    AlreadyRegistered(ReasoningTask),
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, spec: SubroutineSpec) -> Result<(), RegistryError> {
        if self.get(spec.task).is_some() {
            return Err(RegistryError::AlreadyRegistered(spec.task));
        }
        self.specs.push(spec);
        self.specs.sort_by_key(|s| s.task.id());
        Ok(())
    }

    pub fn get(&self, task: ReasoningTask) -> Option<&SubroutineSpec> {
        self.specs.iter().find(|s| s.task == task)
    }

    /// Specs in stable-id order.
    pub fn specs(&self) -> &[SubroutineSpec] {
        &self.specs
    }
}

/// A structured reasoning request from a higher-level reasoner.
#[derive(Clone, Debug, PartialEq)]
pub struct ReasoningRequest {
    pub input: InputKind,
    /// Problem size (nodes, actions, unknowns, …), used for cost estimates.
    pub size: u32,
    /// Cost limit. Routines estimated above it are not applicable.
    pub budget: Option<u32>,
    /// Expected output kind, if the caller needs a particular one.
    pub expects: Option<OutputKind>,
    /// Must be this routine (Soar: require).
    pub require: Option<ReasoningTask>,
    /// Must not be these routines (Soar: prohibit).
    pub prohibit: Vec<ReasoningTask>,
    /// `(a, b)`: prefer `a` over `b` (Soar: better).
    pub prefer: Vec<(ReasoningTask, ReasoningTask)>,
}

impl ReasoningRequest {
    pub fn new(input: InputKind, size: u32) -> Self {
        Self {
            input,
            size,
            budget: None,
            expects: None,
            require: None,
            prohibit: Vec::new(),
            prefer: Vec::new(),
        }
    }
}

/// Why a registered routine is or is not a candidate for a request.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Applicability {
    Applicable,
    WrongInput,
    WrongOutput,
    PreconditionFailed,
    OverBudget { estimate: u32, budget: u32 },
}

impl SubroutineSpec {
    /// Symbolic applicability: input and output kinds, precondition, and
    /// budget.
    pub fn applicability(&self, req: &ReasoningRequest) -> Applicability {
        if self.accepts != req.input {
            return Applicability::WrongInput;
        }
        if req.expects.is_some_and(|o| o != self.returns) {
            return Applicability::WrongOutput;
        }
        if let Some(pre) = self.precondition {
            if !pre(req) {
                return Applicability::PreconditionFailed;
            }
        }
        if let Some(budget) = req.budget {
            let estimate = self.cost.estimate(req.size);
            if estimate > budget {
                return Applicability::OverBudget { estimate, budget };
            }
        }
        Applicability::Applicable
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(task: ReasoningTask, accepts: InputKind) -> SubroutineSpec {
        SubroutineSpec {
            task,
            accepts,
            returns: OutputKind::Number,
            cost: Cost::Linear {
                base: 10,
                per_item: 2,
            },
            precondition: None,
        }
    }

    #[test]
    fn registry_rejects_duplicates_and_keeps_id_order() {
        let mut r = Registry::new();
        r.register(spec(ReasoningTask::Pathfinding, InputKind::SpatialMap))
            .unwrap();
        r.register(spec(
            ReasoningTask::EpistemicEvaluation,
            InputKind::Proposition,
        ))
        .unwrap();
        assert_eq!(
            r.register(spec(ReasoningTask::Pathfinding, InputKind::Graph)),
            Err(RegistryError::AlreadyRegistered(ReasoningTask::Pathfinding))
        );
        let ids: Vec<u8> = r.specs().iter().map(|s| s.task.id()).collect();
        assert_eq!(ids, [0x01, 0x05]);
    }

    #[test]
    fn applicability_checks_in_order() {
        let s = SubroutineSpec {
            precondition: Some(|r| r.size > 0),
            ..spec(ReasoningTask::GraphAnalysis, InputKind::Graph)
        };
        let mut req = ReasoningRequest::new(InputKind::Graph, 5);
        assert_eq!(s.applicability(&req), Applicability::Applicable);
        req.budget = Some(19);
        assert_eq!(
            s.applicability(&req),
            Applicability::OverBudget {
                estimate: 20,
                budget: 19
            }
        );
        req.size = 0;
        assert_eq!(s.applicability(&req), Applicability::PreconditionFailed);
        req.expects = Some(OutputKind::Path);
        assert_eq!(s.applicability(&req), Applicability::WrongOutput);
        req.input = InputKind::Proposition;
        assert_eq!(s.applicability(&req), Applicability::WrongInput);
    }

    #[test]
    fn cost_saturates() {
        assert_eq!(
            Cost::Linear {
                base: 1,
                per_item: u32::MAX
            }
            .estimate(3),
            u32::MAX
        );
        assert_eq!(Cost::Constant(7).estimate(1000), 7);
    }
}
