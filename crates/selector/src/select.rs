// SPDX-License-Identifier: AGPL-3.0-only

//! Selecting exactly one subroutine per request, through Soar's decision
//! procedure (`soar-decide`).
//!
//! 1. **Applicability (symbolic).** Every registered routine is checked
//!    against the request: input kind, expected output, precondition and
//!    budget. Only applicable routines get an acceptable preference, so
//!    nothing else can ever be selected. A learned model cannot change
//!    this.
//! 2. **Constraints (symbolic).** The request's `require`, `prohibit` and
//!    `prefer` become Soar require, prohibit and better preferences.
//! 3. **Preference.** Either
//!    - [`DeterministicSelector`]: cheaper beats dearer (Soar better), and
//!      equal cost is indifferent. Ties go to the lowest stable id
//!      (`first` policy over id-ordered candidates). No random numbers.
//!    - [`UtilitySelector`]: a [`UtilityModel`] gives each applicable
//!      routine a numeric-indifferent value. The highest wins (ε-greedy,
//!      ε = 0), and exact ties are broken with Soar's RNG from a fixed
//!      seed, so the result is still reproducible. This is the slot a Burn
//!      model fills (issue #4 phase 3; literature review §5.2).
//! 4. **Outcome.** One routine, or a typed error:
//!    - no candidates → [`SelectionError::NoApplicableRoutine`];
//!    - tie, conflict or constraint failure →
//!      [`SelectionError::AmbiguousSelection`];
//!    - a malformed request → [`SelectionError::InvalidInput`].

use alloc::vec::Vec;

use soar_decide::{
    decide_context_slot, Decision, Exploration, ImpasseType, ParameterName, Policy, Preference,
    PreferenceType as P, Slot, SoarRng,
};

use crate::registry::{Applicability, ReasoningRequest, Registry};
use crate::task::ReasoningTask;

/// Why no routine was selected (issue #4's `SelectionError`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SelectionError {
    /// No registered routine is applicable, or every applicable one was
    /// prohibited.
    NoApplicableRoutine,
    /// The request is malformed: it requires a routine that is not
    /// applicable, or the utility model returned a non-finite value.
    InvalidInput,
    /// The preferences do not single out one routine (Soar tie, conflict
    /// or constraint-failure impasse).
    AmbiguousSelection,
}

/// Issue #4's selector interface. Implementations return a routine
/// identifier; they never run the routine.
pub trait SubroutineSelector {
    fn select(&self, request: &ReasoningRequest) -> Result<ReasoningTask, SelectionError> {
        self.select_traced(request).result
    }

    /// Select, and record everything needed to inspect and replay the
    /// decision.
    fn select_traced(&self, request: &ReasoningRequest) -> SelectionTrace;
}

/// A learned or hand-written utility for an applicable routine. This is
/// the replaceable "neural selection" interface. It only ever sees routines
/// that are already applicable.
pub trait UtilityModel {
    fn utility(&self, task: ReasoningTask, request: &ReasoningRequest) -> f64;
}

/// Everything about one selection.
#[derive(Clone, Debug, PartialEq)]
pub struct SelectionTrace {
    pub request: ReasoningRequest,
    /// Every registered routine and why it was or was not a candidate.
    pub considered: Vec<(ReasoningTask, Applicability)>,
    /// Utility per applicable routine (empty for the deterministic
    /// selector).
    pub utilities: Vec<(ReasoningTask, f64)>,
    /// Seed of the tie-breaking RNG.
    pub seed: u32,
    /// The Soar impasse behind an ambiguity, if any.
    pub impasse: Option<ImpasseType>,
    pub result: Result<ReasoningTask, SelectionError>,
}

/// Cheapest applicable routine; ties go to the lowest id. Draws no random
/// numbers.
#[derive(Clone, Debug)]
pub struct DeterministicSelector {
    pub registry: Registry,
}

/// Highest-utility applicable routine under a [`UtilityModel`]; exact ties
/// are broken by Soar's RNG seeded with `seed`.
#[derive(Clone, Debug)]
pub struct UtilitySelector<M> {
    pub registry: Registry,
    pub model: M,
    pub seed: u32,
}

impl SubroutineSelector for DeterministicSelector {
    fn select_traced(&self, request: &ReasoningRequest) -> SelectionTrace {
        run::<NoModel>(&self.registry, request, None, 0)
    }
}

