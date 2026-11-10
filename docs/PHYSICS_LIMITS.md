# Physics limits of the implemented model

Equations below are the ones in the source. Each number is copied from a file already in the repository. This chapter does not add a measurement. Agreement with an oracle tests the implementation of these equations. It does not test whether the physical law is the law of gravity.

`Validation::graded` is not the certified tier. `types.rs` sets `certified()` to `f_v = 7e-4`, `grade = 0`, and `graded()` to `f_v = 0.0025`, `grade = 0.125`. `shadowgrav-ui/src/sim.rs` labels `f_v <= 7.5e-4` as Certified and `f_v <= 0.0025` as Fine, so the graded preset is labelled Fine. `WS3_QUADRATURE.md` section 4 says adopting graded as the Spec's certified tier is an owner decision. Nothing here does that.

## Thin limit (tau_c -> 0)

`derive.rs`:

- radius `a = cbrt(3 m / (4 π rho))`
- `tau = 2 beta a` with `beta = kappa rho`
- `A(tau)` from `a_fun`: the 40-term series when `tau < SERIES_TAU` (`SERIES_TAU = 1.0`), otherwise `1 - 2/tau^2 * (1 - (1+tau) exp(-tau))`
- `a_eff = π a^2 A(tau)`
- `s = 3 A(tau) / (2 tau)` when `tau > 0`, else `1`

`pair.rs` `pair_p` returns `a_eff / d^2` when the pair is exterior (`d > a`), `alpha == 0`, and `force_gl` is false. That is the Newtonian-shaped two-body force of this model: it falls as `1/d^2`, with the factor `A(tau)` still in `a_eff`.

`verify.rs` stage A checks `F d^2 = S^2` with `S = 1.5 * A(tau) / tau`, for `tau` in {0.1, 1, 10} and `d/a` in {1.5, 5, 50}, bound `1e-12`. That check is source. It was not re-run for this chapter.

Recorded thin-sphere comparison, `TUTORIAL.md` section 2 only: `scenarios/two_body_thin.json`, `tau_c = 1e-6`, separation `10 a0`. Hand value `F = 1/(10 a0)^2`. The tutorial records engine `F = 100.07463`, hand `100.07471`, absolute difference `7.5e-5`, relative difference `7.5e-7`, and calls that the thin-limit saturation error (`delta_sat`) at `tau_c = 1e-6`. No rustc 1.88.0 log of that hand comparison is in the tree. The tutorial does not name a compiler.

## Saturation (large tau_c)

As `tau` grows, the closed form in `a_fun` keeps `A(tau)` below 1 by the term `2/tau^2 * (1 - (1+tau) exp(-tau))`. The factor `s = 3 A / (2 tau)` falls as `1/tau`. `verify.rs` names the two-body identity `F d^2 = S^2`.

`pair.rs` implements the exterior, `alpha_bg = 0` pair as `a_eff / d^2` with `a_eff = π a^2 A(tau)`. `WS6_VALIDATION.md` section 1 calls that closed form `pi s^2 A(tau)` and contrasts it with the oracle's 1-D integral. This chapter does not add a conversion between those two writings.

Test (c) oracle values, copied from `oracle/README.md` (not recomputed here):

| tau | R(tau) |
|---|---|
| 1 | 0.877785076145848 |
| 5 | 0.801867822859093 |
| 20 | 0.800000002009972 |

`oracle/README.md` says `R(20)` is `2.0e-9` above `0.8`. The digit `0.800000002009972 - 0.8` is `2.009972e-9`. `CLAIMS.md` writes `0.8 + 2.01e-9`. The tests named in `oracle/README.md` compare the engine with these constants. Passing that comparison checks the implementation. It does not establish the screening law.

## Overlap and non-additivity

`reference.rs` evaluates

```text
D_k = -expm1(-sum_j beta_j L_j(n))
```

which is `1 - exp(-sum_j beta_j L_j)` along direction `n`. For `x_j = beta_j L_j > 0` with two or more positive terms,

```text
1 - exp(-sum_j x_j) < sum_j (1 - exp(-x_j))
```

The left side is the joint deficit. The right side is the sum of single-occluder deficits. The joint shadow is smaller than the sum of the separate shadows, so the screening correction is subadditive. No new numerical ratio is computed here.

`WS6_VALIDATION.md` section 2, class A (no overlapping shadows): the total equals the pair sum to `2e-13` at every tier, and the engine takes the pair path (`leaves = 0`). That sentence is from that file. The 58-geometry run was not repeated for this chapter.

## Interior targets

`pair.rs` uses the exterior node set when `d > a` and the eq. 6b interior panels when `d <= a`. The interior force is written as an integral of `D(mu) - D(-mu)` so the two hemispheres are not subtracted after the fact. The comment in `pair.rs` says that difference is

