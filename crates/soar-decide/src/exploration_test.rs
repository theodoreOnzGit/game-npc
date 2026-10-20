// SPDX-License-Identifier: AGPL-3.0-only
//! Unit tests for exploration. Upstream Soar has no unit tests for this
//! code; these were written from reading `exploration.cpp`.
//! `tests/differential.rs` checks selections and probabilities against the
//! real `soar` binary.

use super::*;
use crate::preference::{Preference, PreferenceType as P};
use alloc::vec;
use alloc::vec::Vec;

fn numeric_slot(values: &[(u32, f64)]) -> Slot<u32> {
    let mut s = Slot::new();
    for (v, x) in values {
        s.push(Preference::numeric(*v, *x));
    }
    s
}

#[test]
fn defaults_match_soar() {
    let e = Exploration::default();
    assert_eq!(e.policy, Policy::Softmax);
    assert_eq!(e.epsilon(), 0.1);
    assert_eq!(e.temperature(), 25.0);
    assert_eq!(e.numeric_indifferent_mode, NumericIndifferentMode::Sum);
    assert!(!e.auto_update);
}

#[test]
fn values_sum_or_average() {
    let mut s = numeric_slot(&[(1, 0.5), (1, 0.25)]);
    s.push(Preference::binary(P::BinaryIndifferent, 1, 2));
    let mut e = Exploration::default();
    assert_eq!(e.compute_value_of_candidate(&s, &1, 0.0), 0.75);
    e.numeric_indifferent_mode = NumericIndifferentMode::Avg;
    // The binary-indifferent preference counts: 0.75 / 3.
    assert_eq!(e.compute_value_of_candidate(&s, &1, 0.0), 0.25);
    assert_eq!(e.compute_value_of_candidate(&s, &7, 0.5), 0.5);
}

#[test]
fn parameter_validation() {
    let mut e = Exploration::default();
    assert_eq!(
        e.set_parameter(ParameterName::Epsilon, 1.5),
        Err(ExplorationError::InvalidEpsilon)
    );
    assert_eq!(
        e.set_parameter(ParameterName::Temperature, 0.0),
        Err(ExplorationError::InvalidTemperature)
    );
    assert!(e.set_parameter(ParameterName::Temperature, 0.5).is_ok());
    assert_eq!(
        e.set_reduction_rate(ParameterName::Epsilon, ReductionPolicy::Exponential, 2.0),
        Err(ExplorationError::InvalidReductionRate)
    );
    assert_eq!(
        e.set_reduction_rate(ParameterName::Epsilon, ReductionPolicy::Linear, -1.0),
        Err(ExplorationError::InvalidReductionRate)
    );
}

#[test]
fn parameters_decay_only_with_auto_update() {
    let mut e = Exploration::default();
    e.set_reduction_rate(ParameterName::Epsilon, ReductionPolicy::Exponential, 0.5)
        .unwrap();
    e.update_parameters();
    assert_eq!(e.epsilon(), 0.1);
    e.auto_update = true;
    e.update_parameters();
    assert_eq!(e.epsilon(), 0.05);
}

#[test]
fn linear_decay_floors_epsilon_and_never_zeroes_temperature() {
    let mut e = Exploration {
        auto_update: true,
        ..Exploration::default()
    };
    for name in [ParameterName::Epsilon, ParameterName::Temperature] {
        e.set_reduction_rate(name, ReductionPolicy::Linear, 30.0)
            .unwrap();
    }
    e.parameter_mut(ParameterName::Epsilon).reduction_policy = ReductionPolicy::Linear;
    e.parameter_mut(ParameterName::Temperature).reduction_policy = ReductionPolicy::Linear;
    e.update_parameters();
    assert_eq!(e.epsilon(), 0.0);
    // Upstream would set temperature to 0 (DEVIATIONS.md S3).
    assert_eq!(e.temperature(), 25.0);
}

