# Performance phase: what changed, how to measure, what to expect

## Changes (all in shadowgrav-core; the force core is still a pure function of its inputs)
1. **Rayon over targets** (`force.rs`: `Shared` read-only context, `eval_target`, `run_targets`). No interior mutability:
   each target builds its own scratch, results are collected in index order. Parallel == serial **bitwise**
   (`tests/perf_equiv.rs::parallel_hash_and_brute_are_bitwise_identical`). Cargo feature `parallel` (default on);
   `Options.parallel` switches it at run time. Thread count: `RAYON_NUM_THREADS`.
2. **Spatial-hash candidate graph** (`overlap.rs`: `candidates_with`, `mark_grid`). Exact equivalent of the O(m^2)
   reference (`candidates_brute`), O(m log m) per target. Discs with theta <= 0.3 go into a cubic grid of cell
   2 theta_max on the unit vectors (neighbour cells suffice because chord <= psi < theta_p + theta_q <= cell size);
   the few larger discs are tested against everything with the same predicate. Handles V2 shifted discs. Python
   model: 0 mismatches in 40 random trials incl. injected huge/tiny discs; Rust test
   `hash_candidates_equal_brute` repeats it. `Options.brute_candidates` selects the reference path.
3. **Thin-fast preview** (`thin_fast.rs`): Barnes-Hut monopole octree, uniform-sphere pair field, no overlap correction,
   O(N log N), rayon over targets. Gated (V0, alpha_bg = 0, all tau_c < 1e-2, opening angle <= 0.5), flagged in
   `Status::thin_fast`, red badge + "thin-fast: NON-CONFORMING" label in the UI. Python model of the same tree at
   N = 400: vector-norm error 5e-3 (theta 0.5), 1e-3 (theta 0.3); worst single target 13% (near-cancelling forces).
   Test `thin_fast_matches_conforming_within_barnes_hut_error` asserts < 2e-2.
4. UI: "parallel" and "thin-fast" checkboxes, "Dense random N" preset (N slider up to 2000), bench binary.

## Complexity per force evaluation (N bodies, m = N-1 occluders per target)
| stage | before | after |
|---|---|---|
| pair stage | N^2 closed forms, 1 thread | N^2 closed forms, / cores |
| candidate graph | N * m^2 = O(N^3) angle tests | N * m log m = O(N^2 log N), / cores |
| overlap HEALPix correction | sum over targets of (leaves x touching set) | unchanged work, / cores |
| thin-fast | n/a | O(N log N), / cores (non-conforming) |

## Measure (I could not run Rust in my sandbox, so there are NO measured timings in this note)
    cargo run --release -p shadowgrav-core --bin bench -- 50 100 200 400
    cargo run --release -p shadowgrav-core --bin bench -- 800 1600 --slow-max 100     (skips the slow serial+brute rows)
    cargo run --release -p shadowgrav-core --bin bench -- 400 --tau 1e-1               (heavier overlap)
Table to fill (ms per force evaluation; `bitwise=` must read true on every conforming row):

| N | serial+brute (old) | serial+hash | parallel+brute | parallel+hash (new) | thin-fast |
|---|---|---|---|---|---|
| 50 | | | | | |
| 100 | | | | | |
| 200 | | | | | |
| 400 | | | | | |

## What to expect (reasoning, not measurement)
* The hash removes the O(N^3) term; it matters most when the graph, not the HEALPix traversal, dominates (large N, mostly
  disjoint discs, e.g. N >= 1000 at small a).
* In overlap-heavy scenes the HEALPix correction dominates and is untouched algorithmically: the gain there is the
  core count (near-linear when many targets carry similar work; a single very heavy target bounds the speed-up).
* Conforming interactive N therefore improves mainly by the core count; the large-N envelope is thin-fast.

## New limitations / behaviour changes
* On error, all targets are now evaluated before the first error (in target order) is returned (was: early abort).
  The reported error is deterministic and identical to the serial one.
* Quad tables are still rebuilt per call (cheap vs. the rest); hash threshold is m >= 64 (below that the reference is used).
* Thin-fast violates Newton's third law at the BH error level (R_P > 0) and has no overlap correction or saturation of
  occluders beyond the m^g factor; never use it for conformance runs.
* The brute tier (discs with theta > 0.3) is O(big x m): scenes with many very close spheres lose the hash advantage.
