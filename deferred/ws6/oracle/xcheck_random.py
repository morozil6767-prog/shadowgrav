"""WS6 cross-check: the Rust engine against an independent Python implementation on random geometries.

What is independent: the force law F_k = Gamma_k * Int n D_k(n) dOmega, D_k = -expm1(-sum_j beta_j L_j(n)) (V0, alpha_bg = 0) is
evaluated here by a different method in a different language from the engine (no HEALPix, no overlap graph, no touching sets, no
closed-form pair term): Gauss-Legendre in sin-substituted polar coordinates about every occluder disc, every direction counted once by
chord-weight sharing; whole-sphere sampling for a target that lies inside a sphere. The pair force is the direct one-dimensional
integral. a, beta, tau_c, A_eff are recomputed from eqs. 1-2 as written in the repository's documentation.
What is NOT independent: the physical model itself (this script encodes the same equations as the engine, as documented in
shadowgrav-core/src/reference.rs and derive.rs; the frozen Spec PDF was not available), and the engine's own reference.rs is a
different method for the same law. Agreement therefore shows that the engine implements this law accurately; it does not test the law.

Usage: python3 oracle/xcheck_random.py --dump <path to xcheck_dump> [--seed 20261107] [--out-prefix prefix] [--certified-every 6]
Needs numpy and scipy. Prints a Markdown report and writes <prefix>_rows.csv and <prefix>_report.md."""
import argparse, csv, math, subprocess, sys
import numpy as np
from scipy.integrate import quad

RHO0 = 2.39e5
A0 = (3.0 / (4.0 * math.pi * RHO0)) ** (1.0 / 3.0)  # radius of the m = 1, rho = RHO0 sphere
TIERS = ["standard", "validation", "graded", "certified"]


# ------------------------------------------------------------------------------------------------------------ derived (eqs. 1-2)
def a_fun(t):
    if t < 0.5:  # Taylor series of 1 - 2/t^2 [1 - (1+t) e^-t] = -2 sum_{n>=3} (-1)^n (n-1) t^(n-2)/n!
        s, u = 0.0, t / 6.0
        for n in range(3, 40):
            s += -2.0 * (1.0 if n % 2 == 0 else -1.0) * (n - 1.0) * u
            u *= t / (n + 1.0)
        return s
    return 1.0 - 2.0 / (t * t) * (1.0 - (1.0 + t) * math.exp(-t))


def derived(m, rho, kappa0):
    a = (3.0 * m / (4.0 * math.pi * rho)) ** (1.0 / 3.0)
    beta = kappa0 * rho
    tau = 2.0 * beta * a
    return a, beta, tau, math.pi * a * a * a_fun(tau)


# ------------------------------------------------------------------------------------------------------------ chords and sky integral
def chord(n, c, a):
    """Length of the intersection of the ray from the origin along unit rows n with the sphere (centre c, radius a), clipped to s >= 0."""
    b = n @ c
    disc = b * b - (c @ c - a * a)
    hit = disc > 0.0
    sq = np.sqrt(np.where(hit, disc, 0.0))
    return np.where(hit, np.maximum(b + sq, 0.0) - np.maximum(b - sq, 0.0), 0.0)


