"""Analytic model of the leaf count of the Spec 4.3 / Addendum II.4 quadrature on the test (c) geometry, checked against
report/data/convergence.txt.  Pure Python (math only).  No Rust, no HEALPix library.

Geometry: targets A and C each see two NESTED discs (B at 10a, theta_B = asin(1/10); the far sphere at 20a,
theta_far = asin(1/20), entirely inside B's disc).  Target B sees two disjoint discs (no overlap, no leaves).
Leaf rule (overlap.rs::cell_body): a cell with |T_c| >= 2 is a leaf at level l iff rho_l <= f_v * theta_min
(interior) and, if some disc silhouette lies within rho_l of the cell centre, also rho_l <= f_s * theta_eff of that disc.
rho_l = 1.5 sqrt(A_l), A_l = pi / (3 Nside^2), Nside = 8 * 2^l.   theta_min = theta_far inside the small disc.
"""
import math

TH_FAR = math.asin(1 / 20)
OMEGA_FAR = 2 * math.pi * (1 - math.cos(TH_FAR))          # solid angle of the small disc


def rho(l): return 1.5 * math.sqrt(math.pi / 3) / (8 * 2 ** l)
def area(l): return math.pi / (3 * (8 * 2 ** l) ** 2)


def interior_level(f_v):
    """Smallest level with rho_l <= f_v * theta_far."""
    l = 0
    while rho(l) > f_v * TH_FAR: l += 1
    return l


def predicted_interior_leaves(f_v, n_targets=2):
    return n_targets * OMEGA_FAR / area(interior_level(f_v))


if __name__ == "__main__":
    print(f"theta_far = {TH_FAR:.6f} rad, Omega_far = {OMEGA_FAR:.6e} sr")
    print("\nStair boundaries: interior level l is selected for f_v in [rho_l/theta_far, rho_(l-1)/theta_far)")
    for l in range(9, 16):
        print(f"  l={l:2d}  Nside={8 * 2 ** l:7d}  rho={rho(l):.3e}  f_v >= {rho(l) / TH_FAR:.3e}   interior leaves (2 targets) {2 * OMEGA_FAR / area(l):.4e}")
    f_s = 4e-4
    l_rim = 0
    while rho(l_rim) > f_s * TH_FAR: l_rim += 1
    print(f"\nrim level for f_s={f_s}: l={l_rim} (cap must be >= this; rho={rho(l_rim):.3e} <= {f_s * TH_FAR:.3e})")
    print("\nrow check against convergence.txt (tau=20):")
    data = {}
    for ln in open("../report/data/convergence.txt"):
        t = ln.split()
        if not t or t[0].startswith("#") or float(t[0]) != 20.0: continue
        data[(float(t[1]), int(t[3]))] = int(t[4])
    for (fv, cap), leaves in sorted(data.items(), key=lambda kv: -kv[0][0]):
        if fv < 0.01:
            pred = predicted_interior_leaves(fv)
            print(f"  f_v={fv:<7} cap={cap}: measured {leaves:>10d}  interior-model {pred:.4e}  ratio {leaves / pred:.3f}  (level {interior_level(fv)})")
