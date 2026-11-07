"""Regenerates report/figures/*.png from report/data/* (data produced by sgrun, converge and oracle_screening_scipy.py).
   python3 make_figures.py      (numpy, matplotlib)"""
import csv, json, math, os
import numpy as np
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib.colors import LinearSegmentedColormap

HERE = os.path.dirname(os.path.abspath(__file__)); D = os.path.join(HERE, "data"); F = os.path.join(HERE, "figures")
SURFACE, INK, INK2, GRID = "#fcfcfb", "#0b0b0b", "#52514e", "#e4e3df"
BLUE, ORANGE, AQUA, YELLOW = "#2a78d6", "#eb6834", "#1baf7a", "#eda100"      # validated slots 1-4 (dataviz reference palette)
plt.rcParams.update({"figure.facecolor": SURFACE, "axes.facecolor": SURFACE, "savefig.facecolor": SURFACE, "text.color": INK,
    "axes.labelcolor": INK2, "xtick.color": INK2, "ytick.color": INK2, "axes.edgecolor": GRID, "font.size": 10,
    "axes.spines.top": False, "axes.spines.right": False, "axes.grid": True, "grid.color": GRID, "grid.linewidth": 0.6})

def read(path): return list(csv.DictReader(open(os.path.join(D, path))))

# ---------------------------------------------------------------- figure 1: screening curve and error by quadrature mode
exact = [(float(r["tau"]), float(r["R_exact"])) for r in read("screening_curve.csv")]
ex = dict(exact)
modes = [("standard", BLUE, "o", "standard (f = 0.15)"), ("validation", ORANGE, "s", "validation (f_v = 0.02)"),
         ("xhi", AQUA, "^", "xhi (f_v = 0.002)"), ("certified", YELLOW, "D", "certified (f_v = 7e-4)")]
fig, (a1, a2) = plt.subplots(1, 2, figsize=(11, 4.8), gridspec_kw={"width_ratios": [1, 1.35]})
t, r = zip(*exact)
a1.plot(t, r, color=BLUE, lw=2); a1.set_xscale("log"); a1.axhline(0.8, color=INK2, lw=0.8, ls="--")
a1.set_xlabel("optical depth tau_c"); a1.set_ylabel("R = |F_A| / |F_AB + F_AC|"); a1.set_title("Exact reference curve (independent oracle)", loc="left", color=INK, fontsize=11)
a1.text(0.0015, 0.835, "R -> 0.8 as tau_c -> infinity\n(C's disc lies inside B's); dashed = 0.8", color=INK2, fontsize=9)
for name, col, mk, lab in modes:
    rows = [(float(x["tau"]), float(x["R"])) for x in read(f"scan_{name}.csv")]
    import sys; sys.path.insert(0, os.path.join(HERE, "..", "oracle")); from oracle_screening_scipy import R   # exact value at every scanned tau
    tt = [a for a, b in rows]; ee = [max(abs(b - R(a)), 1e-16) for a, b in rows]
    a2.plot(tt, ee, color=col, marker=mk, ms=4.5 if name != "certified" else 6, lw=1.4, label=lab, mec=SURFACE, mew=0.8)
a2.axhline(1e-6, color=INK, lw=1, ls="--", label="Spec tolerance 1e-6")
a2.set_xscale("log"); a2.set_yscale("log"); a2.set_xlim(1e-3, 300); a2.set_ylim(1e-9, 1e-2)
a2.set_xlabel("optical depth tau_c"); a2.set_ylabel("|R - R_exact|"); a2.set_title("Measured quadrature error, test (c) geometry", loc="left", color=INK, fontsize=11)
a2.legend(frameon=False, loc="upper center", bbox_to_anchor=(0.5, -0.17), ncol=3, fontsize=8.5)
fig.tight_layout(); fig.savefig(os.path.join(F, "fig1_screening_error.png"), dpi=200); plt.close(fig)

# ---------------------------------------------------------------- figure 2: convergence (error vs leaves)
data = {}
for line in open(os.path.join(D, "convergence.txt")):
    p = line.split()
    if not p or p[0].startswith("#"): continue
    tau, leaves, err = float(p[0]), float(p[4]), float(p[8]); data.setdefault(tau, {})[leaves] = err      # duplicates (level-capped) collapse
fig, ax = plt.subplots(figsize=(6.4, 4.4))
for tau, col, mk in [(1.0, BLUE, "o"), (5.0, ORANGE, "s"), (20.0, AQUA, "^")]:
    pts = sorted(data[tau].items()); x, y = zip(*pts)
    ax.plot(x, y, color=col, marker=mk, ms=5, lw=1.4, mec=SURFACE, mew=0.8)
    ax.annotate(f"tau_c = {tau:g}", xy=(x[-1], y[-1]), xytext=(6, 0), textcoords="offset points", color=INK2, fontsize=9, va="center")
