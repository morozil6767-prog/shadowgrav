# Instrument-surface pass (world-class product push)

Implements the seven high-value product items to the limit of the current architecture.

## 1. Sky-map panel
- Shown by default (`show_sky: true`)
- Stronger title + purpose line in the panel
- Notes include quad tier, thin-fast non-conformance, and mode guidance
- Larger default height

## 2. Force-component visualization
- Default arrows: total + pair + correction ON, per-component coloring ON
- Selected-body force table already lists |F| for total/pair/corr/drag/aberration with component colours

## 3. Instrument-grade presets
- Retains quantitative blurbs and regime-status read-outs from the quad-tier pass
- Screening exposes R, |R-0.8|, tier, tolerance, WITHIN/OUTSIDE status

## 4. Export
- New `export.rs`: CSV snapshot with metadata headers (badge, mode, tier, thin-fast, diagnostics extras) + per-body state and force components
- UI **Export** button + keyboard **E**
- Status line shows last export path or error

## 5. Session robustness
- Red status banner on badge Red or thin-fast
- Amber caveat strip with mode + tier
- Errors remain in status bar; bad-edit rollback unchanged in sim thread

## 6. UI information hierarchy
- Top: transport + Export + large badge pill
- Bottom: conformance banner (red/amber) then telemetry + shortcuts
- Right: badge card with explicit mode/tier/thin-fast line before tiles
- Preset read-outs remain the scientific pass/fail surface

## 7. Thin-fast vs conforming UX
- Checkbox label emphasizes NON-CONFORMING
- Inline red warning when enabled
- Full-width red banner while active
- Sky-map notes refuse to treat deficit as the force model

## Owner action
```
cargo run --release -p shadowgrav-ui
```
Verify: Export writes CSV; thin-fast shows red banner; sky open by default; force arrows visible on selection; certified tier still available.
