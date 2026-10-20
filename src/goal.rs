//! Symbolic goals: concrete target states generated from motivations.
//!
//! A goal is what the character sets out to bring about; the planner then
//! turns it into a single [`crate::Action`]. Several motivations can map to
//! the same goal (e.g. Connection, Belonging and Affection all lead to some
//! form of socialising).

use crate::action::{Action, SocialTarget};
use crate::motivation::Motivation;

/// A concrete target state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Goal {
    BeFed,
    BeRested,
    BeHealed,
    BeSafe,
    BeSheltered,
    UnderstandSomething,
    ImproveSkill,
    FinishTask,
    ExperienceSomethingNew,
    Socialise(SocialTarget),
    GainRecognition,
    HelpDependant,
    ActIndependently,
    TakeCharge,
    RestoreOrder,
    ObtainResources,
    FulfilObligation,
    RightAWrong,
    ExpressSelf,
    FindPurpose,
    HaveFun,
    Unwind,
    RecoverEmotionally,
}

/// The goal a motivation calls for, independent of whether it is feasible.
pub const fn generate_goal(m: Motivation) -> Goal {
    use Motivation::*;
    match m {
        Sustenance => Goal::BeFed,
        Rest => Goal::BeRested,
        Health => Goal::BeHealed,
        Safety => Goal::BeSafe,
        Comfort => Goal::BeSheltered,
        Curiosity => Goal::UnderstandSomething,
        Mastery => Goal::ImproveSkill,
        Achievement => Goal::FinishTask,
        Novelty => Goal::ExperienceSomethingNew,
        Connection => Goal::Socialise(SocialTarget::Anyone),
        Belonging => Goal::Socialise(SocialTarget::Group),
        Affection => Goal::Socialise(SocialTarget::Companion),
        Recognition => Goal::GainRecognition,
        Caregiving => Goal::HelpDependant,
        Autonomy => Goal::ActIndependently,
        Control => Goal::TakeCharge,
        Order => Goal::RestoreOrder,
        Acquisition => Goal::ObtainResources,
        Duty => Goal::FulfilObligation,
        Justice => Goal::RightAWrong,
        SelfExpression => Goal::ExpressSelf,
        Meaning => Goal::FindPurpose,
        Pleasure => Goal::HaveFun,
        StressRelief => Goal::Unwind,
        EmotionalRecovery => Goal::RecoverEmotionally,
    }
}

/// Deterministic planner: the single action that pursues a goal. Whether it
/// is *possible* is decided separately by [`crate::validate`].
pub const fn plan(goal: Goal) -> Action {
    match goal {
        Goal::BeFed => Action::Eat,
        Goal::BeRested | Goal::BeHealed => Action::Sleep,
        Goal::BeSafe => Action::Flee,
        Goal::BeSheltered => Action::TakeShelter,
        Goal::UnderstandSomething | Goal::ExperienceSomethingNew => Action::Explore,
        Goal::ImproveSkill => Action::Practice,
        Goal::FinishTask | Goal::FulfilObligation => Action::Work,
        Goal::Socialise(target) => Action::Talk(target),
        Goal::GainRecognition => Action::Perform,
        Goal::HelpDependant => Action::Help,
        Goal::ActIndependently => Action::Refuse,
        Goal::TakeCharge => Action::Organise,
        Goal::RestoreOrder => Action::Tidy,
        Goal::ObtainResources => Action::Gather,
        Goal::RightAWrong => Action::Confront,
        Goal::ExpressSelf => Action::Create,
        Goal::FindPurpose | Goal::RecoverEmotionally => Action::Withdraw,
        Goal::HaveFun | Goal::Unwind => Action::Relax,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_motivation_yields_a_plannable_goal() {
        for m in Motivation::ALL {
            let _ = plan(generate_goal(m));
        }
    }

    #[test]
    fn social_motivations_differ_in_target() {
        assert_eq!(
            plan(generate_goal(Motivation::Affection)),
            Action::Talk(SocialTarget::Companion)
        );
        assert_eq!(
            plan(generate_goal(Motivation::Belonging)),
            Action::Talk(SocialTarget::Group)
        );
    }
}