def sky_integral(cs, As, Bs, n_chi, n_phi, n_z):
    """Int n D(n) dOmega as a vector, for occluders (centres cs relative to the target, radii As, betas Bs)."""
    xc, wc = np.polynomial.legendre.leggauss(n_chi)
    chi, wchi = 0.25 * math.pi * (xc + 1.0), 0.25 * math.pi * wc
    xz, wz = np.polynomial.legendre.leggauss(n_z)
    phi = (np.arange(n_phi) + 0.5) * (2.0 * math.pi / n_phi)
    dphi = 2.0 * math.pi / n_phi
    cphi, sphi = np.cos(phi), np.sin(phi)
    acc = np.zeros(3)
    gross = 0.0  # Int D dOmega: the unsigned scale of the integral (cancellation-free)
    for j in range(len(cs)):
        d = float(np.linalg.norm(cs[j]))
        if d > As[j]:  # exterior: cap of half-angle theta_j, sin(psi) = sin(theta_j) sin(chi)
            s = As[j] / d
            e3 = cs[j] / d
            t = np.array([1.0, 0.0, 0.0]) if abs(e3[0]) < 0.9 else np.array([0.0, 1.0, 0.0])
            e1 = np.cross(e3, t); e1 /= np.linalg.norm(e1); e2 = np.cross(e3, e1)
            sinpsi = s * np.sin(chi)
            cospsi = np.sqrt(np.maximum(1.0 - sinpsi * sinpsi, 0.0))
            wgt1 = s * s * np.sin(chi) * np.cos(chi) / cospsi * wchi          # dOmega = s^2 sin(chi) cos(chi)/cos(psi) dchi dphi
            nn = (cospsi[:, None, None] * e3[None, None, :]
                  + sinpsi[:, None, None] * (cphi[None, :, None] * e1[None, None, :] + sphi[None, :, None] * e2[None, None, :]))
            nn = nn.reshape(-1, 3)
            wgt = (np.repeat(wgt1, n_phi) * dphi)
        else:  # target inside (or on) sphere j: the whole sky belongs to disc j
            zz = xz
            r = np.sqrt(1.0 - zz * zz)
            nn = np.stack([r[:, None] * cphi[None, :], r[:, None] * sphi[None, :], np.broadcast_to(zz[:, None], (n_z, n_phi))], axis=-1).reshape(-1, 3)
            wgt = np.repeat(wz, n_phi) * dphi
        Ls = [chord(nn, cs[i], As[i]) * Bs[i] for i in range(len(cs))]
        T = np.sum(Ls, axis=0)
        D = -np.expm1(-T)
        share = np.where(T > 0.0, Ls[j] / np.where(T > 0.0, T, 1.0), 0.0)
        acc += (nn * (D * share * wgt)[:, None]).sum(axis=0)
        gross += float((D * share * wgt).sum())
    return acc, gross


def pair_vector(c, a, beta):
    """Int n D_kj dOmega for ONE occluder (direct 1-D integral; axis = direction to the centre)."""
    d = float(np.linalg.norm(c)); e = c / d
    if d > a:
        s = a / d; tau = 2.0 * beta * a
        val, _ = quad(lambda ch: -math.expm1(-tau * math.cos(ch)) * math.sin(ch) * math.cos(ch), 0.0, 0.5 * math.pi, epsabs=1e-15, epsrel=1e-13)
        return e * (2.0 * math.pi * s * s * val)
    def f(z):
        b = d * z
        L = b + math.sqrt(max(b * b - (d * d - a * a), 0.0))
        return z * -math.expm1(-beta * L)
    val, _ = quad(f, -1.0, 1.0, epsabs=1e-15, epsrel=1e-13, limit=200)
    return e * (2.0 * math.pi * val)


def oracle(x, m, rho, kappa0, res):
    """Returns per-body (total vector, pair vector, gross scale) as F/Gamma, the oracle's own error estimate (max over bodies, |fine - coarse| / gross, gross = Int D dOmega) and the worst conditioning |F|/gross."""
    n = len(x)
    der = [derived(m[i], rho[i], kappa0) for i in range(n)]
    out = []
    err = 0.0
    cond = 1.0
    for k in range(n):
        idx = [j for j in range(n) if j != k]
        cs = [np.asarray(x[j]) - np.asarray(x[k]) for j in idx]
        As = [der[j][0] for j in idx]; Bs = [der[j][1] for j in idx]
        g1, _ = sky_integral(cs, As, Bs, res[0], res[1], res[2])
        g2, gross = sky_integral(cs, As, Bs, 2 * res[0], 2 * res[1], 2 * res[2])
        pair = sum((pair_vector(cs[q], As[q], Bs[q]) for q in range(len(idx))), np.zeros(3))
        out.append((g2, pair, gross))
        err = max(err, float(np.linalg.norm(g2 - g1)) / gross)
        cond = min(cond, float(np.linalg.norm(g2)) / gross)
    return der, out, err, cond


