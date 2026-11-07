import csv
import json
import math
import os
import shutil
import subprocess
import tempfile

try:  # optional
    import numpy as _np
except Exception:  # pragma: no cover
    _np = None

SCENARIO_FORMAT = "shadowgrav-scenario/1"


class ShadowgravError(RuntimeError):
    """The engine rejected the scenario, or the sgrun executable could not be run."""


def find_sgrun(path=None):
    """Locate the sgrun executable: explicit path, $SHADOWGRAV_SGRUN, PATH, then ./target/release (searching upwards)."""
    cands = [path, os.environ.get("SHADOWGRAV_SGRUN"), shutil.which("sgrun")]
    d = os.getcwd()
    for _ in range(6):
        for n in ("sgrun", "sgrun.exe"):
            cands.append(os.path.join(d, "target", "release", n))
        d = os.path.dirname(d)
    for c in cands:
        if c and os.path.isfile(c):
            return os.path.abspath(c)
    raise ShadowgravError("sgrun not found. Build it with `cargo build --release -p shadowgrav-core --bin sgrun` "
                          "or set SHADOWGRAV_SGRUN to its path.")


def _arr(x):
    return _np.array(x, dtype=float) if _np is not None else x


def scenario_dict(bodies, tau_c=None, kappa0=None, mode="v0", quadrature="standard", velocities=None, masses=None, rhos=None,
                  length_unit="a0", h=1e-4, adaptive=None, steps=0, stride=1, leaf_budget=0, reference=False, tau_max=False,
                  title="", g_n=None, xi=None, v_g=None, alpha_bg=None, w=None, pi=None):
    """Build the scenario dictionary (the JSON file `sgrun run` reads). Unknown arguments are impossible by construction."""
    n = len(bodies)
    bl = []
    for i, x in enumerate(bodies):
        b = {"x": [float(v) for v in x]}
        if velocities is not None:
            b["v"] = [float(v) for v in velocities[i]]
        if masses is not None:
            b["m"] = float(masses[i])
        if rhos is not None:
            b["rho"] = float(rhos[i])
        bl.append(b)
    g = {}
    if tau_c is not None:
        g["tau_c"] = float(tau_c)
    if kappa0 is not None:
        g["kappa0"] = float(kappa0)
    for k, v in (("g_n", g_n), ("xi", xi), ("v_g", v_g), ("alpha_bg", alpha_bg), ("pi", pi)):
        if v is not None:
            g[k] = float(v)
    if w is not None:
        g["w"] = [float(v) for v in w]
    sc = {"format": SCENARIO_FORMAT, "title": title, "mode": mode, "quadrature": quadrature, "length_unit": length_unit,
          "globals": g, "bodies": bl,
          "step": ({"adaptive": adaptive} if adaptive else {"h": float(h)}),
          "run": {"steps": int(steps), "stride": int(stride)}, "leaf_budget": int(leaf_budget),
          "diagnostics": {"reference": bool(reference), "tau_max": bool(tau_max)}}
    assert n == len(bl)
    return sc


class Result(object):
    """Output of one `sgrun run`. Arrays are NumPy arrays when NumPy is installed, else nested lists."""

    def __init__(self, raw, trajectory=None):
        self.raw = raw
        self.provenance = raw["provenance"]
        self.title = raw["title"]
        self.time = raw["time"]
        self.steps_done = raw["steps_done"]
        self.error = raw["error"]
        self.status = raw["status"]
        self.labels = raw["labels"]
        self.energy = raw["energy"]
        self.reference = raw["reference"]
        self.tau_max = raw["tau_max"]
        self.scientific = raw.get("scientific")
        self.warnings = raw.get("warnings", [])      # parameters outside the Spec table ranges
        self.aggregates = raw.get("aggregates")      # tau_c, S, D_bar min/mean/max and total heating power (None = not measured)
        bs = raw["bodies"]
        for key in ("x", "v", "f_total", "f_pair", "f_corr"):
            setattr(self, key, _arr([b[key] for b in bs]))
        self.tau_c = _arr([b["tau_c"] for b in bs])
        self.radius = _arr([b["a"] for b in bs])
        self.delta_na = [b["delta_na"] for b in bs]
        self.delta_sat = [b["delta_sat"] for b in bs]
        self.trajectory = trajectory

    def screening_ratio(self, body=0):
        """|F_total| / |F_pair| on one body (the non-additive screening ratio R of the Spec's test (c))."""
        n = lambda v: math.sqrt(sum(float(c) ** 2 for c in v))
        return n(self.f_total[body]) / n(self.f_pair[body])

    def scientific_status(self):
        """Automatic scientific summary when the geometry is recognised (e.g. collinear test (c))."""
        return self.scientific

    def save_bodies_csv(self, path):
        """Write current body state and forces to a simple CSV (positions in code units)."""
        with open(path, "w") as f:
            f.write("index,x0,x1,x2,v0,v1,v2,ft0,ft1,ft2,fp0,fp1,fp2,fc0,fc1,fc2,tau_c,a,delta_na\n")
            for i in range(len(self.x)):
                row = [i]
                for arr in (self.x, self.v, self.f_total, self.f_pair, self.f_corr):
                    row.extend(arr[i])
                row.extend([self.tau_c[i], self.radius[i], self.delta_na[i]])
                f.write(",".join(str(v) for v in row) + "\n")

    def __repr__(self):
        return "<shadowgrav.Result n=%d steps=%d labels=%s>" % (len(self.tau_c), self.steps_done, self.labels)


