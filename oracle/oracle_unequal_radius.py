#!/usr/bin/env python3
"""
Independent Fibonacci-grid oracle for non-equal-radius, non-collinear configurations (V0, α_bg=0).

Extends the equal-radius oracle_sky.py logic to different radii and optical depths.
Accuracy is limited by the grid (~1e-4 relative at N=4e6 for rim-heavy cases); always
compare two N values.

Usage:
  python3 oracle_unequal_radius.py
"""
import numpy as np

def fib_chunks(N, chunk=1_000_000):
    g = np.pi * (3.0 - np.sqrt(5.0))
    for s in range(0, N, chunk):
        i = np.arange(s, min(N, s + chunk), dtype=np.float64) + 0.5
        z = 1.0 - 2.0 * i / N
        r = np.sqrt(np.maximum(0.0, 1.0 - z * z))
        ph = g * i
        yield np.stack([r * np.cos(ph), r * np.sin(ph), z], axis=1)


def force_over_gamma(X, a, tau, k, N=4_000_000):
    """X: list of (3,) centres, a: list of radii, tau: list of tau_c. Target k."""
    beta = [t / (2.0 * aa) for t, aa in zip(tau, a)]
    others = [(j, np.asarray(X[j]) - np.asarray(X[k]), a[j], beta[j])
              for j in range(len(X)) if j != k]
    acc = np.zeros(3)
    for n in fib_chunks(N):
        tot = np.zeros(len(n))
        for j, c, aj, bj in others:
            t = n @ c
            b2 = c @ c - t * t
            m = (t > 0) & (b2 < aj * aj)
            tot += np.where(m, 2.0 * np.sqrt(np.maximum(aj * aj - b2, 0.0)), 0.0) * bj
        D = -np.expm1(-tot)
        acc += (n * D[:, None]).sum(axis=0) * (4.0 * np.pi / N)
    return acc


def main():
    # Example: two equal spheres + one larger, non-collinear
    X = [[0.0, 0.0, 0.0], [12.0, 0.0, 0.0], [6.0, 8.0, 1.0]]
    a = [1.0, 1.0, 1.5]
    tau = [5.0, 5.0, 8.0]
    print("Unequal-radius non-collinear example (V0, alpha_bg=0)")
    print(f"Centres: {X}")
    print(f"Radii: {a}")
    print(f"tau_c: {tau}")
    for N in (2_000_000, 8_000_000):
        print(f"\nN = {N}")
        for k in range(3):
            v = force_over_gamma(X, a, tau, k, N)
            print(f"  target {k}: F/Gamma = [{v[0]:.8e}, {v[1]:.8e}, {v[2]:.8e}]")


if __name__ == "__main__":
    main()

def example_b():
    # Second example: closer approach, more overlap risk, unequal
    X = [[0.0, 0.0, 0.0], [6.0, 0.0, 0.0], [3.0, 4.0, 0.5]]
    a = [1.0, 1.2, 0.8]
    tau = [10.0, 4.0, 6.0]
    print("\n=== Geometry B (closer, mixed radii) ===")
    print(f"Centres: {X}")
    print(f"Radii: {a}")
    print(f"tau_c: {tau}")
    results = {}
    for N in (2_000_000, 8_000_000):
        results[N] = [force_over_gamma(X, a, tau, k, N) for k in range(3)]
        print(f"N = {N}")
        for k, v in enumerate(results[N]):
            print(f"  target {k}: [{v[0]:.6e}, {v[1]:.6e}, {v[2]:.6e}]")
    # rough rel diff
    for k in range(3):
        d = np.linalg.norm(results[8_000_000][k] - results[2_000_000][k])
        n = np.linalg.norm(results[8_000_000][k])
        print(f"  target {k} rel diff (2e6 vs 8e6): {d/n if n else 0:.2e}")

if __name__ == "__main__":
    example_b()
