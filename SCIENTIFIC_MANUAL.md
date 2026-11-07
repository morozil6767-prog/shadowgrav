# shadowgrav — Scientific Manual

## 1. What this instrument is

**shadowgrav** is a classical N-body research instrument for a *non-additive shadow-gravity* (Le Sage-type radiation-pressure) model. Bodies are uniform spheres that attenuate an isotropic momentum flux. The force is the exact attenuated-ray integral, not a Newtonian pairwise law with ad hoc softening.

It is a **numerical laboratory** for that mechanism: finite size, saturation, overlap screening, optional background extinction, drag, and first-order aberration.

## 2. What it is not

Not general relativity. Not Sakharov induced gravity. Not a cosmological code. Not a claim of Lorentz-covariant microphysics. Results must not be marketed as GR phenomenology.

## 3. Force law (operator summary)

For each target body, integrate the deficit of incoming momentum flux over the sky. Closed-form pair terms handle isolated discs; overlapping discs use adaptive HEALPix quadrature for the non-additive correction. Optical depth uses a deficit/expm1 formulation for cancellation safety. Interior high-τ pair integrals use cancellation-free odd/even pairing.

**Velocity modes**

| Mode | Content | Badge |
|------|---------|--------|
| V0 | Static shadow force | Green when other gates pass |
| V1 | + flux drag | Green/amber per diagnostics |
| V2 | + first-order aberration | Amber (caveats) |

## 4. Quadrature tiers (official)

| Tier | Parameters | Typical cost | Test (c) \|R−0.8\| |
|------|------------|--------------|---------------------|
| Standard | f=0.15, cap 12 | ~1e3 leaves | ≤ 5e-3 provisional (meas. ~1.3e-3) |
| Validation | f_v=0.02, f_s=4e-4, cap 14 | ~5e5 leaves | ~1.6e-5 |
| Fine | f_v=0.002, f_s=4e-4, cap 14 | ~4.5e6 leaves | ~3.3e-6 |
| Certified | f_v=7e-4, f_s=4e-4, cap 15 | ~6.5e7 leaves, ~10–20 s | ≤ 1e-6 (meas. ~4.3e-7) |

Leaf counts step in HEALPix level stairs (~4× per level). Do not interpolate tiers as a continuum.

**Rim note:** Refining only the rim at fixed interior can *increase* |R−0.8| slightly (partial cancellation of interior vs rim error). Certified is certified on the three-sphere geometry with that understanding.

## 5. Verification spine

### 5.1 Unit / integration tests
`cargo test --release -p shadowgrav-core`

Includes identities, HEALPix reference, integrator, oracle_lens, oracle_sky, reference_mode, edge_cases, diag_gaps, perf_equiv.

### 5.2 Test (c) ladder
`cargo run --release -p shadowgrav-core --bin verify -- c`

### 5.3 Certified single evaluation
`cargo run --release -p shadowgrav-core --bin sgrun -- certify`

### 5.4 Interactive
`cargo run --release -p shadowgrav-ui`  
Presets (a)–(d3), quad tier control, sky map, force components, CSV export (E).

### 5.5 Independent oracles
Python under `oracle/`: screening curve (mpmath), sharp lens, sky grid. See `oracle/README.md`.

## 6. Reading the UI like an instrument

1. **Badge** — CONFORMING / CAVEATS / NON-CONFORMING. Red reasons are disqualifying for quotes.  
2. **Quad tier** — always know which tier produced a number.  
3. **Preset read-outs** — R, |R−0.8|, regime status on screening; Δ^NA vs oracle on partial overlap.  
4. **Thin-fast** — preview only; red banner; do not publish forces.  
5. **Export** — rectangular numeric CSV plus a same-stem `.meta.json` sidecar. The CSV contains one row per body/probe and a stable header; parameters, conformance reasons and diagnostics live in the JSON metadata file.

## 7. Conservation and diagnostics

- V0, no overlap, α_bg=0: energy/angular momentum diagnostics are meaningful; dE scales as Verlet theory.  
- Overlap, V1/V2, or α_bg>0: energy may be labelled non-conservative; use the stated diagnostics, not Newtonian intuition.  
- R_P (shadow momentum residual) monitors quadrature balance.  
- Reference-mode error and τ_max are available in diagnostics (extra diag path / tests).

## 8. Limitations (honest)

- Certified cost precludes long interactive certified campaigns.  
- Partial-overlap Standard tier is coarse by design.  
- V2 validation depth < V0/V1.  
- Cross-platform bitwise reproducibility must be checked via CI, not assumed.  
- Sky map is a diagnostic visualization of deficit structure, not the integrator.

## 9. Citation stance

Cite measured numbers with tier, geometry, and leaf count. Prefer CLAIMS.md wording. If a new measurement contradicts CLAIMS.md, amend CLAIMS.md.

## 10. Architecture (for auditors)

- `shadowgrav-core`: pure physics, binary64, no UI types  
- Simulation thread owns `World`; UI receives immutable snapshots and sends `Command`s  
- Reference mode shares no HEALPix/overlap-graph code with production quadrature  

This separation is intentional: UI bugs cannot silently rewrite the force law.
