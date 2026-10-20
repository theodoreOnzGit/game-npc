//! Representative NPC scenarios run through the full closed loop:
//! appraise → predict → plan → validate → apply → appraise …
//!
//! These are behavioural regression tests for the deterministic baseline,
//! and the reference the future Burn model will be compared against.

use game_npc::{
    appraise, decide, replay, Action, CharacterState, DecisionTrace, DeterministicModel,
    Motivation, MotivationVector, Personality, WorldEvent, WorldState,
};

struct Npc {
    personality: Personality,
    state: CharacterState,
    motivation: MotivationVector,
}

/// Run `steps` decision steps, returning every trace.
fn run(npc: &mut Npc, world: &mut WorldState, steps: usize) -> Vec<DecisionTrace> {
    let model = DeterministicModel::default();
    let mut traces = Vec::new();
    for _ in 0..steps {
        npc.state = appraise(
            &npc.state,
            &npc.personality,
            &WorldEvent::TimePassed { ticks: 10 },
        );
        let input = game_npc::model::ModelInput {
            previous: npc.motivation,
            state: npc.state,
            personality: npc.personality,
            perception: world.perceive(),
        };
        let trace = decide(&model, &input, world);
        npc.motivation = trace.vector;
        if let Some(va) = trace.chosen {
            let (w, ev) = world.apply(&va);
            *world = w;
            npc.state = appraise(&npc.state, &npc.personality, &ev);
        }
        traces.push(trace);
    }
    traces
}

fn chosen(t: &DecisionTrace) -> Option<Action> {
    t.chosen.map(|v| v.action())
}

/// The same attack, two personalities that differ ONLY in Safety
/// disposition (everything else neutral): the cautious one flees, the bold
/// one keeps working on its pending task.
#[test]
fn same_event_different_priorities() {
    let world = WorldState {
        threat: 1,
        escape_route: true,
        pending_tasks: 3,
        ..WorldState::default()
    };
    let mut outcomes = Vec::new();
    for safety in [1.0, 0.1] {
        let p = Personality::neutral().with(Motivation::Safety, safety);
        let mut npc = Npc {
            personality: p,
            state: appraise(
                &CharacterState::default(),
                &p,
                &WorldEvent::Attacked { severity: 0.3 },
            ),
            motivation: MotivationVector::zero(),
        };
        let mut w = world;
        let t = run(&mut npc, &mut w, 1);
        outcomes.push(chosen(&t[0]));
    }
    assert_eq!(outcomes[0], Some(Action::Flee));
    assert_eq!(outcomes[1], Some(Action::Work));
}

/// A hungry character with food eats until it is no longer the top want,
/// then turns to something else; with no food it does something feasible.
#[test]
fn hunger_is_satisfied_then_released() {
    let p = Personality::neutral();
    let mut npc = Npc {
        personality: p,
        state: CharacterState {
            satiety: 0.0,
            ..CharacterState::default()
        },
        motivation: MotivationVector::zero(),
    };
    let mut world = WorldState {
        food: 5,
        unexplored_sites: 2,
        ..WorldState::default()
    };
    let traces = run(&mut npc, &mut world, 6);
    assert_eq!(chosen(&traces[0]), Some(Action::Eat));
    assert!(traces.iter().any(|t| chosen(t) != Some(Action::Eat)));
    assert!(world.food < 5);
    assert!(npc.state.satiety > 0.5);
}

/// Every step of a long run replays bit-for-bit, and the whole run is
/// reproducible end to end.
#[test]
fn long_run_replays_and_is_reproducible() {
    let make = || {
        (
            Npc {
                personality: Personality::neutral()
                    .with(Motivation::Curiosity, 0.8)
                    .with(Motivation::Connection, 0.7),
                state: CharacterState::default(),
                motivation: MotivationVector::zero(),
            },
            WorldState {
                food: 3,
                bed_available: true,
                others_nearby: 2,
                companion_nearby: true,
                unexplored_sites: 4,
                resources: 5,
                pending_tasks: 2,
                disorder: 2,
                ..WorldState::default()
            },
        )
    };
    let (mut a, mut wa) = make();
    let (mut b, mut wb) = make();
    let ta = run(&mut a, &mut wa, 200);
    let tb = run(&mut b, &mut wb, 200);
    let model = DeterministicModel::default();
    for (x, y) in ta.iter().zip(&tb) {
        assert_eq!(x.checksum, y.checksum);
        assert_eq!(replay(&model, x), Ok(()));
    }
    assert_eq!(wa, wb);
}

/// Golden checksum for lockstep: a fixed scenario's final decision checksum,
/// pinned so that any platform, compiler or code change that alters the
/// result bits is caught. If this fails after an *intentional* change to
/// scoring or appraisal, update the constant and say so in the commit.
///
/// Recorded 2026-10-20, x86_64-unknown-linux-gnu, rustc 1.97.0. Reproduced
/// bit-for-bit on wasm32-wasip1 (Node 22 WASI) the same day.
#[test]
fn golden_checksum() {
    let mut npc = Npc {
        personality: Personality::neutral()
            .with(Motivation::Safety, 0.8)
            .with(Motivation::Caregiving, 0.9),
        state: CharacterState::default(),
        motivation: MotivationVector::zero(),
    };
    let mut world = WorldState {
        food: 2,
        bed_available: true,
        shelter_available: true,
        escape_route: true,
        threat: 1,
        others_nearby: 1,
        dependant_in_need: true,
        unexplored_sites: 3,
        pending_tasks: 2,
        wrongdoer_nearby: true,
        ..WorldState::default()
    };
    let traces = run(&mut npc, &mut world, 50);
    let last = traces.last().unwrap().checksum;
    println!("golden checksum = {last:#018x}");
    assert_eq!(last, GOLDEN);
}

const GOLDEN: u64 = 0x3e6f_dae2_ecf6_e9f0;
