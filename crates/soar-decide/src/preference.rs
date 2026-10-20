// SPDX-License-Identifier: AGPL-3.0-only
// Translated from Soar `Core/SoarKernel/src/shared/enums.h`,
// `soar_representation/preference.{h,cpp}` and `slot.cpp`
// (https://github.com/SoarGroup/Soar, commit 3fc337a, BSD, Copyright
// 1995-2012 Carnegie Mellon University, The Regents of the University of
// Michigan, USC/ISI). Modified: see docs/DEVIATIONS.md.

//! Preferences and the slot that collects them.

use alloc::vec::Vec;

/// Preference types, with Soar's numbering (`enums.h`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum PreferenceType {
    /// `+`
    Acceptable = 0,
    /// `!`
    Require = 1,
    /// `-`
    Reject = 2,
    /// `~`
    Prohibit = 3,
    /// `@` (parsed by Soar, unused by the decision procedure)
    Reconsider = 4,
    /// `=` (unary)
    UnaryIndifferent = 5,
    /// unused by the decision procedure
    UnaryParallel = 6,
    /// `>` (unary)
    Best = 7,
    /// `<` (unary)
    Worst = 8,
    /// `= <x>`
    BinaryIndifferent = 9,
    /// unused by the decision procedure
    BinaryParallel = 10,
    /// `> <x>`
    Better = 11,
    /// `< <x>`
    Worse = 12,
    /// `= number`
    NumericIndifferent = 13,
}

/// Number of preference types (`NUM_PREFERENCE_TYPES`).
pub const NUM_PREFERENCE_TYPES: usize = 14;

impl PreferenceType {
    pub const ALL: [PreferenceType; NUM_PREFERENCE_TYPES] = [
        Self::Acceptable,
        Self::Require,
        Self::Reject,
        Self::Prohibit,
        Self::Reconsider,
        Self::UnaryIndifferent,
        Self::UnaryParallel,
        Self::Best,
        Self::Worst,
        Self::BinaryIndifferent,
        Self::BinaryParallel,
        Self::Better,
        Self::Worse,
        Self::NumericIndifferent,
    ];

    /// Binary types compare a value with a referent.
    pub const fn is_binary(self) -> bool {
        matches!(
            self,
            Self::BinaryIndifferent | Self::BinaryParallel | Self::Better | Self::Worse
        )
    }
}

/// The second operand of a preference.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Referent<V> {
    /// Unary preferences.
    None,
    /// Binary preferences: another value.
    Value(V),
    /// Numeric-indifferent preferences, or any binary preference whose
    /// referent is a number.
    Number(f64),
}

impl<V> Referent<V> {
    /// Soar's `get_number_from_symbol`: the number, or 0 for anything else.
    pub fn number(&self) -> f64 {
        match self {
            Referent::Number(x) => *x,
            _ => 0.0,
        }
    }

    pub(crate) fn value(&self) -> Option<&V> {
        match self {
            Referent::Value(v) => Some(v),
            _ => None,
        }
    }
}

/// One preference for a value in a slot (Soar: `preference`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Preference<V> {
    pub kind: PreferenceType,
    pub value: V,
    pub referent: Referent<V>,
}

impl<V> Preference<V> {
    pub fn unary(kind: PreferenceType, value: V) -> Self {
        Self {
            kind,
            value,
            referent: Referent::None,
        }
    }

    pub fn binary(kind: PreferenceType, value: V, referent: V) -> Self {
        Self {
            kind,
            value,
            referent: Referent::Value(referent),
        }
    }

    pub fn numeric(value: V, number: f64) -> Self {
        Self {
            kind: PreferenceType::NumericIndifferent,
            value,
            referent: Referent::Number(number),
        }
    }
}

/// The preferences for one (identifier, attribute) slot, one list per type
/// (Soar: `slot::preferences[]`).
///
/// **List order matters**: the decision procedure builds its candidate list
/// by pushing onto the front while walking these lists, and both the
/// better/worse conflict marking and the `first`/`last`/random policies
/// depend on that order. Soar inserts a newly asserted preference at the
/// *head* of its list ([`Slot::assert_preference`]). [`Slot::push`] appends
/// instead, for building a slot in a known list order, for example from
/// Soar's `preferences` command output.
#[derive(Clone, Debug, PartialEq)]
pub struct Slot<V> {
    lists: [Vec<Preference<V>>; NUM_PREFERENCE_TYPES],
}

impl<V> Default for Slot<V> {
    fn default() -> Self {
        Self {
            lists: core::array::from_fn(|_| Vec::new()),
        }
    }
}

impl<V> Slot<V> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Append to the end of its type's list.
    pub fn push(&mut self, pref: Preference<V>) {
        self.lists[pref.kind as usize].push(pref);
    }

    /// Insert at the head of its type's list, as Soar does when a
    /// preference is asserted (`add_preference_to_slot`, same goal level).
    pub fn assert_preference(&mut self, pref: Preference<V>) {
        self.lists[pref.kind as usize].insert(0, pref);
    }

    /// The list for one type, in Soar's list order.
    pub fn list(&self, kind: PreferenceType) -> &[Preference<V>] {
        &self.lists[kind as usize]
    }

    /// True if the slot holds no preferences at all
    /// (`!s->all_preferences`).
    pub fn is_empty(&self) -> bool {
        self.lists.iter().all(|l| l.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbering_matches_soar_enums() {
        for (i, t) in PreferenceType::ALL.iter().enumerate() {
            assert_eq!(*t as usize, i);
        }
        assert_eq!(PreferenceType::NumericIndifferent as u8, 13);
    }

    #[test]
    fn assert_inserts_at_head_push_appends() {
        let mut s: Slot<u32> = Slot::new();
        s.assert_preference(Preference::unary(PreferenceType::Acceptable, 1));
        s.assert_preference(Preference::unary(PreferenceType::Acceptable, 2));
        s.push(Preference::unary(PreferenceType::Acceptable, 3));
        let order: Vec<u32> = s
            .list(PreferenceType::Acceptable)
            .iter()
            .map(|p| p.value)
            .collect();
        assert_eq!(order, [2, 1, 3]);
        assert!(!s.is_empty());
        assert!(Slot::<u32>::new().is_empty());
    }

    #[test]
    fn referent_number_defaults_to_zero() {
        assert_eq!(Referent::<u32>::Number(0.5).number(), 0.5);
        assert_eq!(Referent::<u32>::Value(3).number(), 0.0);
        assert_eq!(Referent::<u32>::None.number(), 0.0);
        assert!(PreferenceType::Better.is_binary());
        assert!(!PreferenceType::Best.is_binary());
    }
}
