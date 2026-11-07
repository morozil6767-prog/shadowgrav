# Claims we make / Claims we do not make

**Instrument:** shadowgrav — classical non-additive shadow-gravity (Le Sage-type) N-body research instrument  
**Version intent:** v1.0 candidate. Numerics verified on the author's Linux/Windows runs (see MACHINE_PROOF.md for what is proven where); the egui UI, CI and multi-OS reproducibility are NOT yet proven; prebuilt releases have never been built.

## Claims we make

1. **Model scope.** The engine implements a classical, non-relativistic shadow mechanism: uniform finite spheres attenuating an isotropic momentum flux. Net force is the attenuated-ray integral (Spec eqs. 3–4), including saturation, overlap (non-additive) screening, optional background extinction, V1 drag, and first-order V2 aberration.

2. **Test (c) — collinear three-body screening (τ_c = 20).**  
   - **Standard** (f = 0.15): |R − 0.8| ≤ 5×10⁻³ (provisional Spec clause). Measured ≈ 1.29×10⁻³.  
   - **Validation** (f_v = 0.02): intermediate; measured ≈ 1.58×10⁻⁵.  
   - **Fine** (f_v = 0.002): measured ≈ 3.32×10⁻⁶ (not Spec 1e-6).  
   - **Certified** (f_v = 7×10⁻⁴, ~65M leaves): |R − R_exact| ≤ 1×10⁻⁶. Measured ≈ 4.3×10⁻⁷ on this geometry. A rim contribution of a few ×10⁻⁷ exists; certified is not rim-free.

3. **Independent oracles.** Axisymmetric screening R(τ) and sharp-lens partial-overlap references exist (Python). Exact R(20) = 0.8 + 2.01×10⁻⁹. Production quadrature is compared against these references in automated tests.

4. **In-core reference mode (Spec 4.4).** For N ≤ 50, V0, α_bg = 0, exterior pairs: an all-rays evaluation of the *same* shadow model (not Newtonian) is available and used to judge the production path.

5. **Parallel path.** Target-parallel force evaluation is bitwise-identical to serial on tested configurations (when built with `parallel`).

6. **Conformance UX.** Interactive runs expose a badge (CONFORMING / CAVEATS / NON-CONFORMING) and, for key presets, explicit regime status. Thin-fast is gated and marked NON-CONFORMING.

7. **Reproducibility within a platform.** Default builds produce a stable `sgrun repro-hash` on a given OS/arch after trig routing through `mathx`. Cross-OS bitwise identity requires CI confirmation with `vendored-libm` (see residual risks).

## Claims we do not make

1. **Not** Sakharov induced gravity, Puthoff ZPF, GR, cosmology, or Lorentz-covariant physics.  
2. **Not** that Standard or Validation/Fine tiers meet Spec 1e-6 on test (c). Only **Certified** does, on the stated geometry.  
3. **Not** that certified error is free of rim contribution, or that the next refinement stair is guaranteed to reach 1e-7.  
4. **Not** that V2 is as thoroughly validated as V0/V1; V2 remains caveat-flagged.  
5. **Not** that thin-fast forces are scientifically interchangeable with conforming forces.  
6. **Not** bitwise identical results across all OS/CPU combinations until multi-OS CI green.  
7. **Not** that the tool has been adopted by external research groups; adoption is outside engineering scope.

## Related documents

- `MODEL_VS_OBSERVATION.md` — what the model does and does not reproduce observationally.

- `ERROR_BUDGETS.md` — independent oracle results for unequal-radius / non-collinear geometries.

- `REPRODUCIBILITY.md` — recorded digests and multi-OS comparison template.

- `MACHINE_PROOF.md` — protocol for local and CI evidence.


## How to challenge a claim

Run the verification sequence in `README.md` (§ Verify in 10 minutes). If a measured number disagrees with this sheet, the measurement wins — update the sheet, do not defend the prose.
