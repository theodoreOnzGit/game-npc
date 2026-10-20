//! The motivation taxonomy, v0.1 (GitHub issue #1).
//!
//! Twenty-five motivations in six domains. This is an **engineering
//! synthesis**, not a reproduction of any one publication; the definitions and
//! the candidate correspondences to the literature are in
//! `docs/motivation-taxonomy.md`, which records what has and has not yet been
//! checked against the sources.
//!
//! **Count.** Issue #1 says "24 motivations" and "96 bytes per character", but
//! its domain table and its proposed enum both list **25**. The listed
//! motivations are implemented as written; the vector is therefore 25 × 4 =
//! 100 bytes. Which motivation (if any) should be dropped is an open question
//! for the taxonomy review.
//!
//! **Stable identifiers.** Each motivation has a stable wire id
//! ([`Motivation::id`]) that is independent of declaration order, so the enum
//! can be reordered or extended without breaking serialized data. Ids are
//! grouped by domain in blocks of 16 (`0x0_`, `0x1_`, …), leaving room for
//! additions inside a domain. The dense index ([`Motivation::index`]) is what
//! addresses a [`crate::MotivationVector`] and is **not** stable across
//! taxonomy versions.

/// Number of motivations in the taxonomy (length of a motivation vector).
pub const MOTIVATION_COUNT: usize = 25;

/// A top-level motivation domain.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Domain {
    PhysiologicalProtective,
    ExplorationMastery,
    SocialRelational,
    AutonomyControl,
    ValuesIdentity,
    EmotionalRegulation,
}

impl Domain {
    /// Every domain, in taxonomy order.
    pub const ALL: [Domain; 6] = [
        Domain::PhysiologicalProtective,
        Domain::ExplorationMastery,
        Domain::SocialRelational,
        Domain::AutonomyControl,
        Domain::ValuesIdentity,
        Domain::EmotionalRegulation,
    ];

    /// Human-readable name, as in the issue's domain table.
    pub const fn name(self) -> &'static str {
        match self {
            Domain::PhysiologicalProtective => "Physiological and protective",
            Domain::ExplorationMastery => "Exploration and mastery",
            Domain::SocialRelational => "Social and relational",
            Domain::AutonomyControl => "Autonomy and control",
            Domain::ValuesIdentity => "Values and identity",
            Domain::EmotionalRegulation => "Emotional regulation and enjoyment",
        }
    }

    /// The motivations belonging to this domain, in taxonomy order.
    pub fn motivations(self) -> impl Iterator<Item = Motivation> {
        Motivation::ALL
            .into_iter()
            .filter(move |m| m.domain() == self)
    }
}

/// A single motivation: a persistent want that may or may not be acted upon.
///
/// A motivation is not a goal (a concrete target state), not an action, not
/// an emotion and not a resource; those have their own types.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Motivation {
    // Physiological and protective
    Sustenance,
    Rest,
    Health,
    Safety,
    Comfort,
    // Exploration and mastery
    Curiosity,
    Mastery,
    Achievement,
    Novelty,
    // Social and relational
    Connection,
    Belonging,
    Affection,
    Recognition,
    Caregiving,
    // Autonomy and control
    Autonomy,
    Control,
    Order,
    Acquisition,
    // Values and identity
    Duty,
    Justice,
    SelfExpression,
    Meaning,
    // Emotional regulation and enjoyment
    Pleasure,
    StressRelief,
    EmotionalRecovery,
}

impl Motivation {
    /// Every motivation, in dense-index order.
    pub const ALL: [Motivation; MOTIVATION_COUNT] = [
        Motivation::Sustenance,
        Motivation::Rest,
        Motivation::Health,
        Motivation::Safety,
        Motivation::Comfort,
        Motivation::Curiosity,
        Motivation::Mastery,
        Motivation::Achievement,
        Motivation::Novelty,
        Motivation::Connection,
        Motivation::Belonging,
        Motivation::Affection,
        Motivation::Recognition,
        Motivation::Caregiving,
        Motivation::Autonomy,
        Motivation::Control,
        Motivation::Order,
        Motivation::Acquisition,
        Motivation::Duty,
        Motivation::Justice,
        Motivation::SelfExpression,
        Motivation::Meaning,
        Motivation::Pleasure,
        Motivation::StressRelief,
        Motivation::EmotionalRecovery,
    ];

    /// Dense index into a [`crate::MotivationVector`]. Not stable across
    /// taxonomy versions; use [`Motivation::id`] for serialization.
    pub const fn index(self) -> usize {
        self as usize
    }

    /// Stable wire identifier. Never reuse or renumber a published id.
    pub const fn id(self) -> u8 {
        use Motivation::*;
        match self {
            Sustenance => 0x00,
            Rest => 0x01,
            Health => 0x02,
            Safety => 0x03,
            Comfort => 0x04,
            Curiosity => 0x10,
            Mastery => 0x11,
            Achievement => 0x12,
            Novelty => 0x13,
            Connection => 0x20,
            Belonging => 0x21,
            Affection => 0x22,
            Recognition => 0x23,
            Caregiving => 0x24,
            Autonomy => 0x30,
            Control => 0x31,
            Order => 0x32,
            Acquisition => 0x33,
            Duty => 0x40,
            Justice => 0x41,
            SelfExpression => 0x42,
            Meaning => 0x43,
            Pleasure => 0x50,
            StressRelief => 0x51,
            EmotionalRecovery => 0x52,
        }
    }