impl<M: UtilityModel> SubroutineSelector for UtilitySelector<M> {
    fn select_traced(&self, request: &ReasoningRequest) -> SelectionTrace {
        run(&self.registry, request, Some(&self.model), self.seed)
    }
}

/// Uninhabited: the deterministic selector has no model.
enum NoModel {}

impl UtilityModel for NoModel {
    fn utility(&self, _: ReasoningTask, _: &ReasoningRequest) -> f64 {
        match *self {}
    }
}

/// Re-run a recorded selection and compare.
pub fn replay<S: SubroutineSelector>(selector: &S, trace: &SelectionTrace) -> bool {
    selector.select_traced(&trace.request) == *trace
}

fn run<M: UtilityModel>(
    registry: &Registry,
    req: &ReasoningRequest,
    model: Option<&M>,
    seed: u32,
) -> SelectionTrace {
    let considered: Vec<(ReasoningTask, Applicability)> = registry
        .specs()
        .iter()
        .map(|s| (s.task, s.applicability(req)))
        .collect();
    let applicable: Vec<ReasoningTask> = considered
        .iter()
        .filter(|(_, a)| *a == Applicability::Applicable)
        .map(|(t, _)| *t)
        .collect();
    let mut trace = SelectionTrace {
        request: req.clone(),
        considered,
        utilities: Vec::new(),
        seed,
        impasse: None,
        result: Err(SelectionError::NoApplicableRoutine),
    };

    if let Some(r) = req.require {
        if !applicable.contains(&r) {
            trace.result = Err(SelectionError::InvalidInput);
            return trace;
        }
    }

    let mut slot = Slot::new();
    // Descending id, so Soar's candidate list comes out in ascending id
    // order and `first` means "lowest id".
    for t in applicable.iter().rev() {
        slot.push(Preference::unary(P::Acceptable, *t));
    }
    if let Some(r) = req.require {
        slot.push(Preference::unary(P::Require, r));
    }
    for t in &req.prohibit {
        slot.push(Preference::unary(P::Prohibit, *t));
    }
    for (a, b) in &req.prefer {
        slot.push(Preference::binary(P::Better, *a, *b));
    }

    let mut exploration = Exploration::new(Policy::First);
    match model {
        None => {
            // Cost rule, except for pairs the request ranks itself.
            let ranked = |a: ReasoningTask, b: ReasoningTask| {
                req.prefer
                    .iter()
                    .any(|&(x, y)| (x, y) == (a, b) || (x, y) == (b, a))
            };
            for (i, &a) in applicable.iter().enumerate() {
                for &b in &applicable[i + 1..] {
                    if ranked(a, b) {
                        continue;
                    }
                    let (ca, cb) = (cost(registry, a, req), cost(registry, b, req));
                    if ca < cb {
                        slot.push(Preference::binary(P::Better, a, b));
                    } else if cb < ca {
                        slot.push(Preference::binary(P::Better, b, a));
                    } else {
                        slot.push(Preference::binary(P::BinaryIndifferent, a, b));
                    }
                }
            }
        }
        Some(m) => {
            for &t in &applicable {
                let u = m.utility(t, req);
                if !u.is_finite() {
                    trace.result = Err(SelectionError::InvalidInput);
                    return trace;
                }
                trace.utilities.push((t, u));
                slot.push(Preference::numeric(t, u));
            }
            exploration.policy = Policy::EpsilonGreedy;
            exploration
                .set_parameter(ParameterName::Epsilon, 0.0)
                .expect("0 is a valid epsilon");
        }
    }

    let mut rng = SoarRng::new(seed);
    trace.result = match decide_context_slot(&slot, false, &exploration, &mut rng, &mut None) {
        Decision::Selected(t) => Ok(t),
        Decision::Impasse(imp) => {
            trace.impasse = Some(imp.kind);
            Err(match imp.kind {
                ImpasseType::NoChange => SelectionError::NoApplicableRoutine,
                _ => SelectionError::AmbiguousSelection,
            })
        }
    };
    trace
}

fn cost(registry: &Registry, t: ReasoningTask, req: &ReasoningRequest) -> u32 {
    registry
        .get(t)
        .map_or(u32::MAX, |s| s.cost.estimate(req.size))
}

#[cfg(test)]
#[path = "select_test.rs"]
mod select_test;