def _call(args, sgrun):
    exe = find_sgrun(sgrun)
    p = subprocess.run([exe] + args, stdout=subprocess.PIPE, stderr=subprocess.PIPE, universal_newlines=True)
    return p


def run(*bodies_pos, sgrun=None, trajectory=False, scenario=None, **kw):
    """Run a scenario. Either pass `bodies=[[x,y,z],...]` plus keyword options of `scenario_dict`, or a ready `scenario=dict`.
    trajectory=True also returns the sampled steps in `result.trajectory` (dict of arrays: step, time, body, x, v, f)."""
    if scenario is None:
        bodies = kw.pop("bodies", bodies_pos[0] if bodies_pos else None)
        if bodies is None:
            raise TypeError("run() needs bodies=[[x, y, z], ...] or scenario=dict")
        scenario = scenario_dict(bodies, **kw)
    elif kw or bodies_pos:
        raise TypeError("pass either scenario= or bodies/options, not both")
    with tempfile.TemporaryDirectory() as d:
        sp, op = os.path.join(d, "scenario.json"), os.path.join(d, "traj.csv")
        with open(sp, "w") as f:
            json.dump(scenario, f)
        p = _call(["run", sp, "--out", op], sgrun)
        if p.returncode == 2 or not p.stdout.strip():
            raise ShadowgravError(p.stderr.strip() or "sgrun failed")
        raw = json.loads(p.stdout)
        if p.returncode != 0 and not raw.get("error"):
            raise ShadowgravError(p.stderr.strip() or "sgrun failed")
        traj = None
        if trajectory and os.path.exists(op):
            with open(op) as f:
                rows = list(csv.DictReader(f))
            col = lambda k: [float(r[k]) for r in rows]
            traj = {"step": [int(r["step"]) for r in rows], "time": col("time"), "body": [int(r["body"]) for r in rows]}
            traj["x"] = _arr([[float(r[c]) for c in ("x", "y", "z")] for r in rows])
            traj["v"] = _arr([[float(r[c]) for c in ("vx", "vy", "vz")] for r in rows])
            traj["f"] = _arr([[float(r[c]) for c in ("fx", "fy", "fz")] for r in rows])
            for k in ("step", "time", "body"):
                traj[k] = _arr(traj[k])
    res = Result(raw, traj)
    if res.error:
        raise ShadowgravError("run aborted: %s (state before the failing step is kept by the engine)" % res.error)
    return res


def validate(scenario, sgrun=None):
    """True if the engine accepts the scenario; otherwise raises ShadowgravError with the engine's reason."""
    with tempfile.TemporaryDirectory() as d:
        sp = os.path.join(d, "scenario.json")
        with open(sp, "w") as f:
            json.dump(scenario, f)
        p = _call(["validate", sp], sgrun)
    if p.returncode == 0:
        return True
    msg = p.stderr.strip()
    try:
        msg = json.loads(p.stdout).get("error", msg)
    except Exception:
        pass
    raise ShadowgravError(msg)


def certify_scan(sgrun=None):
    """Run `sgrun certify` (test (c), certified quadrature, about 40 s). Returns (passed, stdout)."""
    p = _call(["certify"], sgrun)
    return p.returncode == 0, p.stdout


def repro_hash(sgrun=None):
    """The platform reproducibility digest (hex string)."""
    p = _call(["repro-hash"], sgrun)
    for line in p.stdout.splitlines():
        if line.startswith("combined"):
            return line.split()[1]
    raise ShadowgravError(p.stderr.strip() or "no digest printed")
