//! The fixed-size motivation priority vector.

use crate::motivation::{Motivation, MOTIVATION_COUNT};
use crate::unit;

/// Strength of every motivation for one character, each in `[0, 1]`.
///
/// Many motivations can be active at once; conflicting motivations simply
/// both carry weight. 25 × `f32` = 100 bytes per character.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MotivationVector {
    values: [f32; MOTIVATION_COUNT],
}

impl Default for MotivationVector {
    fn default() -> Self {
        Self::zero()
    }
}

impl MotivationVector {
    /// All motivations at zero.
    pub const fn zero() -> Self {
        Self {
            values: [0.0; MOTIVATION_COUNT],
        }
    }

    /// Build from raw values; each is clamped to `[0, 1]` (NaN becomes 0).
    pub fn from_array(values: [f32; MOTIVATION_COUNT]) -> Self {
        Self {
            values: values.map(unit),
        }
    }

    /// The raw values, indexed by [`Motivation::index`].
    pub const fn as_array(&self) -> &[f32; MOTIVATION_COUNT] {
        &self.values
    }

    pub fn get(&self, m: Motivation) -> f32 {
        self.values[m.index()]
    }

    /// Set one motivation, clamped to `[0, 1]`.
    pub fn set(&mut self, m: Motivation, value: f32) {
        self.values[m.index()] = unit(value);
    }

    /// Motivations sorted by descending strength. Ties are broken by stable
    /// id, so the ordering is fully deterministic.
    pub fn ranked(&self) -> [Motivation; MOTIVATION_COUNT] {
        let mut order = Motivation::ALL;
        order.sort_unstable_by(|a, b| {
            self.get(*b)
                .total_cmp(&self.get(*a))
                .then(a.id().cmp(&b.id()))
        });
        order
    }

    /// The strongest motivation (ties broken by stable id).
    pub fn top(&self) -> Motivation {
        self.ranked()[0]
    }

    /// Largest absolute per-motivation difference to `other`.
    pub fn max_abs_diff(&self, other: &MotivationVector) -> f32 {
        self.values
            .iter()
            .zip(other.values.iter())
            .map(|(a, b)| (a - b).abs())
            .fold(0.0, f32::max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_is_100_bytes() {
        assert_eq!(core::mem::size_of::<MotivationVector>(), 100);
    }

    #[test]
    fn set_clamps() {
        let mut v = MotivationVector::zero();
        v.set(Motivation::Safety, 3.0);
        v.set(Motivation::Rest, -1.0);
        assert_eq!(v.get(Motivation::Safety), 1.0);
        assert_eq!(v.get(Motivation::Rest), 0.0);
    }

    #[test]
    fn ranking_is_deterministic_on_ties() {
        let mut v = MotivationVector::zero();
        v.set(Motivation::Justice, 0.5);
        v.set(Motivation::Rest, 0.5);
        v.set(Motivation::Curiosity, 0.9);
        let r = v.ranked();
        assert_eq!(r[0], Motivation::Curiosity);
        // Rest (0x01) precedes Justice (0x41) on a tie.
        assert_eq!(r[1], Motivation::Rest);
        assert_eq!(r[2], Motivation::Justice);
        // With all zeros, order is by stable id.
        assert_eq!(MotivationVector::zero().top(), Motivation::Sustenance);
    }

    #[test]
    fn multiple_motivations_coexist() {
        let mut v = MotivationVector::zero();
        v.set(Motivation::Safety, 0.8);
        v.set(Motivation::Caregiving, 0.8);
        assert_eq!(v.get(Motivation::Safety), v.get(Motivation::Caregiving));
    }
}
