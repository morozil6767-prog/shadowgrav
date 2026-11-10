# CHANGES 19: WS3 graded quadrature tier (opt-in)

No Spec text, tolerance, existing test, manifest or lockfile change. Default behaviour is bit-identical (digest `f2e108670c118a3d`).

* `Validation` gains `grade: f64` (0 = off). The four existing presets set `grade: 0.0`; the three struct literals in `bin/verify.rs` (2) and `bin/converge.rs` (1) gain `grade: 0.0`.
* `overlap.rs`: graded refinement rule (see `WS3_QUADRATURE.md`), never counted as a cap hit.
* New preset `Validation::graded()`; `sgrun --mode graded`; scenario `"quadrature": "graded"`.
* New tests (existing tests untouched): `oracle_lens::graded_mode_vs_sharp_lens_oracle`, `oracle_sky::screening_curve_graded_mode_matches_exact_oracle`, `oracle_sky::grading_adds_leaves_never_flags_cap_hits_and_reduces_error`.
* New data and figure: `report/data/ws3_graded.csv`, `report/make_ws3_figure.py`, `report/figures/ws3_convergence.png`.
