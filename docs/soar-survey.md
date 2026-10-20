# Soar survey: what is worth translating (issue #4)

Upstream: <https://github.com/SoarGroup/Soar>, commit
`3fc337a1ab2d04b9e5183335c1496d7dcdc57c1c` (2026-10-07). BSD licence,
Copyright 1995–2012 Carnegie Mellon University, The Regents of the
University of Michigan, and USC/ISI.

Line counts are raw `wc -l` over `.c/.cpp/.h/.hpp/.cxx`, comments and blanks
included. They were measured 2026-10-20.

## Size

| Part | Lines | Notes |
|---|---:|---|
| Whole repository (C/C++) | 583,167 | |
| — vendored SQLite (`SoarKernel/sqlite` + `sqlite3.h`) | 256,407 | third-party |
| — vendored Eigen + libccd (SVS) | 148,571 | third-party |
| **Soar's own code** | **178,189** | the rest of this table |
| Kernel (`SoarKernel/src`, without `sqlite3.h`) | 83,934 | the cognitive architecture |
| SML (client, kernel, connection, ElementXML, shared) | 44,146 | messaging and embedding API |
| CLI | 22,040 | command interpreter |
| SVS (own code) | 15,756 | spatial and visual system |
| Unit tests | 10,466 | plus 84 test agents and directories |
| Tcl, performance tests, SoarCLI front end | 1,759 | |

Soar builds here: CMake, `SVS=OFF`, g++ 13.3, about 5 minutes. A working
`soar` binary is therefore available as a **reference oracle** for
differential tests, as goapai's Go build was for #3.

## Kernel subsystems, ranked by usefulness to the NPC engine

"Fit" means how well the subsystem serves this engine: deterministic,
`no_std`/WASM, a 4 GB target, and NPC behaviour.

| Subsystem | Lines | What it does | Useful for | Fit | Porting cost |
|---|---:|---|---|---|---|
| **Decision procedure**: `decide.cpp` preference semantics, slots, impasses, operator selection | ~5.1k (`decide.*` 4,153, `slot.cpp` 208, `preference.*` 749) | Turns operator proposals and preferences (acceptable, reject, require, prohibit, better/worse, best/worst, indifferent, numeric) into one selected operator, or into an **impasse** (tie, conflict, constraint-failure, no-change) that opens a substate | **The selector core of #4**: "select exactly one", consistent tie-breaking, explicit failure, typed impasses. | excellent: pure logic | low–medium |
| **Exploration policies** (`exploration.*`) + Soar RNG (`soar_rand.h`) | 1,807 | Chooses among indifferent candidates: first, last, softmax, epsilon-greedy, Boltzmann, with parameter decay | Variety in NPC choices. A seeded RNG keeps it lockstep-safe. | excellent | low |
| **Reinforcement learning** (`reinforcement_learning.cpp/.h`) | 1,403 | SARSA / Q-learning on numeric-indifferent preferences, eligibility traces, hierarchical RL | NPCs that learn preferences from outcomes, **without a neural net**. A cheap deterministic alternative or baseline for the Burn selector (#4 phase 3). | very good | low–medium |
| **Working-memory activation** (`working_memory_activation.*`) | 1,693 | Base-level activation and decay, forgetting | Memory realism: NPCs forget unused facts. | good | low |
| **Production system**: Rete matcher, productions, conditions, tests, RHS, instantiations, working memory, symbols, reordering | ~23k (`rete.*` 10,140, `production_reorder` 1,488, `instantiation` 1,601, `test` 1,055, `rhs*` 4,677, `symbol*` 1,946, `working_memory` 566, …) | Matches IF-THEN rules against working memory incrementally; fires rules that propose operators and preferences | Designer-authored behaviour rules in Soar syntax; reuse of Soar's agents and tests. **Not needed** for a selector over a finite vocabulary of subroutines. | good, but large | **high** |
| **Parser / lexer** | 3,715 | Soar production syntax | Required with the Rete matcher to run Soar's own test agents for verification. | good | medium |
| **Decision cycle** (`run_soar.cpp`, `consistency.cpp`) | 2,656 | Phase loop (input, propose, decide, apply, output), o-support and i-support, consistency checking | Needed once the production system is in. | good | medium |
| **Episodic memory** | 6,705 | Records and recalls past working-memory snapshots | NPCs that remember *events* ("the player robbed me"). Very relevant to believability. | **poor as is**: stored in SQLite. An in-memory backend would be needed for `no_std`/WASM. | high |
| **Semantic memory** | 8,798 | Long-term fact store with spreading activation | Persistent world knowledge per NPC. Overlaps petgraph. | poor as is (SQLite) | high |
| **Chunking (EBC) + explanation memory** | 10,037 | Learns new rules from substate reasoning | "Learned reasoning shortcuts" (#4 phase 5). Depends on the whole production system. | medium | very high |
| Interface: io-link, callbacks | 2,275 | Input and output links between agent and environment | The *pattern* is useful (how the SAGE adapter could feed perception), the code less so. | — | low |
| Output, trace, print, XML, memory pools, settings, stats | ~13.5k | Infrastructure | Rust replaces it with its own types. | n/a | skip |

## Outside the kernel

| Part | Lines | Useful? |
|---|---:|---|
| SVS: scene graph, geometric filters (intersect, distance, volume), commands | 15.7k (+ Eigen, libccd) | Low. Overlaps SAGE's spatial and pathfinding code. Possibly later as a perception layer. |
| SML (embedding / messaging API) | 44k | No. It exists for embedding Soar in Java/Python/Tcl; our engine is Rust-native. |
| CLI | 22k | No, except as a reference for the debugging and inspection commands to mirror later (MCP inspection). |
| Unit tests + 84 test agents (blocks world, eight-puzzle, Towers of Hanoi, water jug, chunking cases) | 10.5k + agents | **Yes, as a verification corpus**, once the parser and Rete matcher are ported. |

## Recommended scope (selective translation, option B)

Exhaustive translation of all 178k lines is not a good use of effort: 66k
are SML/CLI glue, and EpMem and SMem are built around SQLite. Within the
kernel, value is concentrated in a small core. In order:

1. **Decision procedure + exploration + RNG** (~7k C++). This is the core of
   #4's selector, and it can stand alone: proposals and preferences come in
   through a typed interface rather than from Rete. Verified differentially
   against the built `soar` binary, using agents whose productions emit
   fixed preference sets.
2. **Reinforcement learning** (~1.4k). Learned selection preferences, as the
   deterministic counterpart to the Burn selector.
3. **Working-memory activation** (~1.7k).
4. **Production system: parser, Rete, decision cycle** (~29k). Only if
   designer-authored rules in Soar syntax are wanted. It is also what lets
   Soar's own test agents run against the port.
5. **Episodic memory** with an in-memory backend. Highest NPC-believability
   payoff of the memory systems, but the SQLite coupling makes it a
   redesign, not a translation.
6. Chunking, semantic memory, SVS: later, or not at all.

Each selected subsystem is translated exhaustively, with upstream defects
fixed and documented, as for goapai (#3).

## Licensing

BSD-style: keep the copyright notice and disclaimer in the source and in
distributions. This is compatible with AGPL-3.0-only for the combined work,
handled as goapai's Apache and Go-BSD notices were.
