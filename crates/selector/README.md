# selector

The issue #4 subroutine selector. It takes a structured reasoning request,
picks **exactly one** subroutine (epistemic evaluation, mathematical
calculation, graph analysis, goal planning or pathfinding), and returns its
id or a typed error. It never runs the subroutine; execution goes through a
separate typed interface. `no_std` + `alloc`, no `unsafe`.

## How it decides

The selector runs Soar's decision procedure (`crates/soar-decide`, verified
against the real `soar` binary):

1. **Symbolic applicability.** A registered routine becomes a candidate
   (a Soar acceptable preference) only if all of these hold:
   - its input kind matches the request;
   - its output kind matches, if the request names one;
   - its precondition holds;
   - its estimated cost fits the budget.
2. **Request constraints.** `require`, `prohibit` and `prefer` become Soar
   require, prohibit and better preferences.
3. **Preference.**
   - `DeterministicSelector`: cheaper beats dearer, and equal cost goes to
     the lowest stable id. It draws no random numbers.
   - `UtilitySelector<M: UtilityModel>`: the model's utilities become
     numeric-indifferent preferences and the highest wins. Exact ties are
     broken by Soar's RNG from a fixed seed. This is the slot for a Burn
     model (#4 phase 3).
4. **Outcome.**
   - one routine;
   - `NoApplicableRoutine` (no candidates);
   - `AmbiguousSelection` (Soar tie, conflict or constraint-failure);
   - `InvalidInput` (`require` names an inapplicable routine, or the model
     returns a non-finite utility).

A utility model only scores routines that are already applicable, so it
cannot select an incompatible one. `SelectionTrace` records:
- each routine's applicability verdict;
- the utilities and the seed;
- any impasse, and the result.

`replay` re-checks a trace.

## Acceptance criteria from #4

| Criterion | Where |
|---|---|
| One subroutine per request | `SubroutineSelector::select` returns one `ReasoningTask` |
| Incompatible subroutines cannot be selected | applicability gate; property test `selected_routine_is_always_admissible` (3000 random requests) |
| Selection and execution are separate | `select.rs` vs `execute.rs`; adapters live in `tests/delegation.rs` |
| Deterministic selection passes unit tests | `src/select_test.rs` |
| Neural selection is independently replaceable | `UtilityModel` trait; Burn model not built yet |
| Typed results and errors | `SelectionError`, `ExecutionError<E>` |
| Reproducible traces | `SelectionTrace`, `replay` |
| Native and WASM builds | `cargo build --release --target wasm32-unknown-unknown`; tests also pass on wasm32-wasip1 |
| Works without SAGE | no SAGE dependency |

**Delegation demo** (`tests/delegation.rs`): goal planning runs on
`crates/goapai`, and root finding on PETIR's Brent solver (GSL port).
Neither is a dependency of the selector.

**Learned baseline without a neural net:** `ActrUtility` uses ACT-R's utility-learning rule `U ← U + α(R − U)` as a `UtilityModel`. It is deterministic and is the baseline a Burn model must beat (see `docs/oltramari-2023-critique.md`).

**Not done:**
- the Burn model and its training data (#4 phase 3);
- petgraph and SAGE adapters;
- Soar-RL as a learned-preference baseline.
