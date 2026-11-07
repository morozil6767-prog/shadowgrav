# shadowgrav-ui: design note

## Goal
A native research instrument on top of `shadowgrav-core`. Correctness and responsiveness first, visuals second.

## Hard rules (Architecture section 2)
1. The physics core stays a pure function; this crate never modifies it and never calls `evaluate_forces` itself.
2. The UI thread has no access to `World`. It exchanges only `protocol::Command` (UI -> sim) and
   `Arc<protocol::FrameSnapshot>` (sim -> UI).
3. No physics in the UI layer. `view3d.rs` rotates/projects/colours numbers; `app.rs` formats numbers. Every
   diagnostic is computed by `shadowgrav_core::diagnostics` inside the sim thread.

## Threads
```
 UI thread (eframe)                          sim thread ("sim")
 ------------------                          ------------------
 egui update()  --Command (mpsc)-->          drain commands (between steps only)
                                             step loop: advance(&mut World)  [8 ms budget per pass]
                <--Arc<FrameSnapshot>--      publish (latest-wins Mutex<Option<Arc<..>>>) at <= 60 Hz,
                    + ctx.request_repaint()  slow diagnostics rebuilt at <= 10 Hz and shared by Arc
```
* Stepping is paced by `SetRate` (steps/s) or "max" (as many steps as fit in 8 ms per pass).
* A long force evaluation (dense overlap, validation mode) blocks only the sim thread; the UI keeps rendering the
  last snapshot. Commands are applied at the next step boundary (latency = one step).
* Any engine error pauses the run, shows in the status bar and turns the badge red. `advance` and the live-edit
  methods are atomic/rolled back in the core, so state is never half-updated.

## Snapshot format (`protocol.rs`)
* `FrameSnapshot` (fast, every publish): time, step, running, rate, steps/s, `bodies: Vec<BodyView>` (x, v, radius, tau_c,
  S, probe flag, and the five force buffers f_total/f_pair/f_corr/f_dr/f_ab), `ParamView`, badge + reasons, error.
* `DiagView` (slow, `Arc` shared): conservation ledger (E, dE/E0, dP, dL, R_P with running maxima), per-target
  Delta^NA / Delta^sat / R^N / R^thin / gamma, pair G_eff vs expected, quadrature health (leaves, cap hits,
  max|F_corr|/|F_sh|), eq. 12 iterations/residual, tau_c and S statistics, validity flags, Contract labels,
  and preset-specific read-outs (orbit omega/T, screening R and |R-0.8|, drag gamma and u/u_expected-1).
* Positions are f64 end to end; the view converts to f32 only after subtracting the centre (floating origin).

## Conformance badge (Architecture section 7)
* RED: engine error, cap-hit, eq. 12 not converged, V2 gating violated.
* AMBER: adaptive h, uncalibrated Pi, energy "diagnostic, non-conservative" label (always in V1/V2), V2 selected.
* GREEN: none of the above. Hover the badge for the reasons.

## Rendering and current interaction surface

The 3D view is a CPU perspective projection drawn with the egui painter. It supports point probes, body selection,
trail history, axis views, fit/reset, force-component arrows, hover diagnostics and a visible conformance stamp. Arrow
vectors use the differential of the same perspective projection as body positions, including foreshortening; the
common-scale mode preserves force-magnitude comparisons while per-component mode emphasizes direction. Trails are
split when points cross the camera plane instead of connecting across hidden sections.

The sky panel provides Mollweide all-sky and azimuthal-equidistant disc zoom views, deficit/non-additive modes, disc
silhouettes, map statistics and force-direction overlays. It is a diagnostic raster only; it never feeds force evaluation.
The renderer remains CPU/egui and its interactive performance must be measured on representative hardware before
publishing an N-body throughput claim.

## Deferred

Arbitrary scene editing/initial-condition import, a full run recorder, a wgpu instanced renderer, GPU compute backends,
and a V2 shifted-disc overlay on the sky map remain future work. Snapshot export is implemented as rectangular CSV plus
JSON metadata sidecar. A native screenshot and live interaction check still require a real desktop session.

---

# UI/UX pass 2: research-instrument polish

Scope: presentation and clarity only. Architecture unchanged (sim thread + `Arc<FrameSnapshot>`), physics core
untouched except for ONE new read-only module (`shadowgrav-core/src/skymap.rs`). Performance work is deliberately not
touched; the one new cost (the sky map) has its own budget and an off switch.

## 1. Sky-map panel (priority 1)

**What it shows.** For the selected body k: the deficit D_k(n), the fraction of flux from direction n that is blocked
by all other bodies with mutual shadowing included, and D^NA_k(n) = D_k(n) - sum_j D_kj(n), the non-additive part that
the overlap correction (eq. 4, second sum) integrates. D^NA <= 0 everywhere and is exactly zero wherever at most one disc
covers n, so the D^NA map is a direct picture of where, and how much, the model departs from pairwise additivity.
Overlays: disc silhouettes labelled by body index, F_pair / F_total / F_corr directions on the map, graticule, colour bar,
hover read-out (lon, lat, D, D^NA, offset from the zoom centre).

**Why two projections.** In the presets the occluders are small: a ~ 0.01 at separation 1 is a 0.57 degree disc, below the
~1.5 degree pixel of any affordable all-sky grid. An all-sky-only panel would show nothing for presets (a), (d1)-(d3).
So there is (i) all-sky Mollweide (equal area, so the pixel mean is the sphere mean) and (ii) a zoom (azimuthal
equidistant, half field of view = factor x angular radius of one chosen disc). On load the panel zooms once, automatically,
on the largest disc if it is smaller than 0.15 rad; clicking a disc number in the table, or the `view` selector, re-frames.

