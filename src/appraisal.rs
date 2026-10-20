//! Deterministic appraisal: how a world event changes a character's state.
//!
//! Appraisal is where personality first meets an event. The same event can
//! hit two characters differently: for example, a threat raises fear more in
//! a character with a strong Safety disposition. Every update is plain
//! `+ - *` arithmetic followed by a clamp, which keeps it bit-identical across
//! platforms (required for lockstep simulation).

use crate::motivation::Motivation;
use crate::personality::Personality;
use crate::state::CharacterState;
use crate::unit;

/// A discrete event the character experiences. Magnitudes are in `[0, 1]`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WorldEvent {
    /// Simulation time passes; resources drift.
    TimePassed { ticks: u32 },
    /// The character is attacked or injured.
    Attacked { severity: f32 },
    /// The character eats.
    Ate { amount: f32 },
    /// The character sleeps or rests.
    Rested { amount: f32 },
    /// The character finds shelter or warmth.
    Sheltered { amount: f32 },
    /// The character spends time with others.
    Socialised { amount: f32 },
    /// The character is praised or acknowledged.
    Praised { amount: f32 },
    /// The character is insulted or humiliated.
    Insulted { amount: f32 },
    /// The character does something enjoyable or novel.
    Enjoyed { amount: f32 },
}

/// Per-tick drift rates used by [`WorldEvent::TimePassed`].
pub mod drift {
    pub const SATIETY: f32 = 0.002;
    pub const ENERGY: f32 = 0.001;
    pub const BOREDOM: f32 = 0.002;
    pub const LONELINESS: f32 = 0.001;
    /// Fraction of the remaining distance to baseline recovered per tick.
    pub const SOCIAL_RECOVERY: f32 = 0.002;
    pub const STRESS_DECAY: f32 = 0.002;
    pub const FEAR_DECAY: f32 = 0.01;
}

/// Apply one event to a character's state.
pub fn appraise(
    state: &CharacterState,
    personality: &Personality,
    event: &WorldEvent,
) -> CharacterState {
    let mut s = *state;
    match *event {
        WorldEvent::TimePassed { ticks } => {
            // Linear drift per tick, applied tick by tick so the result does
            // not depend on how time was batched.
            for _ in 0..ticks {
                s.satiety = unit(s.satiety - drift::SATIETY);
                s.energy = unit(s.energy - drift::ENERGY);
                s.boredom = unit(s.boredom + drift::BOREDOM);
                s.loneliness = unit(s.loneliness + drift::LONELINESS);
                s.social_capacity =
                    unit(s.social_capacity + drift::SOCIAL_RECOVERY * (1.0 - s.social_capacity));
                s.stress = unit(s.stress - drift::STRESS_DECAY * s.stress);
                s.emotion.fear = unit(s.emotion.fear - drift::FEAR_DECAY * s.emotion.fear);
            }
        }
        WorldEvent::Attacked { severity } => {
            let sev = unit(severity);
            // A Safety-disposed character is frightened more by the same blow.
            let sensitivity = 0.5 + personality.weight(Motivation::Safety);
            s.health = unit(s.health - 0.5 * sev);
            s.emotion.fear = unit(s.emotion.fear + sev * sensitivity);
            s.stress = unit(s.stress + 0.5 * sev * sensitivity);
            s.emotion.arousal = unit(s.emotion.arousal + sev);
            s.emotion.valence = (s.emotion.valence - sev).clamp(-1.0, 1.0);
            s.emotional_battery = unit(s.emotional_battery - 0.2 * sev);
        }
        WorldEvent::Ate { amount } => {
            s.satiety = unit(s.satiety + unit(amount));
        }
        WorldEvent::Rested { amount } => {
            let a = unit(amount);
            s.energy = unit(s.energy + a);
            s.emotional_battery = unit(s.emotional_battery + 0.3 * a);
            s.health = unit(s.health + 0.1 * a);
        }
        WorldEvent::Sheltered { amount } => {
            s.comfort = unit(s.comfort + unit(amount));
        }
        WorldEvent::Socialised { amount } => {
            let a = unit(amount);
            s.loneliness = unit(s.loneliness - a);
            // Interaction spends social capacity; how much depends on how much
            // the character wants contact in the first place.
            let cost = 1.0 - personality.weight(Motivation::Connection);
            s.social_capacity = unit(s.social_capacity - a * cost);
        }
        WorldEvent::Praised { amount } => {
            let a = unit(amount) * (0.5 + personality.weight(Motivation::Recognition));
            s.emotion.valence = (s.emotion.valence + 0.5 * a).clamp(-1.0, 1.0);
            s.emotional_battery = unit(s.emotional_battery + 0.2 * a);
        }
        WorldEvent::Insulted { amount } => {
            let a = unit(amount) * (0.5 + personality.weight(Motivation::Recognition));
            s.emotion.valence = (s.emotion.valence - 0.5 * a).clamp(-1.0, 1.0);
            s.stress = unit(s.stress + 0.3 * a);
            s.emotional_battery = unit(s.emotional_battery - 0.3 * a);
        }
        WorldEvent::Enjoyed { amount } => {
            let a = unit(amount);
            s.boredom = unit(s.boredom - a);
            s.stress = unit(s.stress - 0.3 * a);
            s.emotion.valence = (s.emotion.valence + 0.3 * a).clamp(-1.0, 1.0);
        }
    }
    s.clamped()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_attack_frightens_cautious_character_more() {
        let s = CharacterState::default();
        let cautious = Personality::neutral().with(Motivation::Safety, 1.0);
        let bold = Personality::neutral().with(Motivation::Safety, 0.0);
        let ev = WorldEvent::Attacked { severity: 0.4 };
        let a = appraise(&s, &cautious, &ev);
        let b = appraise(&s, &bold, &ev);
        assert!(a.emotion.fear > b.emotion.fear);
        assert_eq!(a.health, b.health);
    }

    #[test]
    fn time_passing_is_batch_independent() {
        let s = CharacterState::default();
        let p = Personality::neutral();
        let once = appraise(&s, &p, &WorldEvent::TimePassed { ticks: 100 });
        let mut twice = appraise(&s, &p, &WorldEvent::TimePassed { ticks: 40 });
        twice = appraise(&twice, &p, &WorldEvent::TimePassed { ticks: 60 });
        assert_eq!(once, twice);
        assert!(once.satiety < 1.0 && once.boredom > 0.0);
    }

    #[test]
    fn eating_restores_satiety_and_stays_bounded() {
        let s = CharacterState {
            satiety: 0.2,
            ..CharacterState::default()
        };
        let s = appraise(
            &s,
            &Personality::neutral(),
            &WorldEvent::Ate { amount: 5.0 },
        );
        assert_eq!(s.satiety, 1.0);
    }

    #[test]
    fn every_event_keeps_state_in_range() {
        let p = Personality::neutral();
        let events = [
            WorldEvent::TimePassed { ticks: 10_000 },
            WorldEvent::Attacked { severity: 1.0 },
            WorldEvent::Ate { amount: 1.0 },
            WorldEvent::Rested { amount: 1.0 },
            WorldEvent::Sheltered { amount: 1.0 },
            WorldEvent::Socialised { amount: 1.0 },
            WorldEvent::Praised { amount: 1.0 },
            WorldEvent::Insulted { amount: 1.0 },
            WorldEvent::Enjoyed { amount: 1.0 },
        ];
        let mut s = CharacterState::default();
        for _ in 0..3 {
            for e in &events {
                s = appraise(&s, &p, e);
                assert_eq!(s, s.clamped(), "{e:?}");
            }
        }
    }
}
