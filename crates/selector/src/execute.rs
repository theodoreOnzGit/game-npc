// SPDX-License-Identifier: AGPL-3.0-only

//! Delegated execution: the typed interface a subroutine implements.
//!
//! The selector picks a [`ReasoningTask`]; the host then runs the matching
//! [`Subroutine`] through [`execute`]. That call:
//! - checks that the subroutine really is the selected one;
//! - checks the input precondition;
//! - enforces the work limit;
//! - propagates the subroutine's own error, typed.
//!
//! Implementations live outside this crate (see `tests/delegation.rs`
//! for GOAP and PETIR adapters), so the selector depends on none of them.

use crate::task::ReasoningTask;

/// Execution limits handed to a subroutine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    /// Maximum work units (iterations, expanded nodes, plan depth, …).
    pub max_work: u32,
}

/// A reasoning subroutine with typed input, output and error.
pub trait Subroutine {
    type Input;
    type Output;
    type Error;

    /// The task this subroutine implements.
    const TASK: ReasoningTask;

    /// Cheap check of the input before running.
    fn precondition(&self, input: &Self::Input) -> bool;

    /// Estimated work for `input`, compared against [`Limits::max_work`].
    fn estimate(&self, input: &Self::Input) -> u32;

    fn run(&self, input: &Self::Input, limits: Limits) -> Result<Self::Output, Self::Error>;
}

/// Why delegated execution failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExecutionError<E> {
    /// The subroutine does not implement the selected task.
    WrongSubroutine {
        selected: ReasoningTask,
        implements: ReasoningTask,
    },
    PreconditionFailed,
    LimitExceeded {
        estimate: u32,
        limit: u32,
    },
    /// The subroutine's own error.
    Subroutine(E),
}

/// Run `sub` for the `selected` task.
pub fn execute<S: Subroutine>(
    selected: ReasoningTask,
    sub: &S,
    input: &S::Input,
    limits: Limits,
) -> Result<S::Output, ExecutionError<S::Error>> {
    if S::TASK != selected {
        return Err(ExecutionError::WrongSubroutine {
            selected,
            implements: S::TASK,
        });
    }
    if !sub.precondition(input) {
        return Err(ExecutionError::PreconditionFailed);
    }
    let estimate = sub.estimate(input);
    if estimate > limits.max_work {
        return Err(ExecutionError::LimitExceeded {
            estimate,
            limit: limits.max_work,
        });
    }
    sub.run(input, limits).map_err(ExecutionError::Subroutine)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Halve;
    impl Subroutine for Halve {
        type Input = i64;
        type Output = i64;
        type Error = &'static str;
        const TASK: ReasoningTask = ReasoningTask::MathematicalCalculation;
        fn precondition(&self, x: &i64) -> bool {
            *x >= 0
        }
        fn estimate(&self, x: &i64) -> u32 {
            *x as u32
        }
        fn run(&self, x: &i64, _: Limits) -> Result<i64, &'static str> {
            if x % 2 == 0 {
                Ok(x / 2)
            } else {
                Err("odd")
            }
        }
    }

    #[test]
    fn checks_happen_in_order_and_errors_are_typed() {
        let l = Limits { max_work: 10 };
        let m = ReasoningTask::MathematicalCalculation;
        assert_eq!(execute(m, &Halve, &8, l), Ok(4));
        assert_eq!(
            execute(ReasoningTask::Pathfinding, &Halve, &8, l),
            Err(ExecutionError::WrongSubroutine {
                selected: ReasoningTask::Pathfinding,
                implements: m
            })
        );
        assert_eq!(
            execute(m, &Halve, &-2, l),
            Err(ExecutionError::PreconditionFailed)
        );
        assert_eq!(
            execute(m, &Halve, &12, l),
            Err(ExecutionError::LimitExceeded {
                estimate: 12,
                limit: 10
            })
        );
        assert_eq!(
            execute(m, &Halve, &3, l),
            Err(ExecutionError::Subroutine("odd"))
        );
    }
}
