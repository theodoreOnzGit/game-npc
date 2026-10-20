//! Motivation models: map (previous motivation, state, personality,
//! perception) to the next motivation vector,
//! `m_{t+1} = F(m_t, s_t, p, w_t)`.
//!
//! [`MotivationModel`] is the replaceable interface. The planner and trace
//! code are generic over it, so a Burn FP32 network can be dropped in later
//! without touching the goal planner. A model returns only a
//! [`MotivationVector`]; it receives no handle on the world and so cannot
//! mutate it.
//!
//! [`DeterministicModel`] is the reference baseline the neural model will be
//! compared against.

use crate::motivation::Motivation;
use crate::personality::Personality;
use crate::state::{CharacterState, Perception};
use crate::unit;
use crate::vector::MotivationVector;

/// Everything a motivation model may see.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ModelInput {
    pub previous: MotivationVector,
    pub state: CharacterState,
    pub personality: Personality,
    pub perception: Perception,
}

/// Compile-time contract for a motivation model.
///
/// Implementations used in lockstep simulation must be bit-reproducible:
/// the same input must give the same output bits on every platform.
pub trait MotivationModel {
    fn predict(&self, input: &ModelInput) -> MotivationVector;
}

/// Parameters of the deterministic baseline.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScoringParams {
    /// Fraction of the previous motivation retained each step, in `[0, 1]`.
    /// Non-zero persistence is what lets a motivation outlast its trigger
    /// and persist whether or not it is acted upon.
    pub persistence: f32,
}

impl Default for ScoringParams {
    fn default() -> Self {
        Self { persistence: 0.5 }
    }
}

/// Hand-written baseline: each motivation's target is a *need signal* in
/// `[0, 1]` derived from state and perception, scaled by a personality gain
/// of `2 × weight` (so a neutral 0.5 weight passes the need through
/// unchanged), then blended with the previous value:
///
/// `m_{t+1} = persistence · m_t + (1 − persistence) · clamp(2·w · need)`
///
/// The need formulas are v0.1 engineering heuristics. They are **not**
/// derived from the literature and have **not** been calibrated against any
/// data; see `docs/motivation-taxonomy.md`. Only `+ - *` and `max`/`clamp`
/// are used, so the result is bit-identical across IEEE-754 platforms.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DeterministicModel {
    pub params: ScoringParams,
}

impl DeterministicModel {
    pub fn new(params: ScoringParams) -> Self {
        Self { params }
    }

    /// The need signal for one motivation, before personality and
    /// persistence.
    pub fn need(m: Motivation, s: &CharacterState, w: &Perception) -> f32 {
        use Motivation::*;
        let neg_valence = unit(-s.emotion.valence);
        let raw = match m {
            Sustenance => 1.0 - s.satiety,
            Rest => 1.0 - s.energy,
            Health => 1.0 - s.health,
            Safety => w.threat.max(s.emotion.fear),
            Comfort => 1.0 - s.comfort,
            Curiosity => w.unexplored * s.energy,
            Mastery => 0.5 * s.energy * (1.0 - s.stress),
            Achievement => 0.5 * w.obligation_pending + 0.3 * s.boredom,
            Novelty => s.boredom,
            Connection => s.loneliness * s.social_capacity,
            Belonging => s.loneliness * (1.0 - 0.5 * w.group_present),
            Affection => 0.5 * s.loneliness + 0.5 * w.companion_present * s.social_capacity,
            Recognition => 0.5 * neg_valence + 0.25 * w.others_present,
            Caregiving => w.dependant_in_need,
            Autonomy => w.coercion,
            Control => 0.5 * (w.coercion + w.disorder),
            Order => w.disorder,
            Acquisition => w.resources_available,
            Duty => w.obligation_pending,
            Justice => w.injustice,
            SelfExpression => 0.5 * s.boredom * s.emotional_battery,
            Meaning => 0.5 * neg_valence * (1.0 - s.boredom * 0.5),
            Pleasure => 0.5 * s.boredom + 0.5 * neg_valence,
            StressRelief => s.stress,
            EmotionalRecovery => 1.0 - s.emotional_battery,
        };
        unit(raw)
    }
}

