// SPDX-License-Identifier: AGPL-3.0-only

//! Differential test: the Rust port against the original Go goapai.
//!
//! **Methodology.** `differential/main.go` generates 3000 random planning
//! scenarios with a fixed-seed PRNG (splitmix64, seed `0x60a9a1`): 1-4 typed
//! state keys across all seven state types, 1-5 actions with random costs,
//! repeatability, conditions and effects, 1-3 goals with distinct priorities
//! (5 % with all priorities zero), and `max_depth` 1-6. It deliberately
//! includes type-mismatched conditions and effects, invalid operators on
//! bool and string, `int8`/`uint8` overflow, `uint64` values near the
//! maximum, and NaN floats. Goal conditions are mostly derived from the
//! actions' effects, so many goals are reachable. 35 % of scenarios run in
//! "tie mode" (all costs 1, actions duplicated under new names), so the plan
//! returned depends on the heap's tie-breaking order. Upstream goapai
//! (commit 2e86c5e, Go 1.24.7, linux/amd64) solves each scenario, and the
//! selected goal, the plan's action names and the bit pattern of its `f32`
//! total cost are recorded in `tests/fixtures/differential.txt`. This test
//! rebuilds every scenario through the Rust API and requires all three to
//! match exactly.
//!
//! Inputs where the port deliberately differs from upstream are excluded by
//! the generator; see `docs/DEVIATIONS.md`.
//!
//! **Results (2026-10-20).** 3000 / 3000 scenarios identical (goal, action
//! sequence and cost bits), natively (x86_64-unknown-linux-gnu) and as
//! WebAssembly (wasm32-wasip1 under Node 22 WASI). Plan lengths in the
//! fixture, root action included: 2303 empty (unreachable, or no goal with
//! positive priority in 139 of them), 379 already at the goal (length 1),
//! 270 / 32 / 7 / 5 / 4 with one to five actions.
//!
//! **Sensitivity check.** The comparison was confirmed able to fail: each
//! of these deliberate mutations made it fail on the scenario shown, and
//! was then reverted:
//! - heap `less` changed from `<` to `<=` (tie order): scenario 678;
//! - successors generated in reverse action order: scenario 57;
//! - heuristic dropped from `total_cost`: scenario 475 (on an earlier
//!   fixture without tie mode).

use goapai::*;

fn f64_bits(s: &str) -> f64 {
    f64::from_bits(u64::from_str_radix(s, 16).unwrap())
}

fn f32_bits(s: &str) -> f32 {
    f32::from_bits(u32::from_str_radix(s, 16).unwrap())
}

fn value(t: &str, v: &str) -> Value {
    match t {
        "i8" => Value::Int8(v.parse().unwrap()),
        "int" => Value::Int(v.parse().unwrap()),
        "u8" => Value::Uint8(v.parse().unwrap()),
        "u64" => Value::Uint64(v.parse().unwrap()),
        "f64" => Value::Float64(f64_bits(v)),
        "bool" => Value::Bool(v == "1"),
        "str" => Value::String(v.into()),
        _ => panic!("unknown type {t}"),
    }
}

const OPERATORS: [Operator; 6] = [
    Operator::Equal,
    Operator::NotEqual,
    Operator::LowerOrEqual,
    Operator::Lower,
    Operator::UpperOrEqual,
    Operator::Upper,
];

const ARITHMETIC: [Arithmetic; 5] = [
    Arithmetic::Set,
    Arithmetic::Add,
    Arithmetic::Subtract,
    Arithmetic::Multiply,
    Arithmetic::Divide,
];

fn condition(key: StateKey, v: Value, op: Operator) -> ConditionInterface {
    match v {
        Value::Int8(x) => Condition::new(key, op, x).into(),
        Value::Int(x) => Condition::new(key, op, x).into(),
        Value::Uint8(x) => Condition::new(key, op, x).into(),
        Value::Uint64(x) => Condition::new(key, op, x).into(),
        Value::Float64(x) => Condition::new(key, op, x).into(),
        Value::Bool(value) => ConditionBool {
            key,
            value,
            operator: op,
        }
        .into(),
        Value::String(value) => ConditionString {
            key,
            value,
            operator: op,
        }
        .into(),
    }
}

fn effect(key: StateKey, v: Value, op: Arithmetic) -> EffectInterface {
    match v {
        Value::Int8(x) => Effect::new(key, op, x).into(),
        Value::Int(x) => Effect::new(key, op, x).into(),
        Value::Uint8(x) => Effect::new(key, op, x).into(),
        Value::Uint64(x) => Effect::new(key, op, x).into(),
        Value::Float64(x) => Effect::new(key, op, x).into(),
        Value::Bool(value) => EffectBool {
            key,
            value,
            operator: op,
        }
        .into(),
        Value::String(value) => EffectString {
            key,
            value,
            operator: op,
        }
        .into(),
    }
}

