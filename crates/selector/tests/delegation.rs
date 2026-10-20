// SPDX-License-Identifier: AGPL-3.0-only

//! Delegated execution end to end: the selector chooses a routine and the
//! host runs it through the typed [`Subroutine`] interface. Two real
//! adapters are used:
//! - GOAP planning via `crates/goapai`;
//! - root finding via PETIR's Brent solver (`petir::roots`, a GSL port).
//!
//! The selector crate itself depends on neither.

use selector::*;

struct GoapPlanner;

struct PlanningInput {
    agent: goapai::Agent,
    max_depth: u16,
}

impl Subroutine for GoapPlanner {
    type Input = PlanningInput;
    type Output = Vec<String>;
    type Error = goapai::GoapError;
    const TASK: ReasoningTask = ReasoningTask::GoalPlanning;

    fn precondition(&self, input: &PlanningInput) -> bool {
        !input.agent.goals().is_empty()
    }

    fn estimate(&self, input: &PlanningInput) -> u32 {
        u32::from(input.max_depth)
    }

    fn run(
        &self,
        input: &PlanningInput,
        _limits: Limits,
    ) -> Result<Vec<String>, goapai::GoapError> {
        let (_, plan) = goapai::try_get_plan(&input.agent, input.max_depth)?;
        Ok(plan
            .steps()
            .iter()
            .map(|a| a.get_name().to_string())
            .collect())
    }
}

struct BrentRoot;

/// Find a root of the polynomial `coeffs[0] + coeffs[1] x + …` in
/// `[lo, hi]`.
struct RootInput {
    coeffs: Vec<f64>,
    lo: f64,
    hi: f64,
}

impl Subroutine for BrentRoot {
    type Input = RootInput;
    type Output = f64;
    type Error = petir::PetirError;
    const TASK: ReasoningTask = ReasoningTask::MathematicalCalculation;

    fn precondition(&self, input: &RootInput) -> bool {
        !input.coeffs.is_empty() && input.lo < input.hi
    }

    fn estimate(&self, _: &RootInput) -> u32 {
        100
    }

    fn run(&self, input: &RootInput, limits: Limits) -> Result<f64, petir::PetirError> {
        let c = input.coeffs.clone();
        let f = move |x: f64| c.iter().rev().fold(0.0, |acc, k| acc * x + k);
        let mut s = petir::roots::BracketingSolver::new(
            petir::roots::BracketingMethod::Brent,
            f,
            input.lo,
            input.hi,
        )?;
        s.solve(1e-12, 0.0, limits.max_work as usize)
    }
}

fn registry() -> Registry {
    let mut r = Registry::new();
    r.register(SubroutineSpec {
        task: ReasoningTask::GoalPlanning,
        accepts: InputKind::PlanningProblem,
        returns: OutputKind::ActionPlan,
        cost: Cost::Linear {
            base: 10,
            per_item: 5,
        },
        precondition: None,
    })
    .unwrap();
    r.register(SubroutineSpec {
        task: ReasoningTask::MathematicalCalculation,
        accepts: InputKind::NumericProblem,
        returns: OutputKind::Number,
        cost: Cost::Constant(100),
        precondition: None,
    })
    .unwrap();
    r
}

fn fire_agent() -> goapai::Agent {
    use goapai::*;
    let mut actions = Actions::new();
    actions.add_action(
        "get_wood",
        2.0,
        false,
        vec![],
        vec![EffectBool {
            key: 1,
            value: true,
            operator: Arithmetic::Set,
        }
        .into()],
    );
    actions.add_action(
        "get_matches",
        1.0,
        false,
        vec![],
        vec![EffectBool {
            key: 2,
            value: true,
            operator: Arithmetic::Set,
        }
        .into()],
    );
    actions.add_action(
        "make_fire",
        1.0,
        false,
        vec![
            ConditionBool {
                key: 1,
                value: true,
                operator: Operator::Equal,
            }
            .into(),
            ConditionBool {
                key: 2,
                value: true,
                operator: Operator::Equal,
            }
            .into(),
        ],
        vec![EffectBool {
            key: 3,
            value: true,
            operator: Arithmetic::Set,
        }
        .into()],
    );
    let goals = Goals::new().with(
        "stay_warm",
        GoalInterface::new(
            vec![ConditionBool {
                key: 3,
                value: true,
                operator: Operator::Equal,
            }
            .into()],
            |_| 1.0,
        ),
    );
    let mut agent = create_agent(goals, actions);
    for k in 1..=3 {
        set_state(&mut agent, k, false);
    }
    agent
}

#[test]
fn planning_request_is_routed_to_goap() {
    let selector = DeterministicSelector {
        registry: registry(),
    };
    let req = ReasoningRequest::new(InputKind::PlanningProblem, 3);
    let task = selector.select(&req).unwrap();
    assert_eq!(task, ReasoningTask::GoalPlanning);

    let input = PlanningInput {
        agent: fire_agent(),
        max_depth: 5,
    };
    let plan = execute(task, &GoapPlanner, &input, Limits { max_work: 10 }).unwrap();
    assert_eq!(plan.last().map(String::as_str), Some("make_fire"));
    assert_eq!(plan.len(), 3);
}

#[test]
fn numeric_request_is_routed_to_petir() {
    let selector = DeterministicSelector {
        registry: registry(),
    };
    let task = selector
        .select(&ReasoningRequest::new(InputKind::NumericProblem, 1))
        .unwrap();
    assert_eq!(task, ReasoningTask::MathematicalCalculation);

    // x^2 - 2 on [0, 2]: sqrt(2).
    let input = RootInput {
        coeffs: vec![-2.0, 0.0, 1.0],
        lo: 0.0,
        hi: 2.0,
    };
    let root = execute(task, &BrentRoot, &input, Limits { max_work: 100 }).unwrap();
    assert!((root - std::f64::consts::SQRT_2).abs() < 1e-12, "{root}");
}

#[test]
fn execution_errors_propagate_typed() {
    let m = ReasoningTask::MathematicalCalculation;
    // Running the wrong subroutine for the selected task is refused.
    let input = RootInput {
        coeffs: vec![-2.0, 0.0, 1.0],
        lo: 0.0,
        hi: 2.0,
    };
    assert!(matches!(
        execute(
            ReasoningTask::GoalPlanning,
            &BrentRoot,
            &input,
            Limits { max_work: 100 }
        ),
        Err(ExecutionError::WrongSubroutine { .. })
    ));
    // The work limit is enforced before running.
    assert_eq!(
        execute(m, &BrentRoot, &input, Limits { max_work: 10 }),
        Err(ExecutionError::LimitExceeded {
            estimate: 100,
            limit: 10
        })
    );
    // PETIR's own error comes through: no sign change on [3, 4].
    let bad = RootInput {
        coeffs: vec![-2.0, 0.0, 1.0],
        lo: 3.0,
        hi: 4.0,
    };
    assert_eq!(
        execute(m, &BrentRoot, &bad, Limits { max_work: 100 }),
        Err(ExecutionError::Subroutine(petir::PetirError::Invalid))
    );
    // A malformed input fails the precondition.
    let empty = RootInput {
        coeffs: vec![],
        lo: 0.0,
        hi: 1.0,
    };
    assert_eq!(
        execute(m, &BrentRoot, &empty, Limits { max_work: 100 }),
        Err(ExecutionError::PreconditionFailed)
    );
}
