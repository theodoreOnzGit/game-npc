// SPDX-License-Identifier: AGPL-3.0-only
// Translated from goapai `planer.go` (https://github.com/akmonengine/goapai,
// commit 2e86c5e, Apache-2.0, Copyright 2024 Agoune Ramine). Modified: see
// docs/DEVIATIONS.md.

//! Goal selection and the planning entry point (`planer.go`; the upstream
//! file name is kept for traceability).

use alloc::sync::Arc;
use alloc::vec::Vec;
use core::ops::Deref;

use crate::action::{Action, GoapError};
use crate::agent::{Agent, GoalName};
use crate::astar::astar;

/// An ordered list of actions. Index 0 is the root's empty action
/// (Go: `Plan`).
#[derive(Clone, Debug, Default)]
pub struct Plan(pub Vec<Arc<Action>>);

impl Plan {
    /// Sum of the actions' costs, in plan order (Go: `Plan.GetTotalCost`).
    pub fn get_total_cost(&self) -> f32 {
        let mut cost: f32 = 0.0;
        for action in &self.0 {
            cost += action.cost;
        }
        cost
    }

    /// The real actions to execute, i.e. without the root's empty action.
    pub fn steps(&self) -> &[Arc<Action>] {
        self.0.get(1..).unwrap_or(&[])
    }
}

impl Deref for Plan {
    type Target = Vec<Arc<Action>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// The highest-priority goal and the cheapest plan to reach it within
/// `max_depth` actions (Go: `GetPlan`).
///
/// Returns `("", empty)` when no goal has positive priority, and
/// `(goal, empty)` when the goal is unreachable. Use [`try_get_plan`] to tell
/// those apart.
pub fn get_plan(agent: &Agent, max_depth: u16) -> (GoalName, Plan) {
    try_get_plan(agent, max_depth).unwrap_or_default()
}

/// [`get_plan`], with "no goal available" reported as an error.
pub fn try_get_plan(agent: &Agent, max_depth: u16) -> Result<(GoalName, Plan), GoapError> {
    let goal_name = agent.get_prioritized_goal_name()?;
    reset_condition_caches(agent);
    let goal = agent.goals.get(&goal_name).expect("selected goal exists");
    let plan = astar(&agent.w, goal, &agent.actions, max_depth, &agent.sensors);
    Ok((goal_name, plan))
}

/// Procedural conditions resolve once per planning request.
fn reset_condition_caches(agent: &Agent) {
    for (_, goal) in agent.goals.iter() {
        goal.conditions.iter().for_each(|c| c.reset_cache());
    }
    for action in agent.actions.iter() {
        action.conditions.iter().for_each(|c| c.reset_cache());
    }
}

impl Agent {
    /// The goal with the highest strictly positive priority. Ties go to the
    /// goal inserted first (Go: `getPrioritizedGoalName`).
    pub fn get_prioritized_goal_name(&self) -> Result<GoalName, GoapError> {
        let mut best: Option<&GoalName> = None;
        let mut best_value: f32 = 0.0;
        for (name, goal) in self.goals.iter() {
            let priority = (goal.priority_fn)(&self.sensors);
            if priority > best_value {
                best = Some(name);
                best_value = priority;
            }
        }
        best.cloned().ok_or(GoapError::NoGoalAvailable)
    }
}

#[cfg(test)]
#[path = "planer_test.rs"]
mod planer_test;
