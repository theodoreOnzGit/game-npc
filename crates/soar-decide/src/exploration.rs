// SPDX-License-Identifier: AGPL-3.0-only
// Translated from Soar
// `Core/SoarKernel/src/reinforcement_learning/exploration.{h,cpp}`
// (https://github.com/SoarGroup/Soar, commit 3fc337a, BSD, Copyright
// 1995-2012 Carnegie Mellon University, The Regents of the University of
// Michigan, USC/ISI). Modified: see docs/DEVIATIONS.md.

//! Exploration policies: how Soar picks among mutually indifferent
//! candidates, using their numeric-indifferent values.

use crate::preference::{PreferenceType as T, Slot};
use crate::rng::SoarRng;

/// Selection policy (`USER_SELECT_*`, Soar's numbering).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Policy {
    /// Probability ∝ exp(value / temperature).
    Boltzmann = 1,
    /// Probability ε: uniform random. Otherwise the highest value, with
    /// ties broken at random.
    EpsilonGreedy = 2,
    /// First candidate.
    First = 3,
    /// Last candidate.
    Last = 4,
    /// Uniform random. Soar's kernel supports it, but its CLI option is
    /// commented out.
    RandomUniform = 5,
    /// Probability ∝ value, over positive values. Uniform if none is
    /// positive. Soar's default.
    Softmax = 6,
}

/// How a parameter decays each decision when auto-update is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ReductionPolicy {
    /// value ← value × rate, rate ∈ [0, 1].
    Exponential,
    /// value ← max(value − rate, 0), rate ≥ 0.
    Linear,
}

/// One exploration parameter and its decay settings
/// (`exploration_parameter`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Parameter {
    value: f64,
    pub reduction_policy: ReductionPolicy,
    exponential_rate: f64,
    linear_rate: f64,
}

impl Parameter {
    fn new(value: f64) -> Self {
        Self {
            value,
            reduction_policy: ReductionPolicy::Exponential,
            exponential_rate: 1.0,
            linear_rate: 0.0,
        }
    }

    pub fn value(&self) -> f64 {
        self.value
    }

    pub fn rate(&self, policy: ReductionPolicy) -> f64 {
        match policy {
            ReductionPolicy::Exponential => self.exponential_rate,
            ReductionPolicy::Linear => self.linear_rate,
        }
    }
}

/// How several numeric-indifferent values for one candidate combine.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum NumericIndifferentMode {
    #[default]
    Sum,
    Avg,
}

/// A rejected setting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExplorationError {
    /// ε must be in [0, 1].
    InvalidEpsilon,
    /// Temperature must be > 0.
    InvalidTemperature,
    /// Exponential rates must be in [0, 1]; linear rates ≥ 0.
    InvalidReductionRate,
}

/// Which parameter a setting refers to (`EXPLORATION_PARAM_*`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ParameterName {
    Epsilon,
    Temperature,
}

/// Exploration settings (Soar's per-agent exploration state). Defaults match
/// Soar: softmax, ε = 0.1, temperature 25, sum mode, auto-update off.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Exploration {
    pub policy: Policy,
    pub numeric_indifferent_mode: NumericIndifferentMode,
    /// Apply parameter decay in [`Exploration::update_parameters`].
    pub auto_update: bool,
    epsilon: Parameter,
    temperature: Parameter,
}

impl Default for Exploration {
    fn default() -> Self {
        Self {
            policy: Policy::Softmax,
            numeric_indifferent_mode: NumericIndifferentMode::Sum,
            auto_update: false,
            epsilon: Parameter::new(0.1),
            temperature: Parameter::new(25.0),
        }
    }
}

fn valid(name: ParameterName, value: f64) -> bool {
    match name {
        ParameterName::Epsilon => (0.0..=1.0).contains(&value),
        ParameterName::Temperature => value > 0.0,
    }
}

impl Exploration {
    pub fn new(policy: Policy) -> Self {
        Self {
            policy,
            ..Self::default()
        }
    }

    pub fn parameter(&self, name: ParameterName) -> &Parameter {
        match name {
            ParameterName::Epsilon => &self.epsilon,
            ParameterName::Temperature => &self.temperature,
        }
    }

    fn parameter_mut(&mut self, name: ParameterName) -> &mut Parameter {
        match name {
            ParameterName::Epsilon => &mut self.epsilon,
            ParameterName::Temperature => &mut self.temperature,
        }
    }

    pub fn epsilon(&self) -> f64 {
        self.epsilon.value
    }

    pub fn temperature(&self) -> f64 {
        self.temperature.value
    }

