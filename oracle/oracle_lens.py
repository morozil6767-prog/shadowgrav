"""Sharp oracle for PARTIAL and 3D overlap (replaces the 6e-5-noise Fibonacci grid as the accuracy reference).

For a target at the origin and two exterior sphere occluders B, C (radius a = 1, beta = tau/2), Spec eq. 3/4 gives
    F / Gamma = sum_j P_j chat_j  +  Corr,      P_j = pi a^2 A(tau) / d_j^2   (exact, Spec 2.3, alpha_bg = 0, d > a)
    Corr      = int n [D - D_B - D_C] dOmega = - int_{lens} n D_B(n) D_C(n) dOmega
because 1 - e^{-(x+y)} - (1 - e^{-x}) - (1 - e^{-y}) = -(1 - e^{-x})(1 - e^{-y}).  The lens (both discs) is a 2D region; the
integrand has square-root edges at both silhouettes.  Nested QUADPACK, inner limit in closed form (arccos), breakpoints at the
silhouette ranges.  Two independent parametrisations (axis along chat_B, axis along chat_C) are evaluated and compared, so the
reported accuracy is measured, not assumed.   numpy + scipy only.   python3 oracle_lens.py [--write ref.json]
"""
import json, math, sys
import numpy as np
from scipy.integrate import quad


def A_fun(t):
    if t < 0.05:
        return 2*t/3 - t**2/4 + t**3/15 - t**4/72 + t**5/420 - t**6/2880
    return 1 - 2/t**2*(1 - (1 + t)*math.exp(-t))


def _chord_deficit(d, sin_psi, cos_psi, beta):
    b = d*sin_psi
    if cos_psi <= 0 or b >= 1.0: return 0.0
    return -math.expm1(-beta*2.0*math.sqrt(1.0 - b*b))


def lens_correction(cB, cC, tau, tol=1e-13):
    """Return (vector Corr/Gamma, abs-error estimate) for occluder centres cB, cC (units of a) seen from the origin."""
    cB, cC = np.asarray(cB, float), np.asarray(cC, float)
    beta = tau/2.0
    dB, dC = np.linalg.norm(cB), np.linalg.norm(cC)
    uB, uC = cB/dB, cC/dC
    gam = math.acos(max(-1.0, min(1.0, float(uB@uC))))
    thB, thC = math.asin(1/dB), math.asin(1/dC)
    if gam >= thB + thC: return np.zeros(3), 0.0

    def run(axis_d, axis_u, th_a, other_d, other_u, th_o):
        e3 = axis_u
        e1 = (other_u - (axis_u@other_u)*axis_u); e1 /= np.linalg.norm(e1)
        e2 = np.cross(e3, e1)
        cg, sg = float(axis_u@other_u), math.sqrt(1 - float(axis_u@other_u)**2)
        cth_o = math.cos(th_o)

        def inner(psi, comp):
            s, c = math.sin(psi), math.cos(psi)
            DB = _chord_deficit(axis_d, s, c, beta)
            if DB == 0.0: return 0.0
            c0 = (cth_o - c*cg)/(s*sg) if s*sg > 0 else 2.0
            if c0 >= 1.0: return 0.0
            ph0 = math.pi if c0 <= -1.0 else math.acos(c0)
            def g(phi):
                cpo = c*cg + s*sg*math.cos(phi)
                spo = math.sqrt(max(0.0, 1 - cpo*cpo))
                DC = _chord_deficit(other_d, spo, cpo, beta)
                return DC*(c if comp == 0 else s*math.cos(phi))
            v, _ = quad(g, 0.0, ph0, epsabs=tol*1e-3, epsrel=tol, limit=400)
            return 2.0*v*DB*s            # phi in [-ph0, ph0] by symmetry; sin(psi) dpsi measure

        lo, hi = max(0.0, gam - th_o), min(th_a, gam + th_o)
        pts = [x for x in (th_o - gam, gam + th_o) if lo < x < hi]
        out, err = np.zeros(3), 0.0
        for comp, vec in ((0, e3), (1, e1)):
            v, e = quad(lambda p: inner(p, comp), lo, hi, points=pts or None, epsabs=tol*1e-2, epsrel=tol, limit=800)
            out += -v*vec; err += e
        return out, err

    v1, e1_ = run(dB, uB, thB, dC, uC, thC)
    v2, e2_ = run(dC, uC, thC, dB, uB, thB)
    return 0.5*(v1 + v2), float(np.linalg.norm(v1 - v2))      # error = disagreement between the two parametrisations


def force_over_gamma(xs, tau):
    """F_k/Gamma_k for three bodies (units of a), exterior geometry. Returns (vectors, delta_na per body, parametrisation spread)."""
    xs = [np.asarray(x, float) for x in xs]
    Ae = math.pi*A_fun(tau)
    res, nas, spread = [], [], 0.0
    for k in range(3):
        j1, j2 = [i for i in range(3) if i != k]
        c1, c2 = xs[j1] - xs[k], xs[j2] - xs[k]
        pair = Ae*(c1/np.linalg.norm(c1)**3 + c2/np.linalg.norm(c2)**3)
        corr, sp = lens_correction(c1, c2, tau)
        f = pair + corr
        res.append(f); nas.append(float(np.linalg.norm(corr)/np.linalg.norm(f))); spread = max(spread, sp)
    return res, nas, spread


CASES = [([0, 0, 0], [10, 0, 0], [12, 2, 0], 1.0), ([0, 0, 0], [10, 0, 0], [12, 2, 0], 5.0),
         ([0, 0, 0], [10, 0, 0], [12, 1.4, 1.4], 1.0), ([0, 0, 0], [10, 0, 0], [12, 1.4, 1.4], 5.0)]

if __name__ == "__main__":
    out = []
    for xa, xb, xc, tau in CASES:
        f, na, sp = force_over_gamma([xa, xb, xc], tau)
        print(f"x2={xc} tau={tau}:  param-spread (accuracy) {sp:.1e}")
        for k in range(3): print(f"   body {k}: F/Gamma = [{f[k][0]: .10e}, {f[k][1]: .10e}, {f[k][2]: .10e}]  Delta^NA = {na[k]:.12e}")
        out.append({"x": [xa, xb, xc], "tau": tau, "f": [list(map(float, v)) for v in f], "delta_na": na, "spread": sp})
    if "--write" in sys.argv: json.dump(out, open(sys.argv[sys.argv.index("--write") + 1], "w"), indent=1)
