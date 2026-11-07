"""Double-precision (numpy/scipy) twin of oracle_screening.py, for machines without mpmath, and the generator of
screening_curve.csv (the exact R(tau) reference curve used by the technical report and the convergence study).
Same physics, no Spec machinery: R(tau) = F_A / (P_AB + P_AC), F_A = 2 pi int cos(th) D(th) sin(th) dth over the
axisymmetric sky of body A (spheres B at 10a, C at 20a), D = -expm1(-beta (L_B + L_C)), beta = tau / (2a).
Accuracy: scipy QUADPACK with the two silhouette angles as breakpoints, about 1e-13 (checked against the 30-digit values)."""
import math, sys
import numpy as np
from scipy.integrate import quad

def R(tau):
    a = 1.0; beta = tau / (2 * a); dB, dC = 10 * a, 20 * a
    def L(d, th):
        b = d * math.sin(th); t = d * math.cos(th)
        return 0.0 if (t <= 0 or b >= a) else 2 * math.sqrt(a * a - b * b)
    f = lambda th: math.cos(th) * (-math.expm1(-beta * (L(dB, th) + L(dC, th)))) * math.sin(th)
    sB, sC = math.asin(a / dB), math.asin(a / dC)
    F = 0.0
    for lo, hi in [(0.0, sC), (sC, sB)]:
        v, _ = quad(f, lo, hi, epsabs=1e-16, epsrel=1e-13, limit=2000)
        F += v
    F *= 2 * math.pi
    # A_eff = pi a^2 A(tau), A(tau) = 1 - 2/tau^2 [1 - (1+tau) e^-tau]; series below tau = 0.05 (Spec eq. 2)
    if tau < 0.05:
        A = 2*tau/3 - tau**2/4 + tau**3/15 - tau**4/72 + tau**5/420 - tau**6/2880
    else:
        A = 1 - 2 / tau**2 * (1 - (1 + tau) * math.exp(-tau))
    A_eff = math.pi * a * a * A
    return F / (A_eff / dB**2 + A_eff / dC**2)

if __name__ == "__main__":
    ref = {1.0: 0.877785076145848, 5.0: 0.801867822859093, 20.0: 0.800000002009972}
    for t, r in ref.items():
        print(f"tau {t:5}: scipy {R(t):.15f}  mpmath {r:.15f}  diff {abs(R(t) - r):.2e}")
    taus = np.unique(np.concatenate([np.logspace(-3, math.log10(200), 60), [1.0, 5.0, 20.0, 50.0]]))
    with open(sys.argv[1] if len(sys.argv) > 1 else "screening_curve.csv", "w") as fh:
        fh.write("tau,R_exact\n")
        for t in taus: fh.write(f"{t:.10g},{R(t):.15f}\n")
    print("wrote", len(taus), "points")
