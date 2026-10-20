// SPDX-License-Identifier: AGPL-3.0-only

//! Differential test: this crate against the real `soar` binary.
//!
//! **Methodology.** `examples/gen_differential.rs` generates random
//! scenarios with a fixed-seed PRNG. Each one has:
//! - an `srand` seed;
//! - an exploration policy (boltzmann, epsilon-greedy, first, last,
//!   softmax), ε ∈ {0, 0.1, 0.3, 0.5, 1}, temperature ∈ {0.25, 0.5, 1, 25},
//!   and sum or avg numeric-indifferent mode;
//! - one production proposing 1–5 operators with random preferences of
//!   every type the decision procedure reads.
//!
//! Each scenario runs in Soar (commit 3fc337a, kernel 9.6.5, built locally
//! with g++ 13.3, SVS off). For every scenario the fixture records:
//! - each preference list in Soar's own order, read from the `preferences`
//!   command;
//! - the candidates and selection probabilities from that command, which
//!   runs preference semantics in read-only mode;
//! - the decision Soar made: the operator on S1, or the impasse type,
//!   attribute and items on S2.
//!
//! This test rebuilds each slot in the same order, then requires:
//! 1. read-only semantics to return the same candidates in the same order,
//!    with probabilities that print identically at Soar's two decimals;
//! 2. the decision, made with a `SoarRng` seeded like Soar's, to select the
//!    same operator, or to raise the same impasse over the same items
//!    (compared as a set, because `print` order is not the candidate order).
//!
//! **Results (2026-10-20, Soar commit 3fc337a, x86_64 Linux; the same
//! numbers on wasm32-wasip1 under Node 22 WASI).** 3000 / 3000 scenarios
//! identical:
//! - 1961 selections, 613 of them random choices among several candidates;
//! - 682 ties, 51 conflicts, 39 constraint failures, 267 state no-changes;
//! - 1961 candidate and probability lists.
//!
//! The comparison was confirmed able to fail. Four deliberate mutations
//! each failed it (see `docs/DEVIATIONS.md`). The two that did not are
//! measure-zero boundaries, covered by unit tests instead.

use soar_decide::*;

#[derive(Default)]
struct Scenario {
    id: usize,
    seed: u32,
    policy: String,
    epsilon: f64,
    temperature: f64,
    avg: bool,
    slot: Slot<u32>,
    candidates: Option<Vec<(u32, String)>>,
}

fn kind(code: u8) -> PreferenceType {
    PreferenceType::ALL[code as usize]
}

fn exploration(s: &Scenario) -> Exploration {
    let policy = match s.policy.as_str() {
        "boltzmann" => Policy::Boltzmann,
        "epsilon-greedy" => Policy::EpsilonGreedy,
        "first" => Policy::First,
        "last" => Policy::Last,
        "softmax" => Policy::Softmax,
        p => panic!("policy {p}"),
    };
    let mut e = Exploration::new(policy);
    e.set_parameter(ParameterName::Epsilon, s.epsilon).unwrap();
    e.set_parameter(ParameterName::Temperature, s.temperature)
        .unwrap();
    e.numeric_indifferent_mode = if s.avg {
        NumericIndifferentMode::Avg
    } else {
        NumericIndifferentMode::Sum
    };
    e
}

#[derive(Default)]
struct Tally {
    scenarios: usize,
    selected: usize,
    random_choices: usize,
    ties: usize,
    conflicts: usize,
    constraint_failures: usize,
    no_changes: usize,
    probability_lists: usize,
}