    /// Set a parameter, validated as Soar's CLI does (ε ∈ [0, 1],
    /// temperature > 0).
    pub fn set_parameter(
        &mut self,
        name: ParameterName,
        value: f64,
    ) -> Result<(), ExplorationError> {
        if !valid(name, value) {
            return Err(match name {
                ParameterName::Epsilon => ExplorationError::InvalidEpsilon,
                ParameterName::Temperature => ExplorationError::InvalidTemperature,
            });
        }
        self.parameter_mut(name).value = value;
        Ok(())
    }

    /// Set a decay rate (`exploration_set_reduction_rate`).
    pub fn set_reduction_rate(
        &mut self,
        name: ParameterName,
        policy: ReductionPolicy,
        rate: f64,
    ) -> Result<(), ExplorationError> {
        let ok = match policy {
            ReductionPolicy::Exponential => (0.0..=1.0).contains(&rate),
            ReductionPolicy::Linear => rate >= 0.0,
        };
        if !ok {
            return Err(ExplorationError::InvalidReductionRate);
        }
        let p = self.parameter_mut(name);
        match policy {
            ReductionPolicy::Exponential => p.exponential_rate = rate,
            ReductionPolicy::Linear => p.linear_rate = rate,
        }
        Ok(())
    }

    /// Decay each parameter once by its reduction policy, if auto-update is
    /// on. Soar calls this once per decision cycle
    /// (`exploration_update_parameters`). A decay that would make a
    /// parameter invalid (temperature reaching 0) is skipped; upstream
    /// applied it, and Boltzmann then divided by zero (DEVIATIONS.md S3).
    pub fn update_parameters(&mut self) {
        if !self.auto_update {
            return;
        }
        for name in [ParameterName::Epsilon, ParameterName::Temperature] {
            let p = *self.parameter(name);
            let next = match p.reduction_policy {
                ReductionPolicy::Exponential => {
                    if p.exponential_rate == 1.0 {
                        continue;
                    }
                    p.value * p.exponential_rate
                }
                ReductionPolicy::Linear => {
                    if !(p.value > 0.0 && p.linear_rate != 0.0) {
                        continue;
                    }
                    let v = p.value - p.linear_rate;
                    if v > 0.0 {
                        v
                    } else {
                        0.0
                    }
                }
            };
            if valid(name, next) {
                self.parameter_mut(name).value = next;
            }
        }
    }

    /// The numeric value of a candidate: the sum (or, in avg mode, the mean)
    /// of its numeric-indifferent values and of the numeric referents of its
    /// binary-indifferent preferences. A binary-indifferent preference
    /// against another value contributes 0 but still counts towards the
    /// mean. With no contributors the value is `default_value`
    /// (`exploration_compute_value_of_candidate`).
    pub fn compute_value_of_candidate<V: PartialEq>(
        &self,
        slot: &Slot<V>,
        cand: &V,
        default_value: f64,
    ) -> f64 {
        let mut total: u32 = 0;
        let mut value = 0.0;
        for p in slot
            .list(T::NumericIndifferent)
            .iter()
            .chain(slot.list(T::BinaryIndifferent))
        {
            if p.value == *cand {
                total += 1;
                value += p.referent.number();
            }
        }
        if total == 0 {
            value = default_value;
            total = 1;
        }
        if self.numeric_indifferent_mode == NumericIndifferentMode::Avg {
            value /= f64::from(total);
        }
        value
    }

    fn values<V: PartialEq>(&self, slot: &Slot<V>, candidates: &[V]) -> alloc::vec::Vec<f64> {
        candidates
            .iter()
            .map(|c| self.compute_value_of_candidate(slot, c, 0.0))
            .collect()
    }

    /// Index of the chosen candidate (`exploration_choose_according_to_policy`).
    /// `candidates` must be non-empty.
    pub fn choose_according_to_policy<V: PartialEq>(
        &self,
        slot: &Slot<V>,
        candidates: &[V],
        rng: &mut SoarRng,
    ) -> usize {
        let values = self.values(slot, candidates);
        match self.policy {
            Policy::First => 0,
            Policy::Last => candidates.len() - 1,
            Policy::RandomUniform => randomly_select(candidates.len(), rng),
            Policy::Softmax => probabilistically_select(&values, rng),
            Policy::EpsilonGreedy => {
                if rng.rand() < self.epsilon.value {
                    randomly_select(candidates.len(), rng)
                } else {
                    highest_value(&values, rng)
                }
            }
            Policy::Boltzmann => boltzmann_select(&values, self.temperature.value, rng),
        }
    }

