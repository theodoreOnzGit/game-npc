//! Actions and their deterministic validation.
//!
//! An [`Action`] is only a proposal. The world accepts nothing but a
//! [`ValidatedAction`], whose field is private, so the only way to obtain one
//! is [`validate`]. Neither a motivation model nor a planner can bypass the
//! precondition checks.

use crate::world::WorldState;

/// Who a social action is directed at.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SocialTarget {
    Anyone,
    Group,
    Companion,
}

/// A single concrete thing a character can attempt this step.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Action {
    Eat,
    Sleep,
    Flee,
    TakeShelter,
    Explore,
    Practice,
    Work,
    Talk(SocialTarget),
    Perform,
    Help,
    Refuse,
    Organise,
    Tidy,
    Gather,
    Confront,
    Create,
    Withdraw,
    Relax,
}

impl Action {
    /// Stable wire code, used in decision checksums. Never renumber.
    pub const fn code(self) -> u8 {
        match self {
            Action::Eat => 0,
            Action::Sleep => 1,
            Action::Flee => 2,
            Action::TakeShelter => 3,
            Action::Explore => 4,
            Action::Practice => 5,
            Action::Work => 6,
            Action::Talk(SocialTarget::Anyone) => 7,
            Action::Talk(SocialTarget::Group) => 8,
            Action::Talk(SocialTarget::Companion) => 9,
            Action::Perform => 10,
            Action::Help => 11,
            Action::Refuse => 12,
            Action::Organise => 13,
            Action::Tidy => 14,
            Action::Gather => 15,
            Action::Confront => 16,
            Action::Create => 17,
            Action::Withdraw => 18,
            Action::Relax => 19,
        }
    }
}

/// Why an action failed validation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ActionRejection {
    NoFood,
    NoBed,
    NoShelter,
    NoEscapeRoute,
    NothingToExplore,
    NoTask,
    NobodyToTalkTo,
    NoAudience,
    NobodyToHelp,
    NotCoerced,
    NoDisorder,
    NoResources,
    NoWrongdoer,
    UnsafeToIdle,
}

/// An action that has passed [`validate`] against a specific world state.
/// Cannot be constructed any other way.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ValidatedAction {
    action: Action,
}

impl ValidatedAction {
    pub const fn action(&self) -> Action {
        self.action
    }
}

/// Check an action's preconditions against the world.
pub fn validate(action: Action, world: &WorldState) -> Result<ValidatedAction, ActionRejection> {
    use ActionRejection as R;
    let threatened = world.threat > 0;
    let check = |ok: bool, why: ActionRejection| if ok { Ok(()) } else { Err(why) };
    match action {
        Action::Eat => check(world.food > 0, R::NoFood),
        Action::Sleep => {
            check(world.bed_available, R::NoBed)?;
            check(!threatened, R::UnsafeToIdle)
        }
        Action::Flee => check(world.escape_route, R::NoEscapeRoute),
        Action::TakeShelter => check(world.shelter_available, R::NoShelter),
        Action::Explore => check(world.unexplored_sites > 0, R::NothingToExplore),
        Action::Practice | Action::Create => check(!threatened, R::UnsafeToIdle),
        Action::Work => check(world.pending_tasks > 0, R::NoTask),
        Action::Talk(SocialTarget::Anyone) => check(world.others_nearby > 0, R::NobodyToTalkTo),
        Action::Talk(SocialTarget::Group) => check(world.group_nearby, R::NobodyToTalkTo),
        Action::Talk(SocialTarget::Companion) => check(world.companion_nearby, R::NobodyToTalkTo),
        Action::Perform => check(world.others_nearby > 0, R::NoAudience),
        Action::Help => check(world.dependant_in_need, R::NobodyToHelp),
        Action::Refuse => check(world.coerced, R::NotCoerced),
        Action::Organise | Action::Tidy => check(world.disorder > 0, R::NoDisorder),
        Action::Gather => check(world.resources > 0, R::NoResources),
        Action::Confront => check(world.wrongdoer_nearby, R::NoWrongdoer),
        Action::Withdraw | Action::Relax => check(!threatened, R::UnsafeToIdle),
    }?;
    Ok(ValidatedAction { action })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eat_needs_food() {
        let mut w = WorldState::default();
        assert_eq!(validate(Action::Eat, &w), Err(ActionRejection::NoFood));
        w.food = 1;
        assert_eq!(validate(Action::Eat, &w).unwrap().action(), Action::Eat);
    }

    #[test]
    fn cannot_sleep_under_threat() {
        let w = WorldState {
            bed_available: true,
            threat: 1,
            ..WorldState::default()
        };
        assert_eq!(
            validate(Action::Sleep, &w),
            Err(ActionRejection::UnsafeToIdle)
        );
    }

    #[test]
    fn relax_is_always_valid_when_safe() {
        assert!(validate(Action::Relax, &WorldState::default()).is_ok());
    }
}
