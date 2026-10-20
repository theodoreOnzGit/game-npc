//! One full decision step, recorded so it can be inspected and replayed.
//!
//! [`decide`] runs model → goals → planner → validation and records every
//! attempt. The planner walks motivations from strongest to weakest and takes
//! the first whose action validates, so a strong but infeasible motivation
//! (hungry, no food) gives way to a feasible one while staying recorded.
//!
//! **Replay conditions.** [`replay`] re-runs a recorded step and requires a
//! bit-identical result. That holds when the same model parameters are used
//! and the platform implements IEEE-754 binary32 `+ - *` with round-to-nearest
//! and without fused multiply-add, which is what Rust guarantees for `f32`
//! arithmetic on native targets and wasm32. The checksum is the lockstep
//! desync check: two peers that disagree on any input, output or choice
//! disagree on the checksum.

use crate::action::{validate, Action, ActionRejection, ValidatedAction};
use crate::goal::{generate_goal, plan, Goal};
use crate::model::{ModelInput, MotivationModel};
use crate::motivation::{Motivation, MOTIVATION_COUNT};
use crate::vector::MotivationVector;
use crate::world::WorldState;

/// One motivation the planner tried.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Attempt {
    pub motivation: Motivation,
    pub strength: f32,
    pub goal: Goal,
    pub action: Action,
    pub outcome: Result<(), ActionRejection>,
}

/// Full record of one decision.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DecisionTrace {
    pub input: ModelInput,
    pub world: WorldState,
    pub vector: MotivationVector,
    attempts: [Option<Attempt>; MOTIVATION_COUNT],
    pub chosen: Option<ValidatedAction>,
    pub checksum: u64,
}

impl DecisionTrace {
    /// The attempts made, strongest motivation first; the last one is the
    /// chosen action if any validated.
    pub fn attempts(&self) -> impl Iterator<Item = &Attempt> {
        self.attempts.iter().flatten()
    }
}

/// Why a replay did not reproduce the recorded decision.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReplayError {
    /// A motivation value differs at the bit level.
    Vector {
        motivation: Motivation,
        recorded_bits: u32,
        replayed_bits: u32,
    },
    /// A different action was chosen.
    Choice,
    /// Everything above matched but the checksum did not (the trace was
    /// edited or the checksum algorithm changed).
    Checksum { recorded: u64, replayed: u64 },
}

/// Run one decision step.
pub fn decide<M: MotivationModel>(
    model: &M,
    input: &ModelInput,
    world: &WorldState,
) -> DecisionTrace {
    let vector = model.predict(input);
    let mut attempts = [None; MOTIVATION_COUNT];
    let mut chosen = None;
    for (slot, m) in attempts.iter_mut().zip(vector.ranked()) {
        let goal = generate_goal(m);
        let action = plan(goal);
        let result = validate(action, world);
        *slot = Some(Attempt {
            motivation: m,
            strength: vector.get(m),
            goal,
            action,
            outcome: result.map(|_| ()),
        });
        if let Ok(va) = result {
            chosen = Some(va);
            break;
        }
    }
    let checksum = checksum(input, world, &vector, chosen);
    DecisionTrace {
        input: *input,
        world: *world,
        vector,
        attempts,
        chosen,
        checksum,
    }
}

/// Re-run a recorded decision and require a bit-identical result.
pub fn replay<M: MotivationModel>(model: &M, trace: &DecisionTrace) -> Result<(), ReplayError> {
    let again = decide(model, &trace.input, &trace.world);
    for m in Motivation::ALL {
        let (r, p) = (trace.vector.get(m).to_bits(), again.vector.get(m).to_bits());
        if r != p {
            return Err(ReplayError::Vector {
                motivation: m,
                recorded_bits: r,
                replayed_bits: p,
            });
        }
    }
    if trace.chosen != again.chosen {
        return Err(ReplayError::Choice);
    }
    if trace.checksum != again.checksum {
        return Err(ReplayError::Checksum {
            recorded: trace.checksum,
            replayed: again.checksum,
        });
    }
    Ok(())
}

/// 64-bit FNV-1a, fed in a fixed order with explicit little-endian bytes.
struct Fnv(u64);

impl Fnv {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;

    fn bytes(&mut self, bytes: &[u8]) {
        for b in bytes {
            self.0 ^= u64::from(*b);
            self.0 = self.0.wrapping_mul(Self::PRIME);
        }
    }
    fn f32(&mut self, x: f32) {
        self.bytes(&x.to_bits().to_le_bytes());
    }
    fn u32(&mut self, x: u32) {
        self.bytes(&x.to_le_bytes());
    }
    fn bool(&mut self, x: bool) {
        self.bytes(&[u8::from(x)]);
    }
}

