# Deviations from upstream goapai

Upstream: <https://github.com/akmonengine/goapai>, commit
`2e86c5ee56d6d10f43ed563214b1aa86be83db2d` (tag v1.2.0, 2026-09-18).

The planning algorithm is preserved. That covers successor order, the A*
open-set handling, the `container/heap` tie order, the cost and heuristic
arithmetic, the depth limit, and the root action at the head of the plan.
`tests/differential.rs` checks this against upstream on 3000 scenarios.

Following maintainer direction on issue #3 (2026-10-20: "no need to replicate
quirks exactly"), the upstream quirks and bugs listed below were fixed rather
than reproduced. Each has a unit test. The differential generator avoids
these inputs, because on them the port is meant to differ.

| # | Upstream behaviour | Port behaviour | Why | Test |
|---|---|---|---|---|
| D1 | An effect that creates a key absent from the world appends the state **without updating the world hash**, so two worlds that differ only in that key can be merged as the same search node. | The new state goes through `World::store`, so the hash covers it. | Bug: it can drop distinct states from the search. | `astar_test::worlds_differing_by_new_key_are_distinct`, `state_test::world_hash_is_xor_of_states` |
| D2 | `GetPlan(agent Agent, …)` takes the agent by value but writes cached hashes (and, with duplicate keys, *values*) back into the caller's state slice through Go slice aliasing. | `get_plan(&Agent, …)` leaves the agent untouched. | Hidden mutation of the caller's state. | `planer_test::get_plan_leaves_agent_unchanged` |
| D3 | A `ConditionFn` is evaluated once and **cached forever**, across all later `GetPlan` calls. The README says it is "resolved once per planning request". | The cache is reset at the start of every planning request. | Matches upstream's documented intent; a stale sensor read would otherwise never update. | `planer_test::condition_fn_is_resolved_once_per_request` |
| D4 | `SetState` always appends, so setting a key twice creates duplicates. Its doc says "adds or updates". | `set_state` updates the existing key. | Matches upstream's documented intent. | `state_test::set_state_updates_existing_key` |
| D5 | Goals live in a Go map, so on a **priority tie** the selected goal depends on random map iteration order. | `Goals` keeps insertion order; on a tie the first-inserted goal wins. | Lockstep simulation needs determinism. This is one of the outcomes upstream could produce. | `planer_test::goal_priority_ties_are_deterministic` |
| D6 | `EffectString.check` ignores the operator, so `ADD "x"` on a state holding `"x"` counts as already satisfied: the action is treated as a no-op and dropped. | Only `Set` can be a no-op, as for the numeric and bool effects. | Bug: appending to a string always changes it. | `effect_test::string_add_is_never_a_no_op` |
| D7 | Integer `DIVIDE` by zero panics (Go runtime). | Returns `GoapError::DivisionByZero`, so the action is not applicable in that world. Float division still follows IEEE-754. | A planner should not crash the game on bad data. | `effect_test::integer_division_by_zero_is_an_error` |
| D8 | `GetPlan` prints `no goal available` to stdout and returns `("", Plan{})`. | `get_plan` returns `("", empty)` without printing. `try_get_plan` returns `Err(GoapError::NoGoalAvailable)`. | `no_std`, and printing is the caller's choice. | `planer_test::try_get_plan_reports_no_goal` |
| D9 | `world` holds a pointer to its `Agent`, which `ConditionFn` uses to reach the sensors. | Sensors are passed to condition checks explicitly. | No self-referential pointers in Rust; no observable difference. | (all `ConditionFn` tests) |
| D10 | `operator` and `arithmetic` are `uint8`, so out-of-range values such as `operator(7)` compile, and then always fail or do nothing. | Rust enums: invalid operators cannot be built. | Removes a silent misconfiguration. | (compile time) |
| D11 | `maxDepth int` is converted with `uint16(maxDepth)`: −1 becomes 65535. Node depth is `uint16` and can wrap. | `max_depth: u16`; node depth is `u32`. | The type states the valid range; no wrap. | `astar_test::astar_depth_limit_is_inclusive` |
| D12 | Dead code: a "closed" flag and a reopen branch that can never run (closed nodes are no longer in the heap that is searched), and `heap.Fix(h, -1)` (a no-op) after discarding a too-deep node. | Removed. | No behaviour depends on it. | differential test |
| D13 | `computeHeuristic(from, goal, w)` takes an unused `from` argument. | Dropped. | Unused. | — |

## Platform and type adaptations

- **Go `int` becomes `i64`.** Go's `int` is 64-bit on amd64, arm64 and wasm,
  which are the targets in scope. On 32-bit Go platforms `int` is 32-bit,
  and that case is not modelled.
- **Integer arithmetic wraps** (`wrapping_*`), as Go's does. Unary minus on an
  unsigned type wraps too (`-5` as `uint8` is 251).
- **Go type parameters on custom named types** (`~int8` and so on) have no
  counterpart: Rust states use the seven concrete types.
- **Custom state, condition and effect types.** The upstream README offers
  them "through interface". In practice those interfaces cannot be
  implemented outside the package, because their methods take the unexported
  `world` type. The Rust port uses closed enums, so there is no
  functionality loss.
- **Sensors** are `Arc<dyn Any + Send + Sync>`, keyed by name in a
  `BTreeMap`. Go's are a `map[string]any`.
- **`no_std` + `alloc`.** Nothing in the planner needs `std`; the `std`
  feature only enables the benchmark example.

## Not translated

- `benchmark/kelindargoap_test.go`: a benchmark of a different third-party
  library (`kelindar/goap`), used upstream for comparison. It is not part of
  goapai.
- `.github/` workflows (Go CI, lint, coverage), `CONTRIBUTING.md`.
