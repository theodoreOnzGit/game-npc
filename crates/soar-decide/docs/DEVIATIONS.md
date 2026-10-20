# Deviations from upstream Soar

Upstream: <https://github.com/SoarGroup/Soar>, commit
`3fc337a1ab2d04b9e5183335c1496d7dcdc57c1c` (2026-10-07, kernel 9.6.5).

Preference semantics, the context-slot decision, the exploration policies
and the random number stream are preserved. `tests/differential.rs` checks
them against the real `soar` binary: candidate order, selection
probabilities, the selected operator (including random choices), and the
impasse type, attribute and items. Upstream defects and quirks are fixed
rather than reproduced (issue #4: "correct upstream defects rather than
preserving implementation quirks").

| # | Upstream | Port | Why | Test |
|---|---|---|---|---|
| S1 | One process-wide Mersenne Twister. Seeded from `/dev/urandom` or a hash of `time()`/`clock()` unless `srand` is given. | `SoarRng` is an owned value (one per agent or simulation), always seeded by the caller. | Agents no longer share, and perturb, one stream. `no_std` has no entropy source, and lockstep needs explicit seeds. | `rng::tests::*` |
| S2 | Decider flags are stored on the value symbols and survive between decisions. | A fresh flag table per call. | No shared mutable state. Upstream never reads a stale flag, so nothing observable changes. | differential |
| S3 | Parameter decay is applied unchecked. A linear decay (or exponential with rate 0) can drive temperature to 0, and Boltzmann then divides by zero. | A decay step that would make a parameter invalid is skipped (temperature must stay > 0). ε still floors at 0, which is valid. | Bug. | `exploration_test::linear_decay_floors_epsilon_and_never_zeroes_temperature` |
| S4 | With NaN numeric values: softmax and Boltzmann can return `NIL`, and the caller then dereferences it; ε-greedy's tie walk runs off the end of the list. | Softmax falls back to the last positive candidate, Boltzmann to the last candidate, greedy to the first. | Crash on bad data. | `exploration_test::nan_values_do_not_panic` |
| S5 | More than one winner in a context slot is a fatal error (`abort_with_fatal_error`). | `unreachable!`. Decide mode always resolves indifference to one winner. | Structural invariant. | — |
| S6 | `decide select O12` matches the forced operator by its printed id string. | Matches by value equality on the caller's own value type. | No symbol table. Same behaviour, including bypassing every other preference. | `semantics_test::forced_selection_bypasses_preferences_and_is_consumed` |
| S7 | Boltzmann uses the platform `exp` (glibc here). | `petir::real::exp` (musl's libm in pure Rust), reused from outram-park. | Bit-identical across platforms and WASM, as lockstep needs. It may differ from glibc in the last ulp. That could flip a choice only if a draw lands within an ulp of a cumulative boundary; none of the 3000 differential scenarios did. | differential (Boltzmann scenarios) |

## Not translated in this milestone

These are parts of the same upstream functions that depend on subsystems
outside this crate's scope (see `docs/soar-survey.md` at the repo root):

- **Reinforcement learning hooks**: `rl_update_for_one_candidate`,
  `build_rl_trace`, the `rl_rho` bookkeeping in every policy,
  `rl_perform_update`. These belong to the planned Soar-RL milestone and do
  not affect which operator is chosen.
- **Chunking hooks**: `add_OSK`, `update_proposal_OSK`, `add_to_OSK`.
- **Goal-stack and working-memory side effects** of a decision:
  `create_new_context`, impasse WMEs (`^item`, `^item-count`,
  `^non-numeric`), `remove_existing_context_and_descendents`, GDS,
  `wait-snc`. The port returns the decision; building the substate is the
  caller's job.
- **`decide predict`** (`predict_set`).
- **Goal-level ordering in `add_preference_to_slot`**: a preference from a
  deeper match goal is inserted after shallower ones. `Slot` offers
  head-insert (`assert_preference`, the same-level case) and append
  (`push`). There is no goal stack yet.
- `random-uniform` is translated and unit-tested, but cannot be
  differential-tested: Soar's CLI option for it is commented out.

## Known blind spots of the differential test

Two deliberate mutations did **not** fail it, because they only matter on a
measure-zero boundary:

- The `rand()` divisor (2³² − 1 vs 2³²) moves a draw by about 2⁻³².
  `rng::tests::rand_uses_closed_interval_divisor` pins it instead.
- Softmax's `selected <= sum` vs `<` differs only when a draw lands exactly
  on a cumulative sum.

These mutations did fail it: the scenario number in brackets is the one
that caught each.

- "conflicted still dominates" removed [6];
- candidate order reversed [13];
- binary-indifferent preferences dropped from candidate values [299];
- the best filter applied when no candidate is best [39].