fn checksum(
    input: &ModelInput,
    world: &WorldState,
    vector: &MotivationVector,
    chosen: Option<ValidatedAction>,
) -> u64 {
    let mut h = Fnv(Fnv::OFFSET);
    for x in input.previous.as_array() {
        h.f32(*x);
    }
    let s = &input.state;
    for x in [
        s.health,
        s.energy,
        s.satiety,
        s.comfort,
        s.emotional_battery,
        s.social_capacity,
        s.stress,
        s.boredom,
        s.loneliness,
        s.emotion.valence,
        s.emotion.arousal,
        s.emotion.fear,
    ] {
        h.f32(x);
    }
    for x in input.personality.as_array() {
        h.f32(*x);
    }
    let p = &input.perception;
    for x in [
        p.threat,
        p.others_present,
        p.group_present,
        p.companion_present,
        p.dependant_in_need,
        p.unexplored,
        p.obligation_pending,
        p.injustice,
        p.coercion,
        p.disorder,
        p.resources_available,
    ] {
        h.f32(x);
    }
    for x in [
        world.food,
        world.threat,
        world.others_nearby,
        world.unexplored_sites,
        world.resources,
        world.inventory,
        world.pending_tasks,
        world.disorder,
    ] {
        h.u32(x);
    }
    for x in [
        world.bed_available,
        world.shelter_available,
        world.escape_route,
        world.group_nearby,
        world.companion_nearby,
        world.dependant_in_need,
        world.wrongdoer_nearby,
        world.coerced,
    ] {
        h.bool(x);
    }
    for x in vector.as_array() {
        h.f32(*x);
    }
    match chosen {
        Some(va) => h.bytes(&[1, va.action().code()]),
        None => h.bytes(&[0]),
    }
    h.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::DeterministicModel;
    use crate::personality::Personality;
    use crate::state::CharacterState;

    fn hungry_input(world: &WorldState) -> ModelInput {
        ModelInput {
            previous: MotivationVector::zero(),
            state: CharacterState {
                satiety: 0.0,
                ..CharacterState::default()
            },
            personality: Personality::neutral(),
            perception: world.perceive(),
        }
    }

    #[test]
    fn infeasible_top_motivation_falls_through_and_is_recorded() {
        let world = WorldState {
            others_nearby: 1,
            ..WorldState::default()
        };
        let t = decide(
            &DeterministicModel::default(),
            &hungry_input(&world),
            &world,
        );
        let first = t.attempts().next().unwrap();
        assert_eq!(first.motivation, Motivation::Sustenance);
        assert_eq!(first.outcome, Err(ActionRejection::NoFood));
        assert!(t.chosen.is_some());
        assert_ne!(t.chosen.unwrap().action(), Action::Eat);
    }

    #[test]
    fn replay_reproduces_bit_for_bit() {
        let world = WorldState {
            food: 2,
            ..WorldState::default()
        };
        let model = DeterministicModel::default();
        let t = decide(&model, &hungry_input(&world), &world);
        assert_eq!(t.chosen.unwrap().action(), Action::Eat);
        assert_eq!(replay(&model, &t), Ok(()));
    }

    #[test]
    fn replay_detects_a_different_model() {
        let world = WorldState::default();
        let t = decide(
            &DeterministicModel::default(),
            &hungry_input(&world),
            &world,
        );
        let other = DeterministicModel::new(crate::ScoringParams { persistence: 0.9 });
        assert!(matches!(
            replay(&other, &t),
            Err(ReplayError::Vector { .. })
        ));
    }

    #[test]
    fn checksum_detects_tampering() {
        let world = WorldState::default();
        let model = DeterministicModel::default();
        let mut t = decide(&model, &hungry_input(&world), &world);
        t.checksum ^= 1;
        assert!(matches!(
            replay(&model, &t),
            Err(ReplayError::Checksum { .. })
        ));
    }

    #[test]
    fn checksum_differs_when_input_differs() {
        let world = WorldState::default();
        let model = DeterministicModel::default();
        let a = decide(&model, &hungry_input(&world), &world);
        let mut input = hungry_input(&world);
        input.state.satiety = 0.01;
        let b = decide(&model, &input, &world);
        assert_ne!(a.checksum, b.checksum);
    }
}
