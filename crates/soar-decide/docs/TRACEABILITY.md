# Source-to-Rust traceability

Upstream: <https://github.com/SoarGroup/Soar> at commit
`3fc337a1ab2d04b9e5183335c1496d7dcdc57c1c`, BSD.

Scope is item 1 of `docs/soar-survey.md`: the decision procedure,
exploration and the RNG.

| Upstream | Rust | Status |
|---|---|---|
| `shared/soar_rand.h`, `soar_rand.cpp` (`MTRand`, `SoarRand*`, `SoarSeedRNG`) | `src/rng.rs` | translated. Not translated: auto-seeding, `save`/`load`/stream operators, `rand53`, `randNorm`, `randExc`, `randDblExc` (unused by the decision procedure) |
| `shared/enums.h` preference types | `preference::PreferenceType` | translated |
| `shared/constants.h` impasse types, `USER_SELECT_*` | `semantics::ImpasseType`, `exploration::Policy` | translated |
| `soar_representation/preference.{h,cpp}` (`preference`, list insertion) | `preference::{Preference, Referent, Slot}` | translated (same-goal-level insertion; see DEVIATIONS) |
| `decide.cpp` decider flags | `semantics::Flags` | translated (S2) |
| `decide.cpp` `run_preference_semantics` | `semantics::run`, `run_preference_semantics`, `run_preference_semantics_read_only` | translated (RL/OSK hooks excluded) |
| `decide.cpp` `run_non_context_preference_semantics` | `semantics::run_non_context_preference_semantics` | translated |
| `decide.cpp` `decide_context_slot` | `decide::decide_context_slot` | decision logic translated; WM and goal-stack side effects excluded |
| `decision_manipulation.cpp` `select_force` | `force` argument of `run_preference_semantics` | translated (S6) |
| `exploration.cpp` policies, parameters, reduction, `exploration_update_parameters` | `exploration::Exploration` | translated (S3, S4) |
| `exploration.cpp` `exploration_choose_according_to_policy` | `Exploration::choose_according_to_policy` | translated (RL `rho` excluded) |
| `exploration.cpp` `exploration_probability_according_to_policy` | `Exploration::probability_according_to_policy` | translated |
| `exploration.cpp` `exploration_randomly_select`, `_probabilistically_select`, `_boltzmann_select`, `_epsilon_greedy_select`, `_get_highest_q_value_pref` | private functions of the same names in `exploration.rs` | translated |
| `exploration.cpp` `exploration_compute_value_of_candidate` | `Exploration::compute_value_of_candidate` | translated |
| `symbol.cpp` `get_number_from_symbol` | `Referent::number` | translated |
| `CLI/src/cli_preferences.cpp` selection-probability listing | used by the differential generator as the oracle | — |

**Tests.** Upstream has no unit tests for these routines; its test suite
runs whole agents, which needs the production system. The Rust unit tests
were written from the code. `tests/differential.rs` is the independent
check.
