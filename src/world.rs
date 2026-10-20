//! A minimal, integer-valued local world for one character.
//!
//! This stands in for the host game (e.g. the SAGE RTS port) until that
//! integration exists. Counts are integers so that world transitions are
//! exact and lockstep-safe. The host game is expected to supply its own
//! world and its own `perceive`; this type defines the shape of that
//! contract and gives the tests something to act on.

use crate::action::{Action, SocialTarget, ValidatedAction};
use crate::appraisal::WorldEvent;
use crate::state::Perception;

/// Saturation point for integer counts when converting to perception.
pub const PERCEPTION_SATURATION: u32 = 4;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct WorldState {
    /// Units of food within reach.
    pub food: u32,
    pub bed_available: bool,
    pub shelter_available: bool,
    pub escape_route: bool,
    /// Threat level; 0 = none.
    pub threat: u32,
    pub others_nearby: u32,
    pub group_nearby: bool,
    pub companion_nearby: bool,
    pub dependant_in_need: bool,
    pub unexplored_sites: u32,
    /// Harvestable resources nearby.
    pub resources: u32,
    /// Resources the character carries.
    pub inventory: u32,
    pub pending_tasks: u32,
    pub wrongdoer_nearby: bool,
    pub coerced: bool,
    /// Amount of disorder; 0 = tidy.
    pub disorder: u32,
}

fn level(n: u32) -> f32 {
    let n = n.min(PERCEPTION_SATURATION);
    // Exact: small integers are represented exactly and the divisor is fixed.
    n as f32 / PERCEPTION_SATURATION as f32
}

fn flag(b: bool) -> f32 {
    if b {
        1.0
    } else {
        0.0
    }
}

impl WorldState {
    /// Reference perception: what a character standing here would notice.
    pub fn perceive(&self) -> Perception {
        Perception {
            threat: level(self.threat),
            others_present: level(self.others_nearby),
            group_present: flag(self.group_nearby),
            companion_present: flag(self.companion_nearby),
            dependant_in_need: flag(self.dependant_in_need),
            unexplored: level(self.unexplored_sites),
            obligation_pending: level(self.pending_tasks),
            injustice: flag(self.wrongdoer_nearby),
            coercion: flag(self.coerced),
            disorder: level(self.disorder),
            resources_available: level(self.resources),
        }
    }

    /// Apply a validated action. Returns the new world and the event the
    /// acting character experiences, which feeds back into appraisal.
    pub fn apply(&self, va: &ValidatedAction) -> (WorldState, WorldEvent) {
        let mut w = *self;
        let ev = match va.action() {
            Action::Eat => {
                w.food -= 1;
                WorldEvent::Ate { amount: 0.5 }
            }
            Action::Sleep => WorldEvent::Rested { amount: 0.5 },
            Action::Flee => {
                w.threat = 0;
                WorldEvent::TimePassed { ticks: 1 }
            }
            Action::TakeShelter => WorldEvent::Sheltered { amount: 0.5 },
            Action::Explore => {
                w.unexplored_sites -= 1;
                WorldEvent::Enjoyed { amount: 0.3 }
            }
            Action::Practice | Action::Create => WorldEvent::Enjoyed { amount: 0.2 },
            Action::Work => {
                w.pending_tasks -= 1;
                WorldEvent::Praised { amount: 0.2 }
            }
            Action::Talk(SocialTarget::Companion) => WorldEvent::Socialised { amount: 0.5 },
            Action::Talk(_) => WorldEvent::Socialised { amount: 0.3 },
            Action::Perform => WorldEvent::Praised { amount: 0.4 },
            Action::Help => {
                w.dependant_in_need = false;
                WorldEvent::Praised { amount: 0.3 }
            }
            Action::Refuse => {
                w.coerced = false;
                WorldEvent::TimePassed { ticks: 1 }
            }
            Action::Organise | Action::Tidy => {
                w.disorder -= 1;
                WorldEvent::TimePassed { ticks: 1 }
            }
            Action::Gather => {
                w.resources -= 1;
                w.inventory += 1;
                WorldEvent::TimePassed { ticks: 1 }
            }
            Action::Confront => {
                w.wrongdoer_nearby = false;
                WorldEvent::TimePassed { ticks: 1 }
            }
            Action::Withdraw => WorldEvent::Rested { amount: 0.2 },
            Action::Relax => WorldEvent::Enjoyed { amount: 0.3 },
        };
        (w, ev)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::validate;

    #[test]
    fn every_valid_action_applies_without_underflow() {
        // A world where every action is valid.
        let w = WorldState {
            food: 1,
            bed_available: true,
            shelter_available: true,
            escape_route: true,
            threat: 0,
            others_nearby: 1,
            group_nearby: true,
            companion_nearby: true,
            dependant_in_need: true,
            unexplored_sites: 1,
            resources: 1,
            inventory: 0,
            pending_tasks: 1,
            wrongdoer_nearby: true,
            coerced: true,
            disorder: 1,
        };
        let actions = [
            Action::Eat,
            Action::Sleep,
            Action::Flee,
            Action::TakeShelter,
            Action::Explore,
            Action::Practice,
            Action::Work,
            Action::Talk(SocialTarget::Anyone),
            Action::Talk(SocialTarget::Group),
            Action::Talk(SocialTarget::Companion),
            Action::Perform,
            Action::Help,
            Action::Refuse,
            Action::Organise,
            Action::Tidy,
            Action::Gather,
            Action::Confront,
            Action::Create,
            Action::Withdraw,
            Action::Relax,
        ];
        for a in actions {
            let va = validate(a, &w).unwrap_or_else(|e| panic!("{a:?}: {e:?}"));
            let _ = w.apply(&va);
        }
    }

    #[test]
    fn perception_saturates() {
        let w = WorldState {
            threat: 100,
            others_nearby: 2,
            ..WorldState::default()
        };
        let p = w.perceive();
        assert_eq!(p.threat, 1.0);
        assert_eq!(p.others_present, 0.5);
    }
}
