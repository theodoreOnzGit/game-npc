# soar-decide

A Rust translation of [Soar](https://github.com/SoarGroup/Soar)'s decision
procedure. It turns operator proposals and preferences into one selected
operator, or into a typed impasse:
- tie;
- conflict;
- constraint failure;
- no-change.

When candidates are mutually indifferent, it picks with Soar's exploration
policies: first, last, softmax, ε-greedy, Boltzmann and random-uniform.
Random choices use Soar's own Mersenne Twister. It is `no_std` + `alloc`,
contains no `unsafe`, and builds for browser WASM.

It is the symbolic half of the issue #4 selector, and of the literature
review's §5.2: symbolic applicability first, then preference. Require,
reject, prohibit, better/worse and best/worst decide which operators are
admissible. Numeric-indifferent values carry the utility that a learned
model (Soar-RL or Burn) would supply. A learned model therefore cannot make
an inadmissible operator win.

**Status:** translated and checked against the real `soar` binary. Not yet
connected to the selector or the motivation core. Soar-RL, chunking and the
production system are out of scope here (`docs/soar-survey.md`).

## What was checked (2026-10-20)

| Check | Result |
|---|---|
| Differential test against `soar` (kernel 9.6.5, commit 3fc337a), 3000 random scenarios | 3000 / 3000 identical |
| — selections | 1961, including 613 random choices among several candidates, all identical for the same `srand` seed |
| — impasses | 682 tie, 51 conflict, 39 constraint-failure, 267 state no-change; type, attribute and items identical |
| — read-only candidate order and selection probabilities | 1961 lists identical (probabilities to Soar's two printed decimals) |
| Can the differential test fail? | 4 of 6 deliberate mutations caught; the 2 survivors are measure-zero boundaries, listed in DEVIATIONS |
| Unit tests | 39, plus the doc example |
| `wasm32-unknown-unknown` build | compiles |
| Tests on `wasm32-wasip1` (Node WASI) | see the root README command |

Method and scenario mix: `tests/differential.rs`. To regenerate the fixture
(needs a Soar build):

```bash
SOAR_BIN=/path/to/soar cargo run --release -p soar-decide --example gen_differential --features std
```

## Licence

Translated from Soar: Copyright 1995–2012 Carnegie Mellon University, The
Regents of the University of Michigan, and USC/ISI, under a BSD licence
([`LICENSE-SOAR-BSD.md`](LICENSE-SOAR-BSD.md)). The RNG is from Wagner's
MersenneTwister.h (BSD, notice in `src/rng.rs`). See [`NOTICE`](NOTICE).
The translation and modifications are AGPL-3.0-only ([`LICENSE`](LICENSE)).
Deviations: [`docs/DEVIATIONS.md`](docs/DEVIATIONS.md). Mapping:
[`docs/TRACEABILITY.md`](docs/TRACEABILITY.md).
