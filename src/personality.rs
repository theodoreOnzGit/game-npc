//! Personality: stable, per-character weights on each motivation.

use crate::motivation::{Motivation, MOTIVATION_COUNT};
use crate::unit;

/// How strongly a character is disposed towards each motivation, in `[0, 1]`.
///
/// `0.5` is a neutral disposition. Personality scales how much a given need
/// turns into motivation, so the same event yields different priorities for
/// different characters. It changes slowly, if at all; it is not state.
///
/// This is deliberately motivation-indexed rather than a trait model (e.g.
/// Big Five) for v0.1. Reiss (2004) treats individual differences as
/// differences in the priority of basic desires, which is the closest
/// literature match to this representation; mapping trait models onto it is
/// future work.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Personality {
    weights: [f32; MOTIVATION_COUNT],
}

impl Default for Personality {
    fn default() -> Self {
        Self::neutral()
    }
}

impl Personality {
    /// Every weight at 0.5.
    pub const fn neutral() -> Self {
        Self {
            weights: [0.5; MOTIVATION_COUNT],
        }
    }

    /// Return a copy with one weight changed (clamped to `[0, 1]`).
    pub fn with(mut self, m: Motivation, weight: f32) -> Self {
        self.weights[m.index()] = unit(weight);
        self
    }

    pub fn weight(&self, m: Motivation) -> f32 {
        self.weights[m.index()]
    }

    pub const fn as_array(&self) -> &[f32; MOTIVATION_COUNT] {
        &self.weights
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_sets_and_clamps() {
        let p = Personality::neutral()
            .with(Motivation::Curiosity, 0.9)
            .with(Motivation::Safety, 7.0);
        assert_eq!(p.weight(Motivation::Curiosity), 0.9);
        assert_eq!(p.weight(Motivation::Safety), 1.0);
        assert_eq!(p.weight(Motivation::Rest), 0.5);
    }
}
