# goapai (Rust)

A Rust translation of [goapai](https://github.com/akmonengine/goapai), a Go
library for microlithic Goal-Oriented Action Planning (GOAP) with a forward
A* search. It is the planning subsystem of the game-npc intelligence engine
(issue #3). It is `no_std` + `alloc`, contains no `unsafe`, and builds for
native targets and browser WebAssembly.

**Status:** translation done and verified against upstream. Not yet wired
into the game-npc motivation core, PETIR, Burn, petgraph or SAGE: per issue
#3, integration comes after verification.

## What was checked

| Check | Result (2026-10-20) |
|---|---|
| Upstream unit tests, translated | all pass (94 tests, including those added in translation) |
| Differential test against Go goapai (3000 random scenarios: goal, action sequence, `f32` cost bits) | 3000 / 3000 identical |
| Same suite on `wasm32-wasip1` (Node 22 WASI) | all pass, 3000 / 3000 identical |
| `wasm32-unknown-unknown` (browser) build | compiles |
| Differential test can fail | yes: heap tie order, successor order and heuristic mutations each fail it |

Method and numbers are in the doc comment of `tests/differential.rs`.
`differential/regen.sh` rebuilds the fixture from upstream.

## Benchmark

Upstream's `BenchmarkGoapAI` scenario (3 repeatable actions, `max_depth`
15). Both implementations return the same 13-action plan. Measured on one
Intel Xeon @ 2.80 GHz container (4 vCPU), 2026-10-20:

| Implementation | Time per `GetPlan` | Bytes allocated per call | Allocations per call |
|---|---|---|---|
| Go goapai, Go 1.24.7 (`go test -bench -benchmem`, 3 runs) | 19.8–21.2 µs | 8288 | 166 |
| Rust, x86_64 release (`examples/bench.rs`, 2 runs) | 6.0–6.5 µs | 9605 | 42 |
| Rust, wasm32-wasip1 under Node 22 (2 runs) | 10.2–10.5 µs | 7089 | 42 |

These are single-machine numbers from a shared container, not a controlled
benchmark. Peak memory is bounded by the number of search nodes, each holding
a clone of the world. Choose `max_depth` with that in mind: the branching
factor raised to the depth bounds the node count.

```bash
cargo run --release -p goapai --example bench --features std
```

## Differences from upstream

The algorithm and its tie-breaking follow upstream. Some upstream quirks were
fixed instead of reproduced, for example the world hash missing newly created
keys, goal ties decided by random Go map order, and integer division by zero
panicking. See [`docs/DEVIATIONS.md`](docs/DEVIATIONS.md). The Go-to-Rust
symbol and file map is in [`docs/TRACEABILITY.md`](docs/TRACEABILITY.md).

## Licence

Translated from goapai, Copyright 2024 Agoune Ramine, Apache-2.0
([`LICENSE-APACHE-2.0.md`](LICENSE-APACHE-2.0.md), [`NOTICE`](NOTICE)). The
translation and modifications are distributed under AGPL-3.0-only
([`LICENSE`](LICENSE)). Each source file carries an attribution header.