```text
(beta/lam) * exp(-lam * L(-mu)) * (-expm1(-2 * lam * d * mu))
```

`WS6_VALIDATION.md` section 2: a target inside a sphere has `|F|/gross` as low as `1e-3`, and the relative error against `|F|` there is mostly the oracle's own noise. The same file says the class E pair column is `5.1e-15`.

## Near contact

`pair.rs` header: for `Omega`, `tau_bar`, and `E`, the weight is singular near contact. With `r = a/d` and `kappa = sqrt(1-r^2)/r`, the code substitutes `x = kappa * sinh(s)`. The header says the integrand is smooth for all `r < 1`.

`WS6_VALIDATION.md` class D is separation `(a_0 + a_1) (1 + eps)` with `eps` from `1e-6`. The pair column (closed form against the 1-D integral), same file, same table: `9.3e-15` at standard, validation, and graded (58 geometries); `1.8e-15` at certified (10 geometries).

## Quadrature error by tier

`WS6_VALIDATION.md` was not regenerated for this chapter. `WS3_QUADRATURE.md` sections 1–5 are the rustc 1.97.0 record. Its section 0 is a rustc 1.88.0 re-run of the certified, graded, and xhi error rows only. Log `/tmp/sg-ws7d-meas.log`, commit `2b7127db01ab`, `parallel: true`, 4 threads.

Test (c), `|R - R_exact|`, from that log (not from the 1.97.0 rounding):

| tier | tau = 1 | tau = 5 | tau = 20 | leaves |
|---|---|---|---|---|
| certified | 4.741912e-8 | 1.822237e-7 | 4.307904e-7 | 64,767,596 |
| graded | 1.513718e-7 | 3.212910e-7 | 1.496597e-8 | 9,884,156 |
| xhi | 5.554733e-7 | 1.946534e-6 | 3.322159e-6 | 4,467,284 |

These figures in this chapter still come only from the rustc 1.97.0 text in `WS3_QUADRATURE.md`. They are not in `/tmp/sg-ws7d-meas.log`: the defensible graded statement `4.0e-7`; the rim-refined errors `-1.92e-7` (tau 20) and `-3.99e-7` (tau 5) at `f_s` `1e-4`, cap 16; the cap-15 to cap-16 shift `-2.68e-7`; the ladder rungs; the self-convergence indicator; the lens table in that file's section 3; the one-thread seconds `16.8`, `3.5`, and `1.1`; and the rounded error row `4.74e-8`, `1.82e-7`, `4.31e-7`, `1.51e-7`, `3.21e-7`, `1.50e-8`, `5.56e-7`, `1.95e-6`, `3.32e-6`. The leaf counts in that rounded table (`64,767,596`, `9,884,156`, `4,467,284`) are the same three counts as the 1.88.0 log.

`WS6_VALIDATION.md` (seed `20261107`, 58 geometries). Discrepancy over gross:

| tier | geometries | total/gross | pair | max leaves |
|---|---|---|---|---|
| standard | 58 | 7.1e-3 | 9.3e-15 | 9.7e3 |
| validation | 58 | 1.6e-4 | 9.3e-15 | 3.6e6 |
| graded | 58 | 2.1e-5 | 9.3e-15 | 7.0e7 |
| certified | 10 (every 6th) | 2.9e-6 | 1.8e-15 | 1.6e8 |

The same file: graded reaches `2.1e-5` of gross on these 58 geometries (`6.5e-6` class C, `3.8e-6` near contact) and does not show that graded meets `1e-6` on general geometries. On the 10 certified geometries the discrepancy is at most `2.9e-6`, which that file says is within the oracle's own estimate, not a demonstration of `1e-6` on those geometries.

## What the model does not claim

`MODEL_VS_OBSERVATION.md` is the list. This instrument computes one classical non-relativistic shadow model. That file says it does not compute general relativity, and its table says:

- no GR-like perihelion precession (the file's GR column quotes `+42.98″/century`)
- light deflection is not in the model (the file's GR column quotes `1.75″`; the model column says zero)
- no Shapiro delay, no gravitational redshift, no gravitational waves, no frame-dragging, no event horizons
- a velocity-dependent drag is present in the V1 term

Those observational figures are the other column of that document. They were not computed by this engine. `WS6_VALIDATION.md` section 1: the cross-check encodes the same equations as the repository; agreement shows the engine evaluates this law on the cases it ran. It does not test the law, and it does not cover V1, V2, or the dynamics.

`CLAIMS.md` non-claims match that limit: not Sakharov, Puthoff, GR, cosmology, or Lorentz-covariant physics; not a statement that standard, validation, or fine meet Spec `1e-6` on test (c); not a rim-free certified error; not a claim that V2 is validated as thoroughly as V0/V1; not a claim that thin-fast forces may be quoted as conforming; not cross-OS bitwise identity; not external adoption.