#[test]
fn first_and_last() {
    let s = numeric_slot(&[]);
    let mut rng = SoarRng::new(1);
    let c = [10u32, 20, 30];
    assert_eq!(
        Exploration::new(Policy::First).choose_according_to_policy(&s, &c, &mut rng),
        0
    );
    assert_eq!(
        Exploration::new(Policy::Last).choose_according_to_policy(&s, &c, &mut rng),
        2
    );
    assert_eq!(
        Exploration::new(Policy::Last).probability_according_to_policy(&s, &c, 2),
        1.0
    );
    assert_eq!(
        Exploration::new(Policy::First).probability_according_to_policy(&s, &c, 2),
        0.0
    );
}

/// Each random policy's empirical frequencies approach its own
/// `probability_according_to_policy`.
#[test]
fn empirical_frequencies_match_reported_probabilities() {
    let s = numeric_slot(&[(0, 0.5), (1, 1.5), (2, -1.0), (3, 1.5)]);
    let c = [0u32, 1, 2, 3];
    let mut eps = Exploration::new(Policy::EpsilonGreedy);
    eps.set_parameter(ParameterName::Epsilon, 0.3).unwrap();
    let mut boltz = Exploration::new(Policy::Boltzmann);
    boltz
        .set_parameter(ParameterName::Temperature, 0.7)
        .unwrap();
    for e in [
        Exploration::new(Policy::Softmax),
        Exploration::new(Policy::RandomUniform),
        eps,
        boltz,
    ] {
        let mut rng = SoarRng::new(2024);
        let n = 40_000;
        let mut counts = [0u32; 4];
        for _ in 0..n {
            counts[e.choose_according_to_policy(&s, &c, &mut rng)] += 1;
        }
        let mut total_p = 0.0;
        for (i, count) in counts.iter().enumerate() {
            let p = e.probability_according_to_policy(&s, &c, i);
            total_p += p;
            let f = f64::from(*count) / f64::from(n);
            assert!(
                (f - p).abs() < 0.01,
                "{:?} candidate {i}: freq {f} vs p {p}",
                e.policy
            );
        }
        assert!((total_p - 1.0).abs() < 1e-12, "{:?}", e.policy);
    }
}

#[test]
fn softmax_without_positive_values_is_uniform() {
    let s = numeric_slot(&[(0, -1.0), (1, 0.0)]);
    let e = Exploration::new(Policy::Softmax);
    assert_eq!(e.probability_according_to_policy(&s, &[0u32, 1], 0), 0.5);
    let mut rng = SoarRng::new(9);
    let picks: Vec<usize> = (0..50)
        .map(|_| e.choose_according_to_policy(&s, &[0u32, 1], &mut rng))
        .collect();
    assert!(picks.contains(&0) && picks.contains(&1));
}

#[test]
fn greedy_breaks_ties_at_random() {
    let s = numeric_slot(&[(0, 1.0), (1, 2.0), (2, 2.0)]);
    let mut e = Exploration::new(Policy::EpsilonGreedy);
    e.set_parameter(ParameterName::Epsilon, 0.0).unwrap();
    let mut rng = SoarRng::new(3);
    let picks: Vec<usize> = (0..100)
        .map(|_| e.choose_according_to_policy(&s, &[0u32, 1, 2], &mut rng))
        .collect();
    assert!(!picks.contains(&0));
    assert!(picks.contains(&1) && picks.contains(&2));
}

#[test]
fn nan_values_do_not_panic() {
    let s = numeric_slot(&[(0, f64::NAN), (1, f64::NAN)]);
    let c = [0u32, 1];
    let mut rng = SoarRng::new(1);
    for policy in [Policy::Softmax, Policy::EpsilonGreedy, Policy::Boltzmann] {
        let i = Exploration::new(policy).choose_according_to_policy(&s, &c, &mut rng);
        assert!(i < 2);
    }
    let _ = vec![0];
}
