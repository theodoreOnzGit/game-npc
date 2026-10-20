# Source-to-Rust traceability

Upstream: <https://github.com/akmonengine/goapai> at commit
`2e86c5ee56d6d10f43ed563214b1aa86be83db2d` (tag v1.2.0, committed
2026-09-18, `go 1.24.0`), Apache-2.0, Copyright 2024 Agoune Ramine.

## Source inventory

| Upstream file | Lines | Rust file | Status |
|---|---|---|---|
| `state.go` | 384 | `src/state.rs` | translated |
| `action.go` | 283 | `src/action.rs` | translated |
| `agent.go` | 128 | `src/agent.rs` | translated |
| `astar.go` | 226 | `src/astar.rs` | translated |
| `distance.go` | 112 | `src/distance.rs` | translated |
| `heap.go` | 32 | `src/heap.rs` | translated, together with Go's `container/heap` algorithm |
| `planer.go` | 60 | `src/planer.rs` | translated |
| `state_test.go` | 376 | `src/state_test.rs` | translated |
| `action_test.go` | 167 | `src/action_test.rs` | translated |
| `effect_test.go` | 345 | `src/effect_test.rs` | translated |
| `agent_test.go` | 190 | `src/agent_test.rs` | translated |
| `astar_test.go` | 468 | `src/astar_test.rs` | translated |
| `distance_test.go` | 216 | `src/distance_test.rs` | translated |
| `heap_test.go` | 213 | `src/heap_test.rs` | translated |
| `planer_test.go` | 362 | `src/planer_test.rs` | translated |
| `benchmark/goapai_test.go` | 91 | `examples/bench.rs` | translated |
| `benchmark/kelindargoap_test.go` | 54 | — | not applicable (benchmarks another library) |
| `README.md` usage example | — | `src/lib.rs` doc test | translated |
| `LICENCE.md`, `NOTICE.md` | — | `LICENSE-APACHE-2.0.md`, `NOTICE` | carried over |
| `go.mod`, `.gitignore`, `.github/`, `CONTRIBUTING.md` | — | — | Go tooling, not applicable |

Upstream tests pass on Go 1.24.7 (`go test ./...`, 2026-10-20). Every
translated upstream test passes in Rust. Tests added in the translation sit
below a `Tests added in the translation` marker in each file.

## Symbol map

| Go | Rust |
|---|---|
| `operator`, `EQUAL` … `UPPER` | `Operator::{Equal, NotEqual, LowerOrEqual, Lower, UpperOrEqual, Upper}` |
| `arithmetic`, `SET` `ADD` `SUBSTRACT` `MULTIPLY` `DIVIDE` | `Arithmetic::{Set, Add, Subtract, Multiply, Divide}` |
| `Numeric` | `trait Numeric` for `i8`, `i64`, `u8`, `u64`, `f64` |
| `StateInterface` | `State` (one struct; the value type is in `Value`) |
| `State[T]` | `State { key, value: Value }`; `Value::{Int8, Int, Uint8, Uint64, Float64, Bool, String}` |
| `State.GetKey/GetValue/GetHash/Hash/Check/Store/Distance` | `State::get_key/get_value/get_hash/hash/check/store/distance` |
| `StateKey` | `StateKey = u16` |
| `states`, `states.GetIndex` | `World::states()`, `World::get_index` (returns `Option`) |
| `world`, `world.Check` | `World`, `World::check` |
| `updateHashIncremental` | inside `World::store` |
| `Sensor`, `Sensors`, `Sensors.GetSensor` | `Sensor`, `Sensors`, `Sensors::get_sensor` (plus typed `get`) |
| `ConditionInterface` | `enum ConditionInterface` |
| `Condition[T]` | `Condition<T: Numeric>` |
| `ConditionBool`, `ConditionString` | same names |
| `ConditionFn` | `ConditionFn` (shared via `Arc`, atomic cache) |
| `Conditions`, `Conditions.Check` | `Conditions = Vec<ConditionInterface>`, `check_conditions` |
| `Action`, `Actions`, `Actions.AddAction` | `Action`, `Actions`, `Actions::add_action` |
| `Action.GetName`, `Action.GetEffects` | `Action::get_name`, `Action::get_effects` (+ `get_cost`, `is_repeatable`, `get_conditions`) |
| `EffectInterface` | `enum EffectInterface` |
| `Effect[T]`, `EffectBool`, `EffectString` | same names |
| `Effect*.check`, `Effect*.apply` | `check`, `apply` (crate-private, as upstream) |
| `EffectFn` | `EffectFn` (unused upstream as well) |
| `Effects`, `Effects.satisfyStates`, `Effects.apply` | `Effects = Vec<EffectInterface>`, `satisfy_states`, `apply_effects` |
| `Agent` | `Agent` |
| `goalInterface` | `GoalInterface` |
| `GoalName`, `Goals` | `GoalName = String`, `Goals` (insertion-ordered) |
| `GoalPriorityFn` | `GoalPriorityFn` |
| `CreateAgent`, `SetState`, `SetSensor` | `create_agent`, `set_state`, `set_sensor` |
| `Plan`, `Plan.GetTotalCost` | `Plan`, `Plan::get_total_cost` (+ `steps`) |
| `GetPlan` | `get_plan` (+ `try_get_plan`) |
| `Agent.getPrioritizedGoalName` | `Agent::get_prioritized_goal_name` |
| `node` | `astar::Node` (arena-indexed) |
| `astar` | `astar::astar` |
| `getImpactingActions`, `fetchNodeInHeap`, `buildPlanFromNode`, `simulateActionState`, `allowedRepetition`, `countMissingGoal`, `computeHeuristic` | snake_case functions of the same names in `astar.rs` |
| `calculateNumericDistance` | `distance::calculate_numeric_distance` |
| `nodeHeap` + `container/heap` `Init/Push/Pop/Fix/up/down` | `heap::NodeHeap` (`push`, `pop`, `fix`, `up`, `down`, `push_raw`, `pop_raw`) |

## Test map

Each upstream `TestXxx` maps to a Rust test with the snake_case name in the
matching `*_test.rs` file. Table-driven subtests become loops over case
tables, and failure messages carry the upstream subtest name.
