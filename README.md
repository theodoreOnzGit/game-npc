# game-npc

An NPC intelligence engine core for games. It decides what a character wants
from a small, literature-grounded vocabulary of motivations, and turns those
wants into actions through deterministic, validated steps.

**Status: v0.1 deterministic baseline, plus a verified GOAP planner.** The
motivation taxonomy, state and personality model, deterministic scoring, goal generation, action validation
and replayable decision traces are implemented and tested. The neural
motivation model (Burn), the PETIR and petgraph integrations, and the SAGE
engine bridge are **not implemented yet**. See issues #1 and #2. The GOAP
planner (issue #3) is translated and verified but not yet connected to the
motivation core.

Licence: AGPL-3.0-only.

## Pipeline

```text
World event + personality
  -> appraise()            deterministic appraisal       (appraisal.rs)
  -> CharacterState        resources + emotion           (state.rs)
  -> MotivationModel       deterministic baseline today  (model.rs)
  -> MotivationVector      25 x f32 = 100 bytes          (vector.rs)
  -> generate_goal/plan    symbolic goal -> one action   (goal.rs)
  -> validate()            preconditions vs world        (action.rs)
  -> WorldState::apply     only accepts ValidatedAction  (world.rs)
  -> DecisionTrace         checksum + bit-exact replay   (trace.rs)
  -> feedback event -> appraise() ...
```

- **Distinct types.** Motivation, goal, action, emotion, personality and
  resource are each their own type.
- **Many motivations at once.** Conflicting wants coexist in the vector. The
  planner walks them from strongest to weakest and takes the first one whose
  action is feasible, recording every rejected attempt.
- **Models cannot touch the world.** `MotivationModel::predict` returns only a
  vector. `ValidatedAction` has a private field, so `validate` is the only way
  to build one.
- **Replaceable model.** `decide` is generic over `MotivationModel`, so a
  neural model can replace the baseline without changing the planner.

## Lockstep determinism

The intended host is a lockstep RTS engine (the Rust SAGE port), so every
peer must compute bit-identical results. The core therefore:

- uses only IEEE-754 `f32` `+ - *`, comparisons, `max` and `clamp`: no
  `exp`, `sqrt` or other libm functions, and no fused multiply-add;
- uses integers for world state;
- iterates in fixed orders only (no hash maps) and breaks ranking ties by
  stable id;
- is `no_std`, allocation-free and `#![forbid(unsafe_code)]`;
- emits a 64-bit FNV-1a checksum per decision, for desync detection.

`tests/scenarios.rs` pins a golden checksum, recorded on
x86_64-unknown-linux-gnu and reproduced bit-for-bit on wasm32-wasip1 (Node 22
WASI) on 2026-10-20. (**Corrected 2026-10-20:** an earlier version of this
README said it had not yet been run on a second platform.)

## Workspace

| Crate | What it is |
|---|---|
| `game-npc` (root) | motivation taxonomy and deterministic decision core (issue #1) |
| [`crates/goapai`](crates/goapai/README.md) | Rust translation of the goapai GOAP planner, verified against the Go original (issue #3) |
| [`crates/soar-decide`](crates/soar-decide/README.md) | Rust translation of Soar's decision procedure (preference semantics, impasses, exploration policies, RNG), verified against the `soar` binary (issue #4) |
| [`crates/selector`](crates/selector/README.md) | Issue #4 subroutine selector: one routine per request through Soar preferences, typed errors, replayable traces, pluggable utility model |

## Build and test

```bash
cargo test --release --workspace
cargo clippy --release --workspace --all-targets -- -D warnings
cargo build --release --workspace --target wasm32-unknown-unknown
# Run the same tests as WebAssembly (needs Node >= 20 and the target:
# rustup target add wasm32-wasip1). The runner is set in .cargo/config.toml.
cargo test --release --workspace --target wasm32-wasip1
```

## Taxonomy

See [`docs/motivation-taxonomy.md`](docs/motivation-taxonomy.md) for
definitions, candidate literature correspondences, and the open review
questions: the 24-vs-25 count, overlaps, and omissions. The review against
Talevich et al. (2017) has not been done yet.