# ------------------------------------------------------------------------------------------------------------ geometries
def make_geometries(seed):
    rng = np.random.default_rng(seed)
    G = []

    def unit():
        v = rng.normal(size=3); return v / np.linalg.norm(v)

    def body_props(n, unequal):
        if not unequal:
            return [1.0] * n, [RHO0] * n
        return list(rng.uniform(0.5, 2.0, n)), list(RHO0 * rng.uniform(0.7, 1.4, n))

    def add(cls, x, unequal, tau0):
        m, rho = body_props(len(x), unequal)
        G.append(dict(cls=cls, x=[list(map(float, p)) for p in x], m=m, rho=rho, tau0=tau0, unequal=unequal))

    taus = [0.5, 2.0, 5.0, 20.0]
    for i in range(10):  # A: sparse exterior
        n = int(rng.integers(3, 6)); x = [np.zeros(3)]
        while len(x) < n:
            p = unit() * rng.uniform(6, 30) * A0
            if all(np.linalg.norm(p - q) > 4 * A0 for q in x): x.append(p)
        add("A sparse", x, i % 2 == 1, taus[i % 4])
    for i in range(14):  # B: partial overlap, genuinely 3D clusters seen from body 0
        n = int(rng.integers(3, 6)); x = [np.zeros(3)]; axis = unit()
        for _ in range(n - 1):
            d = rng.uniform(8, 16) * A0
            v = axis + rng.normal(size=3) * rng.uniform(0.05, 0.25); v /= np.linalg.norm(v)
            p = v * d
            if all(np.linalg.norm(p - q) > 3 * A0 for q in x): x.append(p)
        if len(x) >= 3: add("B partial overlap", x, i % 2 == 0, taus[(i + 1) % 4])
    for i in range(12):  # C: deep overlap, near-collinear stacks
        axis = unit(); n = int(rng.integers(3, 6)); x = [np.zeros(3)]; d = rng.uniform(7, 10) * A0
        for _ in range(n - 1):
            x.append(axis * d + rng.normal(size=3) * rng.uniform(0.0, 0.3) * A0); d += rng.uniform(4, 9) * A0
        add("C deep overlap", x, i % 2 == 1, taus[(i + 2) % 4])
    for i in range(12):  # D: near contact between body 0 and body 1 (separation (a0+a1)(1+eps)), others behind
        eps = [1e-6, 1e-3, 1e-2, 0.05][i % 4]; axis = unit()
        m, rho = body_props(2, True)
        a0_, a1_ = derived(m[0], rho[0], 1.0)[0], derived(m[1], rho[1], 1.0)[0]
        x = [np.zeros(3), axis * (a0_ + a1_) * (1 + eps)]
        for _ in range(int(rng.integers(1, 3))):
            v = axis + rng.normal(size=3) * 0.3; v /= np.linalg.norm(v); x.append(v * rng.uniform(8, 14) * A0)
        G.append(dict(cls="D near contact", x=[list(map(float, p)) for p in x], m=m + [1.0] * (len(x) - 2), rho=rho + [RHO0] * (len(x) - 2), tau0=taus[i % 4], unequal=True))
    for i in range(12):  # E: body 0 inside body 1 (overlapping spheres), others outside
        f = [0.2, 0.5, 0.9][i % 3]; axis = unit()
        m, rho = body_props(2, True); a1_ = derived(m[1], rho[1], 1.0)[0]
        x = [np.zeros(3), axis * f * a1_]
        for _ in range(int(rng.integers(1, 3))): x.append(unit() * rng.uniform(5, 14) * A0)
        G.append(dict(cls="E interior", x=[list(map(float, p)) for p in x], m=m + [1.0] * (len(x) - 2), rho=rho + [RHO0] * (len(x) - 2), tau0=taus[i % 4], unequal=True))
    return G


