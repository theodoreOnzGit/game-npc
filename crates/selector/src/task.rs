// SPDX-License-Identifier: AGPL-3.0-only

//! The finite vocabulary of subroutines and the typed data they exchange.

/// A reasoning subroutine the selector can choose (issue #4's
/// `ReasoningTask`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReasoningTask {
    EpistemicEvaluation,
    MathematicalCalculation,
    GraphAnalysis,
    GoalPlanning,
    Pathfinding,
}

impl ReasoningTask {
    /// Every task, in stable-id order.
    pub const ALL: [ReasoningTask; 5] = [
        Self::EpistemicEvaluation,
        Self::MathematicalCalculation,
        Self::GraphAnalysis,
        Self::GoalPlanning,
        Self::Pathfinding,
    ];

    /// Stable wire identifier. Never renumber a published id. Ties between
    /// otherwise equal candidates go to the lowest id.
    pub const fn id(self) -> u8 {
        match self {
            Self::EpistemicEvaluation => 0x01,
            Self::MathematicalCalculation => 0x02,
            Self::GraphAnalysis => 0x03,
            Self::GoalPlanning => 0x04,
            Self::Pathfinding => 0x05,
        }
    }

    pub fn from_id(id: u8) -> Option<Self> {
        Self::ALL.into_iter().find(|t| t.id() == id)
    }
}

/// What kind of input a request carries, and what a subroutine accepts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InputKind {
    /// A proposition to evaluate against beliefs.
    Proposition,
    /// A numeric problem: equation, integral, root bracket, …
    NumericProblem,
    /// A graph and a query over it.
    Graph,
    /// A world state, goals and actions to plan over.
    PlanningProblem,
    /// A spatial map with start and goal positions.
    SpatialMap,
}

/// What kind of result a subroutine returns.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OutputKind {
    TruthValue,
    Number,
    GraphResult,
    ActionPlan,
    Path,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_ordered_and_round_trip() {
        for (i, t) in ReasoningTask::ALL.iter().enumerate() {
            assert_eq!(ReasoningTask::from_id(t.id()), Some(*t));
            if i > 0 {
                assert!(ReasoningTask::ALL[i - 1].id() < t.id());
            }
        }
        assert_eq!(ReasoningTask::from_id(0), None);
    }
}
