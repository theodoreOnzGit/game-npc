// SPDX-License-Identifier: AGPL-3.0-only
// Translated from goapai `agent.go` (https://github.com/akmonengine/goapai,
// commit 2e86c5e, Apache-2.0, Copyright 2024 Agoune Ramine). Modified: see
// docs/DEVIATIONS.md.

//! Agents and goals (`agent.go`).

use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::any::Any;

use crate::action::Actions;
use crate::state::{Conditions, Sensors, State, StateKey, StateValue, World};

/// Goal priority function (Go: `GoalPriorityFn`).
pub type GoalPriorityFn = Arc<dyn Fn(&Sensors) -> f32 + Send + Sync>;

/// A goal: the conditions that achieve it and how urgent it is
/// (Go: `goalInterface`).
#[derive(Clone)]
pub struct GoalInterface {
    pub conditions: Conditions,
    pub priority_fn: GoalPriorityFn,
}

impl core::fmt::Debug for GoalInterface {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("GoalInterface")
            .field("conditions", &self.conditions)
            .finish_non_exhaustive()
    }
}

impl GoalInterface {
    pub fn new(
        conditions: Conditions,
        priority_fn: impl Fn(&Sensors) -> f32 + Send + Sync + 'static,
    ) -> Self {
        Self {
            conditions,
            priority_fn: Arc::new(priority_fn),
        }
    }
}

/// Goal identifier (Go: `GoalName`).
pub type GoalName = String;

/// Named goals in insertion order (Go: `Goals`, a `map[GoalName]goalInterface`).
///
/// Insertion order is what makes goal selection deterministic on priority
/// ties; Go's map order is random. Inserting an existing name replaces that
/// goal in place, as a map would.
#[derive(Clone, Debug, Default)]
pub struct Goals {
    entries: Vec<(GoalName, GoalInterface)>,
}

impl Goals {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, name: &str, goal: GoalInterface) {
        match self.entries.iter_mut().find(|(n, _)| n == name) {
            Some(entry) => entry.1 = goal,
            None => self.entries.push((String::from(name), goal)),
        }
    }

    /// Builder form of [`Goals::insert`].
    pub fn with(mut self, name: &str, goal: GoalInterface) -> Self {
        self.insert(name, goal);
        self
    }

    pub fn get(&self, name: &str) -> Option<&GoalInterface> {
        self.entries.iter().find(|(n, _)| n == name).map(|(_, g)| g)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&GoalName, &GoalInterface)> {
        self.entries.iter().map(|(n, g)| (n, g))
    }
}

/// An agent: world state, goals, actions and sensors (Go: `Agent`).
#[derive(Clone, Debug, Default)]
pub struct Agent {
    pub(crate) actions: Actions,
    pub(crate) w: World,
    pub(crate) sensors: Sensors,
    pub(crate) goals: Goals,
}

impl Agent {
    pub fn actions(&self) -> &Actions {
        &self.actions
    }

    pub fn world(&self) -> &World {
        &self.w
    }

    pub fn sensors(&self) -> &Sensors {
        &self.sensors
    }

    pub fn goals(&self) -> &Goals {
        &self.goals
    }
}

/// Go: `CreateAgent`. Starts with an empty world and no sensors.
pub fn create_agent(goals: Goals, actions: Actions) -> Agent {
    Agent {
        actions,
        w: World::new(),
        sensors: Sensors::new(),
        goals,
    }
}

/// Go: `SetState`. Adds the state, or updates it if the key exists.
pub fn set_state<T: StateValue>(agent: &mut Agent, key: StateKey, value: T) {
    agent.w.store(State::new(key, value));
}

/// Go: `SetSensor`.
pub fn set_sensor<T: Any + Send + Sync>(agent: &mut Agent, name: &str, value: T) {
    agent.sensors.set(name, value);
}

#[cfg(test)]
#[path = "agent_test.rs"]
mod agent_test;