    /// Inverse of [`Motivation::id`].
    pub fn from_id(id: u8) -> Option<Motivation> {
        Motivation::ALL.into_iter().find(|m| m.id() == id)
    }

    /// The domain this motivation belongs to.
    pub const fn domain(self) -> Domain {
        use Motivation::*;
        match self {
            Sustenance | Rest | Health | Safety | Comfort => Domain::PhysiologicalProtective,
            Curiosity | Mastery | Achievement | Novelty => Domain::ExplorationMastery,
            Connection | Belonging | Affection | Recognition | Caregiving => {
                Domain::SocialRelational
            }
            Autonomy | Control | Order | Acquisition => Domain::AutonomyControl,
            Duty | Justice | SelfExpression | Meaning => Domain::ValuesIdentity,
            Pleasure | StressRelief | EmotionalRecovery => Domain::EmotionalRegulation,
        }
    }

    /// Display name.
    pub const fn name(self) -> &'static str {
        use Motivation::*;
        match self {
            Sustenance => "Sustenance",
            Rest => "Rest",
            Health => "Health",
            Safety => "Safety",
            Comfort => "Comfort",
            Curiosity => "Curiosity",
            Mastery => "Mastery",
            Achievement => "Achievement",
            Novelty => "Novelty",
            Connection => "Connection",
            Belonging => "Belonging",
            Affection => "Affection",
            Recognition => "Recognition",
            Caregiving => "Caregiving",
            Autonomy => "Autonomy",
            Control => "Control",
            Order => "Order",
            Acquisition => "Acquisition",
            Duty => "Duty",
            Justice => "Justice",
            SelfExpression => "SelfExpression",
            Meaning => "Meaning",
            Pleasure => "Pleasure",
            StressRelief => "StressRelief",
            EmotionalRecovery => "EmotionalRecovery",
        }
    }

    /// One-line working definition. The full definitions, with provenance
    /// notes, are in `docs/motivation-taxonomy.md`.
    pub const fn definition(self) -> &'static str {
        use Motivation::*;
        match self {
            Sustenance => "Want to obtain food and drink.",
            Rest => "Want to recover physical energy through sleep or inactivity.",
            Health => "Want to avoid or recover from injury and illness.",
            Safety => "Want to avoid physical threat and harm.",
            Comfort => "Want physically pleasant, non-aversive surroundings (warmth, shelter).",
            Curiosity => "Want to understand: to gain information about something specific.",
            Mastery => "Want to improve one's own competence at a skill.",
            Achievement => "Want to complete tasks and attain externally visible accomplishments.",
            Novelty => "Want new, varied experiences for their own sake.",
            Connection => "Want contact and interaction with other people in general.",
            Belonging => "Want acceptance as a member of a group.",
            Affection => "Want close, warm relationships with specific individuals.",
            Recognition => "Want status, respect and acknowledgement from others.",
            Caregiving => "Want to protect and provide for dependants and others in need.",
            Autonomy => "Want one's actions to be self-chosen rather than imposed.",
            Control => "Want influence over other people and events.",
            Order => "Want predictability, organisation and routine.",
            Acquisition => "Want to gain and keep possessions and resources.",
            Duty => "Want to fulfil obligations and commitments one has accepted.",
            Justice => "Want fairness, and to see wrongs set right.",
            SelfExpression => "Want to express one's identity, ideas and creativity.",
            Meaning => "Want one's life and actions to serve a larger purpose.",
            Pleasure => "Want enjoyable experiences and fun.",
            StressRelief => "Want to reduce acute stress and tension.",
            EmotionalRecovery => "Want to restore emotional reserves after depletion.",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_is_in_index_order_and_complete() {
        assert_eq!(Motivation::ALL.len(), MOTIVATION_COUNT);
        for (i, m) in Motivation::ALL.iter().enumerate() {
            assert_eq!(m.index(), i);
        }
    }

    #[test]
    fn ids_are_unique_and_round_trip() {
        for a in Motivation::ALL {
            assert_eq!(Motivation::from_id(a.id()), Some(a));
            for b in Motivation::ALL {
                if a != b {
                    assert_ne!(a.id(), b.id(), "{a:?} and {b:?} share an id");
                }
            }
        }
        assert_eq!(Motivation::from_id(0xFF), None);
    }

    #[test]
    fn id_high_nibble_encodes_domain() {
        for m in Motivation::ALL {
            let domain_index = Domain::ALL.iter().position(|d| *d == m.domain()).unwrap();
            assert_eq!((m.id() >> 4) as usize, domain_index, "{m:?}");
        }
    }

    #[test]
    fn domain_sizes_match_issue_table() {
        let sizes: [usize; 6] = Domain::ALL.map(|d| d.motivations().count());
        assert_eq!(sizes, [5, 4, 5, 4, 4, 3]);
        assert_eq!(sizes.iter().sum::<usize>(), MOTIVATION_COUNT);
    }

    #[test]
    fn every_motivation_has_a_definition() {
        for m in Motivation::ALL {
            assert!(!m.definition().is_empty());
            assert!(!m.name().is_empty());
        }
    }
}