struct PendingAction {
    name: String,
    cost: f32,
    repeatable: bool,
    conditions: Conditions,
    effects: Effects,
}

struct PendingGoal {
    name: String,
    priority: f32,
    conditions: Conditions,
}

enum Owner {
    Action,
    Goal,
}

struct Scenario {
    id: usize,
    max_depth: u16,
    states: Vec<(StateKey, Value)>,
    actions: Vec<PendingAction>,
    goals: Vec<PendingGoal>,
    owner: Owner,
}

impl Scenario {
    fn run(self) -> (String, u32, Vec<String>) {
        let mut actions = Actions::new();
        for a in self.actions {
            actions.add_action(&a.name, a.cost, a.repeatable, a.conditions, a.effects);
        }
        let mut goals = Goals::new();
        for g in self.goals {
            let p = g.priority;
            goals.insert(&g.name, GoalInterface::new(g.conditions, move |_| p));
        }
        let mut agent = create_agent(goals, actions);
        for (k, v) in self.states {
            agent_set(&mut agent, k, v);
        }
        let (goal, plan) = get_plan(&agent, self.max_depth);
        let names = plan
            .iter()
            .map(|a| {
                if a.get_name().is_empty() {
                    "_".into()
                } else {
                    a.get_name().into()
                }
            })
            .collect();
        (goal, plan.get_total_cost().to_bits(), names)
    }
}

fn agent_set(agent: &mut Agent, key: StateKey, v: Value) {
    match v {
        Value::Int8(x) => set_state(agent, key, x),
        Value::Int(x) => set_state(agent, key, x),
        Value::Uint8(x) => set_state(agent, key, x),
        Value::Uint64(x) => set_state(agent, key, x),
        Value::Float64(x) => set_state(agent, key, x),
        Value::Bool(x) => set_state(agent, key, x),
        Value::String(x) => set_state(agent, key, x),
    }
}

#[test]
fn rust_port_matches_go_goapai() {
    let fixture = include_str!("fixtures/differential.txt");
    let mut current: Option<Scenario> = None;
    let (mut checked, mut nontrivial) = (0, 0);
    for line in fixture.lines().filter(|l| !l.starts_with('#')) {
        let t: Vec<&str> = line.split(' ').collect();
        match t[0] {
            "S" => {
                current = Some(Scenario {
                    id: t[1].parse().unwrap(),
                    max_depth: t[2].parse().unwrap(),
                    states: vec![],
                    actions: vec![],
                    goals: vec![],
                    owner: Owner::Action,
                })
            }
            "T" => {
                let s = current.as_mut().unwrap();
                s.states.push((t[1].parse().unwrap(), value(t[2], t[3])));
            }
            "A" => {
                let s = current.as_mut().unwrap();
                s.actions.push(PendingAction {
                    name: t[1].into(),
                    cost: f32_bits(t[2]),
                    repeatable: t[3] == "1",
                    conditions: vec![],
                    effects: vec![],
                });
                s.owner = Owner::Action;
            }
            "G" => {
                let s = current.as_mut().unwrap();
                s.goals.push(PendingGoal {
                    name: t[1].into(),
                    priority: f32_bits(t[2]),
                    conditions: vec![],
                });
                s.owner = Owner::Goal;
            }
            "C" => {
                let s = current.as_mut().unwrap();
                let op = OPERATORS[t[3].parse::<usize>().unwrap()];
                let c = condition(t[1].parse().unwrap(), value(t[2], t[4]), op);
                match s.owner {
                    Owner::Action => s.actions.last_mut().unwrap().conditions.push(c),
                    Owner::Goal => s.goals.last_mut().unwrap().conditions.push(c),
                }
            }
            "E" => {
                let s = current.as_mut().unwrap();
                let op = ARITHMETIC[t[3].parse::<usize>().unwrap()];
                let e = effect(t[1].parse().unwrap(), value(t[2], t[4]), op);
                s.actions.last_mut().unwrap().effects.push(e);
            }
            "R" => {
                let s = current.take().unwrap();
                let id = s.id;
                let want_goal = if t[1] == "-" { "" } else { t[1] };
                let want_cost = u32::from_str_radix(t[2], 16).unwrap();
                let want_len: usize = t[3].parse().unwrap();
                let want_names: Vec<String> =
                    t[4..4 + want_len].iter().map(|s| s.to_string()).collect();
                let (goal, cost, names) = s.run();
                assert_eq!(
                    (goal.as_str(), &names, cost),
                    (want_goal, &want_names, want_cost),
                    "scenario {id}"
                );
                checked += 1;
                if want_len > 1 {
                    nontrivial += 1;
                }
            }
            other => panic!("unknown record {other}"),
        }
    }
    assert_eq!(checked, 3000);
    assert_eq!(nontrivial, 318);
}