fn check(s: Scenario, outcome: &[&str], tally: &mut Tally) {
    let id = s.id;
    let e = exploration(&s);

    // 1. Read-only semantics and selection probabilities.
    let ro = run_preference_semantics_read_only(&s.slot);
    match &s.candidates {
        Some(cands) => {
            assert_eq!(ro.impasse, ImpasseType::None, "scenario {id}");
            let got: Vec<u32> = ro.candidates.clone();
            let want: Vec<u32> = cands.iter().map(|(v, _)| *v).collect();
            assert_eq!(got, want, "scenario {id}: candidate order");
            for (i, (_, pct)) in cands.iter().enumerate() {
                let p = e.probability_according_to_policy(&s.slot, &ro.candidates, i);
                assert_eq!(
                    format!("{:.2}", p * 100.0),
                    *pct,
                    "scenario {id}: probability of candidate {i}"
                );
            }
            tally.probability_lists += 1;
        }
        None => assert!(
            ro.impasse != ImpasseType::None || ro.candidates.is_empty(),
            "scenario {id}: Soar printed no probabilities but semantics found {ro:?}"
        ),
    }

    // 2. The decision itself.
    let mut rng = SoarRng::new(s.seed);
    let d = decide_context_slot(&s.slot, false, &e, &mut rng, &mut None);
    match outcome {
        ["sel", v] => {
            let v: u32 = v.parse().unwrap();
            assert_eq!(d, Decision::Selected(v), "scenario {id}");
            tally.selected += 1;
            if ro.candidates.len() > 1 {
                tally.random_choices += 1;
            }
        }
        ["imp", kind, attr, items @ ..] => {
            let Decision::Impasse(imp) = d else {
                panic!("scenario {id}: expected {kind} impasse, got {d:?}")
            };
            let want_kind = match *kind {
                "tie" => ImpasseType::Tie,
                "conflict" => ImpasseType::Conflict,
                "constraint-failure" => ImpasseType::ConstraintFailure,
                "no-change" => ImpasseType::NoChange,
                k => panic!("impasse {k}"),
            };
            let want_attr = match *attr {
                "state" => ImpasseAttribute::State,
                "operator" => ImpasseAttribute::Operator,
                a => panic!("attribute {a}"),
            };
            assert_eq!(
                (imp.kind, imp.attribute),
                (want_kind, want_attr),
                "scenario {id}"
            );
            let mut got = imp.items.clone();
            got.sort_unstable();
            let mut want: Vec<u32> = items.iter().map(|x| x.parse().unwrap()).collect();
            want.sort_unstable();
            assert_eq!(got, want, "scenario {id}: impasse items");
            match want_kind {
                ImpasseType::Tie => tally.ties += 1,
                ImpasseType::Conflict => tally.conflicts += 1,
                ImpasseType::ConstraintFailure => tally.constraint_failures += 1,
                _ => tally.no_changes += 1,
            }
        }
        other => panic!("scenario {id}: bad outcome {other:?}"),
    }
    tally.scenarios += 1;
}

#[test]
fn rust_port_matches_soar() {
    let fixture = include_str!("fixtures/differential.txt");
    let mut tally = Tally::default();
    let mut cur: Option<Scenario> = None;
    for line in fixture.lines().filter(|l| !l.starts_with('#')) {
        let t: Vec<&str> = line.split_whitespace().collect();
        match t[0] {
            "S" => {
                cur = Some(Scenario {
                    id: t[1].parse().unwrap(),
                    seed: t[2].parse().unwrap(),
                    policy: t[3].into(),
                    epsilon: t[4].parse().unwrap(),
                    temperature: t[5].parse().unwrap(),
                    avg: t[6] == "avg",
                    ..Default::default()
                })
            }
            "P" => {
                let s = cur.as_mut().unwrap();
                let k = kind(t[1].parse().unwrap());
                let value: u32 = t[2].parse().unwrap();
                let referent = match t[3] {
                    "-" => Referent::None,
                    r if r.starts_with('O') => Referent::Value(r[1..].parse().unwrap()),
                    r => {
                        Referent::Number(f64::from_bits(u64::from_str_radix(&r[1..], 16).unwrap()))
                    }
                };
                s.slot.push(Preference {
                    kind: k,
                    value,
                    referent,
                });
            }
            "C" => {
                let s = cur.as_mut().unwrap();
                if t[1] != "-" {
                    s.candidates = Some(
                        t[1..]
                            .chunks(2)
                            .map(|c| (c[0].parse().unwrap(), c[1].to_string()))
                            .collect(),
                    );
                }
            }
            "D" => check(cur.take().unwrap(), &t[1..], &mut tally),
            x => panic!("unknown record {x}"),
        }
    }
    println!(
        "scenarios {} | selected {} (random choice among >1: {}) | tie {} | conflict {} | constraint-failure {} | no-change {} | probability lists {}",
        tally.scenarios,
        tally.selected,
        tally.random_choices,
        tally.ties,
        tally.conflicts,
        tally.constraint_failures,
        tally.no_changes,
        tally.probability_lists
    );
    // Pin the scenario mix, so a regenerated fixture that silently loses
    // coverage is noticed.
    assert_eq!(tally.scenarios, 3000);
    assert_eq!((tally.selected, tally.random_choices), (1961, 613));
    assert_eq!(
        (
            tally.ties,
            tally.conflicts,
            tally.constraint_failures,
            tally.no_changes
        ),
        (682, 51, 39, 267)
    );
    assert_eq!(tally.probability_lists, 1961);
}
