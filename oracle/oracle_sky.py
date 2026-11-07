"""Independent brute-force oracle for eq. 3 / eq. 4 with arbitrary 3D configurations (alpha_bg = 0, V0).
F_k / Gamma_k = int n D_k(n) dOmega, D_k = -expm1(-sum_j beta_j L_j(n)), on a Fibonacci (golden-spiral) sphere grid
with equal weights 4pi/N.  Uses nothing from the Spec's overlap machinery (no HEALPix, no graph, no pair closed form).
Accuracy ~1e-5..1e-4 relative at N = 1.6e7 (rim discontinuities); compare two N to see it."""
import numpy as np

def fib_chunks(N, chunk=1_000_000):
    g = np.pi * (3.0 - np.sqrt(5.0))
    for s in range(0, N, chunk):
        i = np.arange(s, min(N, s + chunk), dtype=np.float64) + 0.5
        z = 1.0 - 2.0 * i / N
        r = np.sqrt(np.maximum(0.0, 1.0 - z * z)); ph = g * i
        yield np.stack([r * np.cos(ph), r * np.sin(ph), z], axis=1)

def force_over_gamma(X, a, tau, k, N=4_000_000):
    """X: (n,3) centres, equal radius a, equal tau_c. Returns int n D dOmega for target k (vector)."""
    beta = tau / (2.0 * a)
    others = [np.asarray(X[j]) - np.asarray(X[k]) for j in range(len(X)) if j != k]
    acc = np.zeros(3)
    for n in fib_chunks(N):
        tot = np.zeros(len(n))
        for c in others:
            t = n @ c; b2 = c @ c - t * t
            m = (t > 0) & (b2 < a * a)
            tot += np.where(m, 2.0 * np.sqrt(np.maximum(a * a - b2, 0.0)), 0.0) * beta
        D = -np.expm1(-tot)
        acc += (n * D[:, None]).sum(axis=0) * (4.0 * np.pi / N)
    return acc

if __name__ == "__main__":
    import sys, json
    a = 1.0
    configs = {
        # partial (non-nested) disc overlap as seen from A: B on the x axis, C offset so the two discs intersect
        "partial_overlap": [[0, 0, 0], [10, 0, 0], [12, 2, 0]],
        # genuinely 3D: C offset out of plane
        "partial_overlap_3d": [[0, 0, 0], [10, 0, 0], [12, 1.4, 1.4]],
    }
    for tau in (1.0, 5.0):
        for name, X in configs.items():
            out = {}
            for N in (4_000_000, 16_000_000):
                out[N] = [force_over_gamma(X, a, tau, k, N) for k in range(3)]
            e = max(np.linalg.norm(out[16_000_000][k] - out[4_000_000][k]) / np.linalg.norm(out[16_000_000][k]) for k in range(3))
            print(f"{name} tau={tau}: N-convergence max rel diff (4e6 vs 1.6e7) = {e:.2e}")
            for k in range(3):
                v = out[16_000_000][k]
                print(f"   k={k}: F/Gamma = [{v[0]:.10e}, {v[1]:.10e}, {v[2]:.10e}]")