xs = np.array([1e3, 1e8]); ax.plot(xs, 2.5e-3 * (xs / 1e3) ** -0.75, color=INK2, lw=0.9, ls=":")
ax.text(4e5, 2.2e-3, "dotted: slope -0.75 (the Spec's O(h^3/2)\nsilhouette-edge midpoint error)", color=INK2, fontsize=8.5)
ax.axhline(1e-6, color=INK, lw=1, ls="--"); ax.text(1.5e3, 1.25e-6, "Spec tolerance 1e-6", color=INK, fontsize=9)
ax.set_xscale("log"); ax.set_yscale("log"); ax.set_xlim(8e2, 6e8); ax.set_ylim(2e-8, 5e-3)
ax.set_xlabel("quadrature leaves (all three targets)"); ax.set_ylabel("|R - R_exact|"); ax.set_title("Convergence of the overlap quadrature", loc="left", color=INK, fontsize=11)
fig.tight_layout(); fig.savefig(os.path.join(F, "fig2_convergence.png"), dpi=200); plt.close(fig)

# ---------------------------------------------------------------- figure 3: sky maps of the selected body (zoom on disc B)
cmapD = LinearSegmentedColormap.from_list("d", [SURFACE, "#9cc3ee", BLUE, "#123f78"])
cmapN = LinearSegmentedColormap.from_list("n", [SURFACE, "#f7b79c", ORANGE, "#7a2a0c"])
cases = [("c", "(c) collinear, nested discs"), ("c2", "(c2) partial overlap, planar"), ("c3", "(c3) partial overlap, 3D")]
fig, axs = plt.subplots(2, 3, figsize=(11, 7.4))
for j, (case, title) in enumerate(cases):
    meta = json.load(open(os.path.join(D, f"sky_{case}.json")))
    Dg = np.loadtxt(os.path.join(D, f"sky_{case}_D.csv"), delimiter=","); Ng = np.loadtxt(os.path.join(D, f"sky_{case}_DNA.csv"), delimiter=",")
    c, e1, e2, hf = map(np.array, (meta["c"], meta["e1"], meta["e2"], [meta["half_fov"]])); hf = float(hf[0])
    for i, (G, cm, lab) in enumerate([(Dg, cmapD, "deficit D"), (-Ng, cmapN, "non-additive part  -D^NA  (= sum D_j - D)")]):
        ax = axs[i, j]; ax.grid(False)
        im = ax.imshow(G, extent=[-1, 1, -1, 1], cmap=cm, vmin=0, vmax=np.nanmax(G) if np.nanmax(G) > 0 else 1, interpolation="nearest")
        for d in meta["discs"]:        # disc outline: azimuthal-equidistant projection of the cone of half-angle theta about the disc centre
            dc = np.array(d["centre"]); u = np.cross(dc, [0.3, 0.5, 0.8]); u /= np.linalg.norm(u); v = np.cross(dc, u)
            ph = np.linspace(0, 2 * np.pi, 400); n = np.cos(d["theta"]) * dc[None] + np.sin(d["theta"]) * (np.cos(ph)[:, None] * u + np.sin(ph)[:, None] * v)
            ang = np.arccos(np.clip(n @ c, -1, 1)); az = np.arctan2(n @ e2, n @ e1)
            ax.plot(ang * np.cos(az) / hf, ang * np.sin(az) / hf, color=INK, lw=1)
            cen = np.arccos(np.clip(dc @ c, -1, 1)); caz = np.arctan2(dc @ e2, dc @ e1)
            ax.text(cen * np.cos(caz) / hf, cen * np.sin(caz) / hf + d["theta"] / hf + 0.06, str(d["j"]), color=INK, ha="center", va="bottom", fontsize=10, fontweight="bold")
        ax.set_xticks([]); ax.set_yticks([]); ax.set_xlim(-1, 1); ax.set_ylim(-1, 1)
        cb = fig.colorbar(im, ax=ax, fraction=0.046, pad=0.03); cb.ax.tick_params(labelsize=8, colors=INK2); cb.outline.set_edgecolor(GRID)
        ax.set_title(f"{title}\n{lab}" if i == 0 else lab, loc="left", fontsize=9.5, color=INK)
fig.suptitle("Sky of body 0 at tau_c = 5 (zoom on occluder 1, half field of view = 3 x its angular radius; discs outlined and numbered)", x=0.01, ha="left", fontsize=10.5, color=INK)
fig.tight_layout(rect=[0, 0, 1, 0.96]); fig.savefig(os.path.join(F, "fig3_skymaps.png"), dpi=170); plt.close(fig)
print("figures written to", F)