    /// Probability that `selection` would be chosen. Draws no random numbers
    /// (`exploration_probability_according_to_policy`).
    pub fn probability_according_to_policy<V: PartialEq>(
        &self,
        slot: &Slot<V>,
        candidates: &[V],
        selection: usize,
    ) -> f64 {
        let values = self.values(slot, candidates);
        let n = candidates.len() as f64;
        match self.policy {
            Policy::First => {
                if selection == 0 {
                    1.0
                } else {
                    0.0
                }
            }
            Policy::Last => {
                if selection + 1 == candidates.len() {
                    1.0
                } else {
                    0.0
                }
            }
            Policy::RandomUniform => 1.0 / n,
            Policy::Softmax => {
                let mut total = 0.0;
                for v in &values {
                    if *v > 0.0 {
                        total += *v;
                    }
                }
                if total > 0.0 {
                    if values[selection] > 0.0 {
                        values[selection] / total
                    } else {
                        0.0
                    }
                } else {
                    1.0 / n
                }
            }
            Policy::EpsilonGreedy => {
                let epsilon = self.epsilon.value;
                let mut top = values[0];
                let mut top_count: u32 = 0;
                for v in &values {
                    if *v > top {
                        top = *v;
                        top_count = 1;
                    } else if *v == top {
                        top_count += 1;
                    }
                }
                let mut p = epsilon / n;
                if values[selection] == top {
                    p += (1.0 - epsilon) / f64::from(top_count);
                }
                p
            }
            Policy::Boltzmann => {
                let t = self.temperature.value;
                let maxq = max_value(&values);
                let mut total = 0.0;
                let mut sel = 0.0;
                for (i, v) in values.iter().enumerate() {
                    let e = petir::real::exp((v - maxq) / t);
                    total += e;
                    if i == selection {
                        sel = e;
                    }
                }
                sel / total
            }
        }
    }
}

/// `exploration_randomly_select`: uniform over `n` candidates.
fn randomly_select(n: usize, rng: &mut SoarRng) -> usize {
    rng.rand_int_max(n as u32 - 1) as usize
}

/// `exploration_probabilistically_select` (softmax).
fn probabilistically_select(values: &[f64], rng: &mut SoarRng) -> usize {
    let mut total = 0.0;
    for v in values {
        if *v > 0.0 {
            total += *v;
        }
    }
    if total == 0.0 {
        return randomly_select(values.len(), rng);
    }
    let selected = total * rng.rand();
    let mut sum = 0.0;
    let mut last_positive = 0;
    for (i, v) in values.iter().enumerate() {
        if *v > 0.0 {
            sum += *v;
            last_positive = i;
            if selected <= sum {
                return i;
            }
        }
    }
    // Unreachable for finite values. Upstream returns NIL here and then
    // dereferences it (DEVIATIONS.md S4).
    last_positive
}

/// `exploration_get_highest_q_value_pref`: the highest value; ties broken
/// uniformly at random.
fn highest_value(values: &[f64], rng: &mut SoarRng) -> usize {
    let mut top_index = 0;
    let mut top = values[0];
    let mut count: u32 = 0;
    for (i, v) in values.iter().enumerate() {
        if *v > top {
            top = *v;
            top_index = i;
            count = 1;
        } else if *v == top {
            count += 1;
        }
    }
    if count == 1 {
        return top_index;
    }
    // Upstream walks to the first value equal to the top, then skips
    // `chosen` more equal values. If the top is NaN no value equals it and
    // upstream runs off the end of the list; here the first candidate is
    // returned instead (DEVIATIONS.md S4).
    let Some(first) = values.iter().position(|v| *v == top) else {
        return 0;
    };
    let mut chosen = rng.rand_int_max(count - 1);
    let mut i = first;
    while chosen > 0 {
        i += 1;
        while values[i] != top {
            i += 1;
        }
        chosen -= 1;
    }
    i
}

fn max_value(values: &[f64]) -> f64 {
    let mut maxq = values[0];
    for v in &values[1..] {
        if maxq < *v {
            maxq = *v;
        }
    }
    maxq
}

/// `exploration_boltzmann_select`.
fn boltzmann_select(values: &[f64], t: f64, rng: &mut SoarRng) -> usize {
    let maxq = max_value(values);
    let mut exps = alloc::vec::Vec::with_capacity(values.len());
    let mut total = 0.0;
    for v in values {
        let e = petir::real::exp((v - maxq) / t);
        exps.push(e);
        total += e;
    }
    let r = rng.rand_max(total);
    let mut sum = 0.0;
    for (i, e) in exps.iter().enumerate() {
        sum += e;
        if sum >= r {
            return i;
        }
    }
    // Unreachable for finite values; upstream returns NIL (DEVIATIONS.md S4).
    values.len() - 1
}

#[cfg(test)]
#[path = "exploration_test.rs"]
mod exploration_test;