impl MotivationModel for DeterministicModel {
    fn predict(&self, input: &ModelInput) -> MotivationVector {
        let s = input.state.clamped();
        let w = input.perception.clamped();
        let keep = unit(self.params.persistence);
        let mut out = MotivationVector::zero();
        for m in Motivation::ALL {
            let gain = 2.0 * input.personality.weight(m);
            let target = unit(gain * Self::need(m, &s, &w));
            out.set(m, keep * input.previous.get(m) + (1.0 - keep) * target);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(
        state: CharacterState,
        personality: Personality,
        perception: Perception,
    ) -> ModelInput {
        ModelInput {
            previous: MotivationVector::zero(),
            state,
            personality,
            perception,
        }
    }

    #[test]
    fn hunger_drives_sustenance() {
        let s = CharacterState {
            satiety: 0.1,
            ..CharacterState::default()
        };
        let v = DeterministicModel::default().predict(&input(
            s,
            Personality::neutral(),
            Perception::default(),
        ));
        assert_eq!(v.top(), Motivation::Sustenance);
    }

    #[test]
    fn personality_changes_priorities_for_same_situation() {
        let s = CharacterState {
            boredom: 0.6,
            ..CharacterState::default()
        };
        let w = Perception {
            unexplored: 0.6,
            dependant_in_need: 0.6,
            ..Perception::default()
        };
        let scholar = Personality::neutral().with(Motivation::Curiosity, 1.0);
        let parent = Personality::neutral().with(Motivation::Caregiving, 1.0);
        let m = DeterministicModel::default();
        assert_eq!(
            m.predict(&input(s, scholar, w)).top(),
            Motivation::Curiosity
        );
        assert_eq!(
            m.predict(&input(s, parent, w)).top(),
            Motivation::Caregiving
        );
    }

    #[test]
    fn motivation_persists_after_trigger_is_gone() {
        let m = DeterministicModel::default();
        let threatened = Perception {
            threat: 1.0,
            ..Perception::default()
        };
        let first = m.predict(&input(
            CharacterState::default(),
            Personality::neutral(),
            threatened,
        ));
        let mut next_in = input(
            CharacterState::default(),
            Personality::neutral(),
            Perception::default(),
        );
        next_in.previous = first;
        let second = m.predict(&next_in);
        assert!(second.get(Motivation::Safety) > 0.0);
        assert!(second.get(Motivation::Safety) < first.get(Motivation::Safety));
    }

    #[test]
    fn conflicting_motivations_are_both_active() {
        // Hungry and threatened: both wants are present at once.
        let s = CharacterState {
            satiety: 0.0,
            ..CharacterState::default()
        };
        let w = Perception {
            threat: 1.0,
            ..Perception::default()
        };
        let v = DeterministicModel::default().predict(&input(s, Personality::neutral(), w));
        assert!(v.get(Motivation::Sustenance) > 0.4);
        assert!(v.get(Motivation::Safety) > 0.4);
    }

    #[test]
    fn every_need_is_in_unit_range_at_extremes() {
        let lo = CharacterState {
            health: 0.0,
            energy: 0.0,
            satiety: 0.0,
            comfort: 0.0,
            emotional_battery: 0.0,
            social_capacity: 0.0,
            stress: 1.0,
            boredom: 1.0,
            loneliness: 1.0,
            emotion: crate::EmotionState {
                valence: -1.0,
                arousal: 1.0,
                fear: 1.0,
            },
        };
        let hi = CharacterState::default();
        let all = Perception {
            threat: 1.0,
            others_present: 1.0,
            group_present: 1.0,
            companion_present: 1.0,
            dependant_in_need: 1.0,
            unexplored: 1.0,
            obligation_pending: 1.0,
            injustice: 1.0,
            coercion: 1.0,
            disorder: 1.0,
            resources_available: 1.0,
        };
        for s in [lo, hi] {
            for w in [all, Perception::default()] {
                for m in Motivation::ALL {
                    let n = DeterministicModel::need(m, &s, &w);
                    assert!((0.0..=1.0).contains(&n), "{m:?} = {n}");
                }
            }
        }
    }
}