# ------------------------------------------------------------------------------------------------------------ engine
def run_engine(dump, g, tier, timeout=900):
    lines = [f"{tier} {g['tau0']}", str(len(g["x"]))]
    for p, mm, rr in zip(g["x"], g["m"], g["rho"]):
        lines.append(" ".join(f"{v:.17e}" for v in (*p, mm, rr)))
    r = subprocess.run([dump], input="\n".join(lines) + "\n", capture_output=True, text=True, timeout=timeout)
    body, status, glob = [], None, None
    for ln in r.stdout.splitlines():
        t = ln.split()
        if not t: continue
        if t[0] == "ERROR": return dict(error=" ".join(t[1:]))
        if t[0] == "GLOBALS": glob = [float(v) for v in t[1:]]
        if t[0] == "BODY": body.append([float(v) for v in t[2:]])
        if t[0] == "STATUS": status = (int(t[1]), int(t[2]), t[3])
    if status is None or glob is None or r.returncode != 0:
        return dict(error=f"no output (rc={r.returncode}) {r.stderr[:200]}")
    return dict(kappa0=glob[0], body=body, leaves=status[0], cap_hits=status[1])


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--dump", required=True)
    ap.add_argument("--seed", type=int, default=20261107)
    ap.add_argument("--out-prefix", default="xcheck")
    ap.add_argument("--certified-every", type=int, default=6, help="run the certified tier on every k-th geometry (0 = never)")
    ap.add_argument("--res", default="192,768,96", help="n_chi,n_phi,n_z of the coarse oracle grid (the fine grid doubles it)")
    a = ap.parse_args()
    res = tuple(int(v) for v in a.res.split(","))
    geoms = make_geometries(a.seed)
    rows = []
    worst_oracle = 0.0
    for gi, g in enumerate(geoms):
        kappa0 = 2.0 * math.pi * A0 ** 2 * g["tau0"] / 3.0  # kappa0_for_tau(tau0, 1, RHO0)
        der, orc, oerr, cond = oracle(g["x"], g["m"], g["rho"], kappa0, res)
        worst_oracle = max(worst_oracle, oerr)
        for tier in TIERS:
            if tier == "certified" and (a.certified_every == 0 or gi % a.certified_every != 0): continue
            e = run_engine(a.dump, g, tier)
            row = dict(geom=gi, cls=g["cls"], n=len(g["x"]), tau0=g["tau0"], unequal=g["unequal"], tier=tier, oracle_err=oerr, cond=cond)
            if "error" in e:
                row.update(refused=e["error"]); rows.append(row); continue
            q_der = q_pair = q_tot = q_corr = q_totrel = 0.0
            for k, b in enumerate(e["body"]):
                a_e, beta_e, tau_e, aeff_e = b[0:4]
                a_p, beta_p, tau_p, aeff_p = der[k]
                q_der = max(q_der, *(abs(u - v) / abs(v) for u, v in ((a_e, a_p), (beta_e, beta_p), (tau_e, tau_p), (aeff_e, aeff_p))))
                tot_e, pair_e, corr_e = np.array(b[4:7]), np.array(b[7:10]), np.array(b[10:13])
                tot_p, pair_p, gross = orc[k]
                nt = max(float(np.linalg.norm(tot_p)), 1e-300)
                q_totrel = max(q_totrel, float(np.linalg.norm(tot_e - tot_p)) / nt)
                q_tot = max(q_tot, float(np.linalg.norm(tot_e - tot_p)) / gross)
                q_pair = max(q_pair, float(np.linalg.norm(pair_e - pair_p)) / max(float(np.linalg.norm(pair_p)), 1e-300))
                q_corr = max(q_corr, float(np.linalg.norm(corr_e - (tot_p - pair_p))) / gross)
            row.update(derived=q_der, pair=q_pair, total=q_tot, corr=q_corr, total_rel=q_totrel, leaves=e["leaves"], cap_hits=e["cap_hits"])
            rows.append(row)
        print(f"[{gi + 1}/{len(geoms)}] {g['cls']:18s} n={len(g['x'])} tau0={g['tau0']:<4} oracle err {oerr:.1e}", file=sys.stderr, flush=True)
    keys = ["geom", "cls", "n", "tau0", "unequal", "tier", "oracle_err", "derived", "pair", "total", "corr", "total_rel", "cond", "leaves", "cap_hits", "refused"]
    with open(a.out_prefix + "_rows.csv", "w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=keys, extrasaction="ignore"); w.writeheader(); w.writerows(rows)
    # ------------------------------------------------------------------ report
    out = [f"# Cross-check report (seed {a.seed}, {len(geoms)} geometries, oracle grid {res} and {tuple(2 * v for v in res)})", "",
           f"Oracle self-estimate (max over geometries of |fine - coarse| / gross, gross = Int D dOmega): {worst_oracle:.2e}", "",
           "Maximum discrepancy per quantity, relative (derived: a, beta, tau_c, A_eff; pair: closed form vs direct 1-D integral; total: |F_engine - F_oracle| / gross with gross = Int D dOmega, the cancellation-free scale; corr = total - pair on the same scale; `total_rel` = relative to |F| itself, shown for geometries with |F|/gross >= 0.05 only). `refused` = engine returned an error.", ""]
    out.append("| class | tier | geometries | refused | derived | pair | total | corr | total/abs(F), cond>=0.05 | beyond oracle noise | max leaves | cap hits |")
    out.append("|---|---|---|---|---|---|---|---|---|---|---|---|")
    for cls in sorted({r["cls"] for r in rows}) + ["ALL"]:
        for tier in TIERS:
            rs = [r for r in rows if r["tier"] == tier and (cls == "ALL" or r["cls"] == cls)]
            if not rs: continue
            ok = [r for r in rs if "derived" in r]
            f = lambda k: f"{max(r[k] for r in ok):.1e}" if ok else "n/a"
            wc = [r for r in ok if r["cond"] >= 0.05]
            rel = f"{max(r['total_rel'] for r in wc):.1e}" if wc else "n/a"
            beyond = sum(1 for r in ok if r["total"] > 3.0 * r["oracle_err"] + 1e-13)
            out.append(f"| {cls} | {tier} | {len(rs)} | {len(rs) - len(ok)} | {f('derived')} | {f('pair')} | {f('total')} | {f('corr')} | {rel} | {beyond} | "
                       f"{max((r['leaves'] for r in ok), default=0):,} | {sum(r['cap_hits'] for r in ok)} |")
    out.append("")
    out.append("Worst three geometries per tier (total):")
    for tier in TIERS:
        ok = sorted([r for r in rows if r["tier"] == tier and "total" in r], key=lambda r: -r["total"])[:3]
        for r in ok:
            out.append(f"- {tier}: geometry {r['geom']} ({r['cls']}, n={r['n']}, tau0={r['tau0']}): total {r['total']:.2e}, oracle err {r['oracle_err']:.1e}, |F|/gross {r['cond']:.1e}")
    ref = [r for r in rows if "refused" in r]
    if ref:
        out.append(""); out.append("Engine refusals:")
        for r in ref[:20]: out.append(f"- geometry {r['geom']} ({r['cls']}) {r['tier']}: {r['refused'][:120]}")
    text = "\n".join(out)
    open(a.out_prefix + "_report.md", "w").write(text + "\n")
    print(text)


if __name__ == "__main__":
    main()
