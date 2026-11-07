"""Independent oracle for test (c): three equal collinear spheres A,B,C (d_AB = d_BC = 10a), target A.
The configuration is axisymmetric about the A->B->C axis, so eq. 3 reduces to a 1D integral in the polar angle theta:
    F_A / Gamma = 2 pi int_0^pi cos(theta) D(theta) sin(theta) dtheta,   D = 1 - exp(-sum_j beta_j L_j(theta))
    pair sum / Gamma = P_AB + P_AC = A_eff/d_AB^2 + A_eff/d_AC^2     (exact closed form, Spec 2.3)
    R(tau) = F_A / (pair sum).
No HEALPix, no overlap graph, no quadrature rules from the Spec: mpmath adaptive quadrature, 30 digits."""
import mpmath as mp
mp.mp.dps = 30

def R(tau):
    tau = mp.mpf(tau); a = mp.mpf(1); beta = tau / (2 * a)
    dB, dC = 10 * a, 20 * a
    def L(d, th):
        b = d * mp.sin(th); t = d * mp.cos(th)
        if t <= 0 or b >= a: return mp.mpf(0)
        return 2 * mp.sqrt(a * a - b * b)
    f = lambda th: mp.cos(th) * (-mp.expm1(-beta * (L(dB, th) + L(dC, th)))) * mp.sin(th)
    sB, sC = mp.asin(a / dB), mp.asin(a / dC)
    # breakpoints at both silhouettes; sqrt edge singularities -> tanh-sinh handles endpoints
    F = 2 * mp.pi * mp.quad(f, [0, sC, sB])
    A_eff = mp.pi * a * a * (1 - 2 / tau**2 * (1 - (1 + tau) * mp.exp(-tau)))
    pair = A_eff / dB**2 + A_eff / dC**2
    return F / pair

if __name__ == "__main__":
    for tau in ["1e-6", "0.1", "1", "5", "20", "50", "200"]:
        r = R(tau); print(f"tau_c = {tau:>5}:  R = {mp.nstr(r, 15)}   R - 0.8 = {mp.nstr(r - mp.mpf('0.8'), 4)}")
