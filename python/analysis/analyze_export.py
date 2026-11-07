#!/usr/bin/env python3
"""
Analyze a shadowgrav trajectory CSV (from `sgrun orbit --out ...`) or a UI snapshot CSV.

Usage:
  python3 analyze_export.py trajectory.csv [--meta snapshot.meta.json] [--out prefix]

Produces:
  - <prefix>.summary.txt
  - <prefix>.energy.png          (if matplotlib present and columns exist)
  - <prefix>.separation.png
  - <prefix>.model_vs_newton.txt

No Rust knowledge required. See TUTORIAL.md §8.
"""
import argparse
import json
import sys
from pathlib import Path

try:
    import numpy as np
except ImportError:
    print("error: numpy is required (pip install numpy)", file=sys.stderr)
    sys.exit(1)

try:
    import matplotlib
    matplotlib.use("Agg")
    import matplotlib.pyplot as plt
    HAS_MPL = True
except ImportError:
    HAS_MPL = False


def load_trajectory(path: Path):
    try:
        return np.genfromtxt(path, delimiter=",", names=True)
    except Exception as e:
        print(f"error reading {path}: {e}", file=sys.stderr)
        sys.exit(1)


def load_meta(path: Path):
    if path is None or not path.exists():
        return {}
    try:
        with open(path) as f:
            return json.load(f)
    except Exception:
        return {}


def summarize(data, meta, out_prefix: Path):
    lines = [f"Source rows: {len(data)}"]
    names = data.dtype.names or []
    if "t" in names:
        lines.append(f"Time span: {data['t'][0]:.6e} → {data['t'][-1]:.6e}")
    if "E" in names:
        lines.append(f"Initial E: {data['E'][0]:.6e}")
    if "dE_rel" in names:
        lines.append(f"Max |dE_rel|: {np.nanmax(np.abs(data['dE_rel'])):.3e}")
    if "dL_rel" in names:
        lines.append(f"Max |dL_rel|: {np.nanmax(np.abs(data['dL_rel'])):.3e}")
    if "dP_rel" in names:
        lines.append(f"Max |dP_rel|: {np.nanmax(np.abs(data['dP_rel'])):.3e}")
    label = meta.get("energy_label") or meta.get("energy", {}).get("label")
    if label:
        lines.append(f"Energy label: {label}")
    summary_path = out_prefix.with_suffix(".summary.txt")
    summary_path.write_text("\n".join(lines) + "\n")
    print(f"wrote {summary_path}")
    return lines


def plot_energy(data, out_prefix: Path):
    if not HAS_MPL or "t" not in (data.dtype.names or []) or "dE_rel" not in (data.dtype.names or []):
        return
    fig, ax = plt.subplots(figsize=(8, 4))
    ax.plot(data["t"], data["dE_rel"], lw=1.0)
    ax.set_xlabel("t")
    ax.set_ylabel(r"$\Delta E / |E|$")
    ax.set_title("Relative energy drift")
    ax.grid(True, alpha=0.3)
    fig.tight_layout()
    p = out_prefix.with_suffix(".energy.png")
    fig.savefig(p, dpi=140)
    plt.close(fig)
    print(f"wrote {p}")


def plot_separation(data, out_prefix: Path):
    if not HAS_MPL or "t" not in (data.dtype.names or []) or "separation" not in (data.dtype.names or []):
        return
    fig, ax = plt.subplots(figsize=(8, 4))
    ax.plot(data["t"], data["separation"], lw=1.0)
    ax.set_xlabel("t")
    ax.set_ylabel("separation")
    ax.set_title("Inter-body separation")
    ax.grid(True, alpha=0.3)
    fig.tight_layout()
    p = out_prefix.with_suffix(".separation.png")
    fig.savefig(p, dpi=140)
    plt.close(fig)
    print(f"wrote {p}")


def newton_comparison(data, out_prefix: Path):
    lines = [
        "Model vs Newtonian expectation (diagnostic only)",
        "-----------------------------------------------",
        "Meaningful only for thin, well-separated, V0, α_bg=0 runs.",
        "Any residual is finite-size / saturation / numerical contribution of the shadow model.",
        "See MODEL_VS_OBSERVATION.md for the broader observational context.",
        "",
    ]
    names = data.dtype.names or []
    if "separation" in names and "t" in names and len(data) > 4:
        sep = data["separation"]
        peaks = np.where((sep[1:-1] > sep[:-2]) & (sep[1:-1] > sep[2:]))[0] + 1
        if len(peaks) >= 2:
            dt = float(data["t"][peaks[-1]] - data["t"][peaks[0]])
            period_est = dt / (len(peaks) - 1)
            lines.append(f"Estimated orbital period from separation peaks: {period_est:.6e}")
        else:
            lines.append("Not enough separation peaks to estimate period.")
    else:
        lines.append("No usable separation/time columns.")
    path = out_prefix.with_suffix(".model_vs_newton.txt")
    path.write_text("\n".join(lines) + "\n")
    print(f"wrote {path}")


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("csv", type=Path)
    ap.add_argument("--meta", type=Path, default=None)
    ap.add_argument("--out", type=Path, default=None)
    args = ap.parse_args()
    if args.out is None:
        args.out = args.csv.with_suffix("")
    data = load_trajectory(args.csv)
    meta = load_meta(args.meta)
    summarize(data, meta, args.out)
    plot_energy(data, args.out)
    plot_separation(data, args.out)
    newton_comparison(data, args.out)
    if not HAS_MPL:
        print("note: matplotlib not installed — plots skipped (pip install matplotlib)", file=sys.stderr)


if __name__ == "__main__":
    main()
