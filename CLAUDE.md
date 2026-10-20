# CLAUDE.md

Guidance for Claude Code (and other AI assistants) working in `game-npc`.

## Reuse before porting or writing (HARD RULE)

**Before porting an upstream project or writing new code, search for code
that already does the job, and reuse it.** Look in this order:

1. **This workspace.** The root crate `game-npc` (motivation taxonomy,
   appraisal, scoring, validation, decision traces) and every crate under
   `crates/` (currently `crates/goapai`, the verified GOAP planner). Grep
   for the domain noun (for example "preference", "planner", "rng",
   "graph"), not for the API name you intend to write.
2. **`outram-park-backend`**
   (<https://github.com/theodoreOnzGit/outram-park-backend>), where it applies.
   It is the maintainer's 48-crate Rust workspace, and several of its crates
   are general-purpose. Known useful starting points:
   - `petir` (also on crates.io): `no_std` core numerics ported from GSL,
     including roots, minimisation, integration, interpolation, ODE, linear
     algebra, special functions, `fast_exp` / `fast_log` / `fast_pow`, and
     the workspace RNG (`petir::rng`). Use it before adding numeric code or
     another RNG crate. Issue #2 already names it as a core dependency.
   - `raffles`: statistics, estimators and seeded distributions.
   - Its root `Cargo.toml` `[workspace.dependencies]` pins `burn`
     (`0.21.0`, `default-features = false`). Match that version when Burn
     comes in, unless there is a reason not to.
   - `docs/claude-md/members-and-dependencies.md` there has a one-line
     description of every crate. Read it rather than guessing from names.

   To read it in a cloud session, attach it (`add_repo`) and clone it with
   `--depth 1`. Its own `CLAUDE.md` governs only that repository, not this
   one.
3. **Only then port**, preferring a mature upstream implementation to
   writing from scratch. Only write new code when both fail.

**Say what you checked.** In the issue, the commit message or the
hand-off, record where you looked and why the existing code did or did not
fit. "Searched `crates/` and `outram-park-backend` (`petir`, `raffles`) for
X; nothing covers Y because Z" can be checked. Silence cannot.

## Conventions already established here

- **Lockstep determinism.** The intended host is a lockstep RTS engine (the
  Rust SAGE port), so simulation code must give bit-identical results on
  every platform:
  - basic IEEE `f32`/`f64` arithmetic only (no libm transcendentals, no FMA);
  - no hash-map iteration order;
  - ties broken by stable ids;
  - pin golden checksums in tests.
- **Ports.**
  - Pin the upstream commit.
  - Keep upstream copyright and licence notices (`NOTICE`, licence files,
    per-file attribution headers). The combined work is AGPL-3.0-only.
  - Write `docs/TRACEABILITY.md` and `docs/DEVIATIONS.md` in the crate.
  - Fix upstream quirks and bugs rather than reproducing them, and document
    each one with a test.
  - Verify with a differential test against the original program (see
    `crates/goapai/differential/`), and confirm that the test can fail.
- **`no_std` and WASM.**
  - Library crates are `no_std` (+ `alloc` where needed).
  - They must build for `wasm32-unknown-unknown`.
  - Tests also run as `wasm32-wasip1` under Node (`.cargo/config.toml`):
    `cargo test --release --workspace --target wasm32-wasip1`.
- **Release mode** for builds and tests (`--release`).
- **Honest numbers.** Say when a heuristic is uncalibrated or a check has not
  been run. Correct a stale claim where it lives, and say that it was
  corrected.
- **Issues** on `theodoreOnzGit/game-npc` track the work. Post progress
  there when asked, and do not close issues without the maintainer's say-so.
