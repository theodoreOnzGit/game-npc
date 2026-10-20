// SPDX-License-Identifier: AGPL-3.0-only
// Translated from goapai `astar.go` (https://github.com/akmonengine/goapai,
// commit 2e86c5e, Apache-2.0, Copyright 2024 Agoune Ramine). Modified: see
// docs/DEVIATIONS.md.

//! Forward A* over world states (`astar.go`).
//!
//! Semantics preserved from upstream:
//!
//! - Only the *open* set is searched for duplicates (by world hash). An
//!   expanded node leaves the heap and is not remembered, so the same world
//!   can be reached and expanded again. Search is bounded by `max_depth` and
//!   by non-repeatable actions.
//! - A node deeper than `max_depth` is discarded when popped, *before* the
//!   goal test, so plans have at most `max_depth` real actions.
//! - Actions whose effects are all already true in the *initial* world are
//!   dropped up front ([`get_impacting_actions`]).
//! - Successors are generated in action order; equal-cost ties follow the
//!   ported `container/heap` order.
//! - The returned plan starts with the root node's empty action.

use alloc::sync::Arc;
use alloc::vec::Vec;

use crate::action::{apply_effects, satisfy_states, Action, Actions};
use crate::agent::GoalInterface;
use crate::heap::NodeHeap;
use crate::planer::Plan;
use crate::state::{check_conditions, Sensors, World};

/// One search node (Go: `node`). Nodes live in an arena; `parent` and the
/// heap refer to them by index.
#[derive(Clone, Debug)]
pub(crate) struct Node {
    pub(crate) action: Arc<Action>,
    pub(crate) world: World,
    pub(crate) parent: Option<usize>,
    pub(crate) cost: f32,
    pub(crate) total_cost: f32,
    pub(crate) heuristic: f32,
    /// `u32` rather than Go's `uint16`, so depth cannot wrap.
    pub(crate) depth: u32,
    pub(crate) heap_index: Option<usize>,
}

impl Node {
    #[cfg(test)]
    pub(crate) fn with_cost(total_cost: f32) -> Self {
        Self {
            action: Arc::new(Action::default()),
            world: World::new(),
            parent: None,
            cost: 0.0,
            total_cost,
            heuristic: 0.0,
            depth: 0,
            heap_index: None,
        }
    }
}

/// Plan from `from` to `goal` using at most `max_depth` actions. Returns an
/// empty plan if the goal is unreachable within that depth.
pub(crate) fn astar(
    from: &World,
    goal: &GoalInterface,
    actions: &Actions,
    max_depth: u16,
    sensors: &Sensors,
) -> Plan {
    let available = get_impacting_actions(from, actions);
    let max_depth = u32::from(max_depth);

    let mut nodes: Vec<Node> = Vec::new();
    let mut heap = NodeHeap::default();
    nodes.push(Node {
        action: Arc::new(Action::default()),
        world: from.clone(),
        parent: None,
        cost: 0.0,
        total_cost: 0.0,
        heuristic: 0.0,
        depth: 0,
        heap_index: None,
    });
    heap.push(&mut nodes, 0);

    while heap.len() > 0 {
        let parent = heap.pop(&mut nodes);

        if nodes[parent].depth > max_depth {
            continue;
        }

        if count_missing_goal(goal, &nodes[parent].world, sensors) == 0 {
            return build_plan_from_node(&nodes, parent);
        }

        for action in available.iter() {
            if !allowed_repetition(&nodes, action, parent) {
                continue;
            }
            if !check_conditions(&action.conditions, &nodes[parent].world, sensors) {
                continue;
            }
            let Some(simulated) = simulate_action_state(action, &nodes[parent].world) else {
                continue;
            };

            let cost = nodes[parent].cost + action.cost;
            let depth = nodes[parent].depth + 1;
            match fetch_node_in_heap(&heap, &nodes, &simulated) {
                Some(existing) => {
                    if cost < nodes[existing].cost {
                        let n = &mut nodes[existing];
                        n.action = Arc::clone(action);
                        n.world = simulated;
                        n.parent = Some(parent);
                        n.cost = cost;
                        n.total_cost = cost + n.heuristic;
                        n.depth = depth;
                        let i = n.heap_index.expect("open node is in the heap");
                        heap.fix(&mut nodes, i);
                    }
                }
                None => {
                    let heuristic = compute_heuristic(goal, &simulated, sensors);
                    nodes.push(Node {
                        action: Arc::clone(action),
                        world: simulated,
                        parent: Some(parent),
                        cost,
                        total_cost: cost + heuristic,
                        heuristic,
                        depth,
                        heap_index: None,
                    });
                    let n = nodes.len() - 1;
                    heap.push(&mut nodes, n);
                }
            }
        }
    }

    Plan::default()
}

/// Actions whose effects are all already true in the initial world cannot
/// lead anywhere new and are dropped (Go: `getImpactingActions`).
pub(crate) fn get_impacting_actions(from: &World, actions: &Actions) -> Actions {
    Actions(
        actions
            .iter()
            .filter(|a| !satisfy_states(&a.effects, from))
            .cloned()
            .collect(),
    )
}

/// First open node whose world hash equals `w`'s (Go: `fetchNodeInHeap`).
pub(crate) fn fetch_node_in_heap(heap: &NodeHeap, nodes: &[Node], w: &World) -> Option<usize> {
    heap.items
        .iter()
        .copied()
        .find(|&n| nodes[n].world.check(w))
}

/// Walk parents back to the root and reverse (Go: `buildPlanFromNode`).
pub(crate) fn build_plan_from_node(nodes: &[Node], node: usize) -> Plan {
    let mut plan = Vec::with_capacity(nodes[node].depth as usize + 1);
    let mut cur = Some(node);
    while let Some(n) = cur {
        plan.push(Arc::clone(&nodes[n].action));
        cur = nodes[n].parent;
    }
    plan.reverse();
    Plan(plan)
}

/// The world after applying `action`'s effects, or `None` if they change
/// nothing or fail (Go: `simulateActionState`).
pub(crate) fn simulate_action_state(action: &Action, w: &World) -> Option<World> {
    if satisfy_states(&action.effects, w) {
        return None;
    }
    let mut next = w.clone();
    apply_effects(&action.effects, &mut next).ok()?;
    Some(next)
}

/// A non-repeatable action may appear once on the path from the root,
/// compared by name (Go: `allowedRepetition`).
pub(crate) fn allowed_repetition(nodes: &[Node], action: &Action, parent: usize) -> bool {
    if action.repeatable {
        return true;
    }
    let mut cur = Some(parent);
    while let Some(n) = cur {
        if nodes[n].action.name == action.name {
            return false;
        }
        cur = nodes[n].parent;
    }
    true
}

/// Number of goal conditions not yet met (Go: `countMissingGoal`).
pub(crate) fn count_missing_goal(goal: &GoalInterface, w: &World, sensors: &Sensors) -> usize {
    goal.conditions
        .iter()
        .filter(|c| !c.check(w, sensors))
        .count()
}

/// Sum over goal conditions of the numeric distance to satisfying them;
/// 1 for an unmet condition on a missing key (Go: `computeHeuristic`).
pub(crate) fn compute_heuristic(goal: &GoalInterface, w: &World, sensors: &Sensors) -> f32 {
    let mut total: f32 = 0.0;
    for condition in &goal.conditions {
        match w.get(condition.get_key()) {
            Some(state) => total += state.distance(condition),
            None => {
                if !condition.check(w, sensors) {
                    total += 1.0;
                }
            }
        }
    }
    total
}

#[cfg(test)]
#[path = "astar_test.rs"]
mod astar_test;
