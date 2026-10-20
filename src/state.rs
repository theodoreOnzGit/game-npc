//! Character state (resources and emotion) and perceived environment.
//!
//! These are kept apart from motivation on purpose: a resource level is a
//! fact about the body or mind ("energy is low"), an emotion is an affective
//! state ("afraid"), and a motivation is a want ("to rest"). Appraisal moves
//! resources and emotion; the motivation model reads them.

use crate::unit;

/// Affective state on a valence/arousal plane plus fear, each clamped.
///
/// `valence` is in `[-1, 1]`; `arousal` and `fear` are in `[0, 1]`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EmotionState {
    pub valence: f32,
    pub arousal: f32,
    pub fear: f32,
}

impl Default for EmotionState {
    fn default() -> Self {
        Self {
            valence: 0.0,
            arousal: 0.3,
            fear: 0.0,
        }
    }
}

impl EmotionState {
    /// Clamp every field to its range; NaN becomes the range's zero.
    pub fn clamped(self) -> Self {
        let valence = if self.valence.is_nan() {
            0.0
        } else {
            self.valence.clamp(-1.0, 1.0)
        };
        Self {
            valence,
            arousal: unit(self.arousal),
            fear: unit(self.fear),
        }
    }
}

/// Physiological and psychological resources, each in `[0, 1]`
/// (1 = full / high).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CharacterState {
    /// Physical health; 1 = uninjured.
    pub health: f32,
    /// Physical energy; 1 = fully rested.
    pub energy: f32,
    /// Satiety; 1 = fully fed.
    pub satiety: f32,
    /// Physical comfort (warmth, shelter); 1 = fully comfortable.
    pub comfort: f32,
    /// Emotional battery; 1 = fully recovered.
    pub emotional_battery: f32,
    /// Capacity for social interaction; 1 = eager, 0 = socially drained.
    pub social_capacity: f32,
    /// Accumulated stress; 1 = overwhelmed.
    pub stress: f32,
    /// Boredom; 1 = extremely bored.
    pub boredom: f32,
    /// Loneliness; 1 = completely isolated.
    pub loneliness: f32,
    pub emotion: EmotionState,
}

impl Default for CharacterState {
    fn default() -> Self {
        Self {
            health: 1.0,
            energy: 1.0,
            satiety: 1.0,
            comfort: 1.0,
            emotional_battery: 1.0,
            social_capacity: 1.0,
            stress: 0.0,
            boredom: 0.0,
            loneliness: 0.0,
            emotion: EmotionState::default(),
        }
    }
}

impl CharacterState {
    /// Clamp every field to its range.
    pub fn clamped(self) -> Self {
        Self {
            health: unit(self.health),
            energy: unit(self.energy),
            satiety: unit(self.satiety),
            comfort: unit(self.comfort),
            emotional_battery: unit(self.emotional_battery),
            social_capacity: unit(self.social_capacity),
            stress: unit(self.stress),
            boredom: unit(self.boredom),
            loneliness: unit(self.loneliness),
            emotion: self.emotion.clamped(),
        }
    }
}

/// What the character currently perceives about its surroundings, each in
/// `[0, 1]`. Produced by the host game from world state; the motivation model
/// sees only this, never the world itself.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Perception {
    /// Perceived physical threat nearby.
    pub threat: f32,
    /// Presence of other friendly characters to interact with.
    pub others_present: f32,
    /// Presence of the character's group (faction, family, guild).
    pub group_present: f32,
    /// Presence of a close companion (friend, partner).
    pub companion_present: f32,
    /// Someone dependent on the character is in need.
    pub dependant_in_need: f32,
    /// Something unknown and investigable is nearby.
    pub unexplored: f32,
    /// The character holds an accepted, unfinished obligation.
    pub obligation_pending: f32,
    /// The character perceives an injustice.
    pub injustice: f32,
    /// The character's choices are being constrained by others.
    pub coercion: f32,
    /// Surroundings are disordered or unpredictable.
    pub disorder: f32,
    /// Valuable, obtainable resources are nearby.
    pub resources_available: f32,
}

impl Perception {
    /// Clamp every field to `[0, 1]`.
    pub fn clamped(self) -> Self {
        Self {
            threat: unit(self.threat),
            others_present: unit(self.others_present),
            group_present: unit(self.group_present),
            companion_present: unit(self.companion_present),
            dependant_in_need: unit(self.dependant_in_need),
            unexplored: unit(self.unexplored),
            obligation_pending: unit(self.obligation_pending),
            injustice: unit(self.injustice),
            coercion: unit(self.coercion),
            disorder: unit(self.disorder),
            resources_available: unit(self.resources_available),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamped_bounds_every_field() {
        let s = CharacterState {
            health: 2.0,
            stress: -1.0,
            energy: f32::NAN,
            emotion: EmotionState {
                valence: -5.0,
                arousal: 9.0,
                fear: -1.0,
            },
            ..CharacterState::default()
        }
        .clamped();
        assert_eq!(s.health, 1.0);
        assert_eq!(s.stress, 0.0);
        assert_eq!(s.energy, 0.0);
        assert_eq!(s.emotion.valence, -1.0);
        assert_eq!(s.emotion.arousal, 1.0);
        assert_eq!(s.emotion.fear, 0.0);

        let p = Perception {
            threat: 3.0,
            ..Perception::default()
        }
        .clamped();
        assert_eq!(p.threat, 1.0);
    }
}