**Where the numbers come from (rule 3 preserved).** `skymap::sky_deficit_map(&World, k, focus, fov)` is a pure function
of `World`: no force evaluation, no mutation. It uses the same per-ray primitives as the force core (eq. 16 chord, the
cancellation-free recursion of `overlap::corr_static` for alpha_bg = 0, the eq. 17 sweep for alpha_bg > 0) on a pixel grid.
It also owns the projection, inverse projection and pixel lookup (`SkyMap::project/unproject/pixel_at`), so the UI only colours,
draws and formats. The map is a diagnostic: forces still come from the HEALPix quadrature and nothing reads this grid back.

**Cost control.** Pixels x occluders is capped at 1.5e6 per map (grid shrinks with the number of occluders: 240x120 for a
handful, 86x43 around N = 400); below 24 rows the map is refused with a visible reason (e.g. N = 2000), never silently
degraded. `Command::SetSkyEnabled(false)` (key S) removes the cost entirely. It is computed at the existing <= 10 Hz
diagnostics cadence for the selected body only.

**Honest limits, stated in the panel.** V2: static geometry only (shifted discs not drawn). thin-fast: the force is not derived
from this deficit (noted). The map mean and eq. 20 D-bar are shown side by side but the map mean is grid-limited when discs span
few pixels; they are a consistency read, not a pass/fail test.

## 2. Hierarchy and layout (priority 2)

* **Badge**: a filled pill, largest element in the top bar, right-aligned; a stamp ("NON-CONFORMING" / "CAVEATS APPLY")
  is also painted into the 3D view so a screenshot cannot be misread. Reasons are no longer hover-only: the right panel
  opens with a conformance card listing them, red reasons first (new `badge_n_red` in the snapshot), then Contract labels.
* **Headline tiles** (dE/E0 max, max D^NA, R_P shadow, cap hits/leaves) replace hunting through sections; colour is used only
  for structural conditions (cap hit, non-converged, diagnostic-only energy), never for invented thresholds.
* **Left panel regrouped**: Scene / Physics / Time stepping / Quadrature / Backend / View, with section rules. The preset
  chooser, description and Load moved out of the top bar, which now holds transport, speed and badge only.
* **Right panel**: conservation, force decomposition, G_eff and the preset read-outs keep their content; quadrature, validity
  and per-body detail are collapsed or conditional. The selected body is highlighted in the decomposition table. Empty or
  uninformative sections (G_eff without instrumentation) are hidden or replaced by one line saying why.

## 3. Force arrows (priority 3)

Any subset of F_total, F_pair, F_corr, F_drag, F_aberr can be drawn at once, each with a fixed colour that is reused in the
legend, the selected-body table and the sky map. Default: selected body only. Two scales: one common length (components are
comparable; F_corr is usually tiny against F_pair) or each to its own maximum (direction only). The legend states the scale
(`70 px = value`). Arrow length is now the true projected length (foreshortened), and a vector along the line of sight is drawn
with the usual dot / cross marker instead of vanishing. Hovering a body shows tau_c, S, |v| and all five force magnitudes.

## 4. Small usability changes (priority 4)

* Preset descriptions: one-line summary plus a "Look at" list per preset (what to read, where, what to expect).
* Tooltips on every physics, stepping, quadrature and backend control, stating what the parameter does and what it gates.
* Default camera chosen from the data (planar scene: look straight at it; otherwise oblique); a body is selected on load so
  arrows and sky map are never empty; "pending changes" label on the Load button.
* Keys: Space, `.`, F fit, 1/2/3 axis views, S sky map, Left/Right previous/next body. Camera buttons XY / XZ / YZ / Fit / Reset.

## Files

| file | change |
|---|---|
| `shadowgrav-core/src/skymap.rs` | NEW. Read-only sky-deficit diagnostic, projections, 3 unit tests. `lib.rs` gains `pub mod skymap;` |
| `src/protocol.rs` | `SetSkyTarget/SetSkyFocus/SetSkyEnabled`; `DiagView.{sky, sky_note, d_bar_eq20}`; `FrameSnapshot.badge_n_red`; `PresetKind::look_for` |
| `src/sim.rs` | sky state + map in `build_diag`; badge returns the red count; 1 new test |
| `src/skyview.rs` | NEW. Panel: texture, overlays, colour bar, statistics, disc table |
| `src/view3d.rs` | component arrows + legend, hover box, axis views, data-driven camera, badge stamp |
| `src/app.rs` | layout, badge pill, tiles, tooltips, scene descriptions, sky wiring and keys |

Conforming behaviour is not touched: no change to `evaluate_forces`, the integrator, presets or the existing diagnostics.

## Verification status

Written without a Rust toolchain available, so NOTHING here has been compiled or run. Pre-checked: bracket balance of every
touched file and a manual API review against egui/eframe 0.29. Run in this order:
`cargo test --release -p shadowgrav-core skymap`, then `cargo test --release -p shadowgrav-ui`, then `cargo run --release -p shadowgrav-ui`.
Likeliest one-line fixes if the compiler objects: `Margin::symmetric`, `Painter::galley` / `rect_stroke` argument lists,
`TextureHandle::set`. Visual checks to do by eye: auto-zoom on presets (a) and (c); D^NA map on (c) shows only the overlap region;
arrows with `corr` on a common scale; the stamp on a red (thin-fast) run.
