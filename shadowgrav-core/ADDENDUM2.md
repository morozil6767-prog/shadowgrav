# Specification Clarification Addendum 2 (numerical hardening, post first execution)

Status: proposed for the frozen contract. Items 1-6 are conforming refinements or tolerance corrections; none
weakens a Contract prohibition. All numbers marked [meas] were reproduced in Python (not Rust).

1. **Test (a) period bound.** The (omega h)^2/24 phase error is the harmonic-oscillator value. For a Kepler circular
   orbit velocity Verlet gives 0.3394 (omega h)^2 [meas: 5.36e-5, 1.340e-5, 3.35e-6, 8.4e-7 at T/500..T/4000].
   Replace "5e-6" by `period_err <= 0.36 (omega h)^2` and require error(h)/error(h/2) in [3.6, 4.4].
2. **Interior pair (eq. 6b).** Conforming rewrite of the same integral: P = 2pi int_0^1 mu [D(mu)-D(-mu)] dmu with
   D(mu)-D(-mu) = (beta/lam) e^{-lam L(-mu)} (-expm1(-2 lam d mu)), L(-mu) = (a^2-d^2)/(sqrt(a^2-d^2+d^2 mu^2)+d mu).
   Exact for every alpha_bg (s1<0). Same panels and nodes as eq. 6b.
3. **Fixed point (eq. 12).** Contraction is ~h*gamma/2 per iteration. (a) Eq. 13's cap 0.1/gamma_max is replaced by
   0.01/gamma_max. (b) The residual denominator is |v| + (h/2)|a| + 1e-30.
4. **Eq. 10 (first-order aberration) verified.** (i) Analytically: the Spec's shifted-geometry map n = lambda q - eps_j
   with Jacobian lambda^2/w gives, to first order, c(1 + c.(3 eps_j + eps_k)) + (eps_k - eps_j). (ii) Numerically, by
   n-space polar integration of g(n;eps_k) D(q_j(n)) that uses NO Jacobian formula (tools/eq10_check.py, a=0.01, d=1,
   tau=1e-6): relative residual / max(|eps|)^2 = 0.109, 0.142, 0.157 at |eps| ~ 0.1, 0.05, 0.025 [meas], i.e. a
   second-order residual with coefficient ~0.15, far inside the 10 eps^2 tolerance of (d3). Eq. 10 may be frozen.
5. **Moments/E accuracy, interior.** E, P0 and the eq. 20 interior moment carry ~1e-15 absolute error on the 2pi scale
   of the unit-sphere integral; relative accuracy is not claimed where P itself is exponentially small.
6. **[SUPERSEDED by ADDENDUM3.md: the xhi figure below did not reproduce; xhi measures 3.32e-6. The 1e-6 target is met by Validation::certified().]** **Test (c) CLOSED.** The floor was the interior midpoint/vector-weight error ~rho^2 set by f_v (validation_hi refined
   only the rim and gained nothing). Measured [meas, Rust]: standard (f=0.15) 1.29e-3; validation f_v=0.02: 1.58e-5;
   f_v=0.002 (Validation::xhi): 6.96e-7, 0 cap-hits, ~1 s. Conformance: standard mode |R-0.8| <= 5e-3; validation mode
   (Validation::xhi) |R-0.8| <= 1e-6, i.e. the original Spec tolerance is met in validation mode.
