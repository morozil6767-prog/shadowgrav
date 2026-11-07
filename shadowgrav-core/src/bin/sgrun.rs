//! sgrun: headless, scriptable front end to shadowgrav-core. Every output carries provenance (version, commit,
//! compiler, platform, parameters), so a number in a paper can be traced to the exact build that produced it.
//!
//!   sgrun provenance
//!   sgrun certify                          test (c) at tau = 1, 5, 20 with Validation::certified() vs the exact oracle
//!                                          (about 40 s; exit code 1 if any |R - R_exact| > 1e-6)
//!   sgrun scan  [--mode standard|validation|xhi|certified] [--taus 0.1,1,5,20] [--out file.csv]
//!   sgrun orbit [--tau 1e-6] [--steps 1000] [--orbits 5] [--mode v0|v1|v2] [--vg 1e3] [--stride 10] [--out run.csv]
//!   sgrun drag  [--hgamma 0.005] [--decays 5]
//!   sgrun skymap [--case c|c2|c3] [--tau 5] [--target 0] [--focus all|1] [--fov 3] [--validation] --out prefix
//!                                          deficit D and non-additive part D^NA of body `target` as CSV grids (prefix_D.csv, prefix_DNA.csv)
//!   sgrun run <scenario.json> [--out traj.csv] [--steps N]   run a scenario file (see scenario.rs for the format); JSON result on stdout
//!   sgrun validate <scenario.json>         parse and build only; exit 0 if the scenario is valid
//!   sgrun repro-hash [--check HEX]         FNV-1a digest of canonical force/state vectors (platform reproducibility)
//! A JSON summary (with provenance) is always printed to stdout; CSV goes to --out when given.
use shadowgrav_core::driver::*;
use shadowgrav_core::geom::*;
use shadowgrav_core::state::{SimConfig, World};
use shadowgrav_core::*;
use std::collections::HashMap;
use std::fmt::Write as _;
use std::time::Instant;

// ------------------------------------------------------------------ provenance and tiny JSON helpers
fn esc(s: &str) -> String { s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', " ") }
fn provenance() -> String {
    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(0);
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    format!(
        "{{\"tool\":\"sgrun\",\"crate\":\"shadowgrav-core\",\"version\":\"{}\",\"git_commit\":\"{}\",\"git_state\":\"{}\",\"rustc\":\"{}\",\
\"os\":\"{}\",\"arch\":\"{}\",\"features\":{{\"parallel\":{},\"vendored_libm\":{}}},\"threads\":{},\"unix_time\":{},\"args\":\"{}\"}}",
        env!("CARGO_PKG_VERSION"), env!("SG_GIT_COMMIT"), env!("SG_GIT_DIRTY"), esc(env!("SG_RUSTC")),
        std::env::consts::OS, std::env::consts::ARCH, cfg!(feature = "parallel"), cfg!(feature = "vendored-libm"),
        threads, now, esc(&std::env::args().skip(1).collect::<Vec<_>>().join(" ")))
}
fn parse_flags(args: &[String]) -> HashMap<String, String> {
    let mut m = HashMap::new();
    let mut i = 0;
    while i < args.len() {
        if let Some(k) = args[i].strip_prefix("--") {
            let v = args.get(i + 1).filter(|v| !v.starts_with("--")).cloned();
            if v.is_some() { i += 1; }
            m.insert(k.to_string(), v.unwrap_or_default());
        }
        i += 1;
    }
    m
}
fn num(f: &HashMap<String, String>, k: &str, d: f64) -> f64 {
    f.get(k).map(|s| s.parse().unwrap_or_else(|_| die(&format!("--{k}: not a number: {s}")))).unwrap_or(d)
}
fn die(msg: &str) -> ! { eprintln!("sgrun: {msg}"); std::process::exit(2) }
fn mode_of(s: &str) -> Mode { match s { "v0" => Mode::V0, "v1" => Mode::V1, "v2" => Mode::V2, o => die(&format!("unknown mode {o}")) } }
fn validation_of(s: &str) -> Option<Validation> {
    match s {
        "standard" => None,
        "validation" => Some(Validation::default_validation()),
        "xhi" => Some(Validation::xhi()),
        "certified" => Some(Validation::certified()),
        o => die(&format!("unknown quadrature mode {o} (standard|validation|xhi|certified)")),
    }
}

/// Exact R(tau) for test (c): oracle/oracle_screening.py (axisymmetric 1D integral, 30 digits).
fn exact_r(tau: f64) -> Option<f64> {
    [(1.0, 0.877785076145848), (5.0, 0.801867822859093), (20.0, 0.800000002009972)].iter()
        .find(|(t, _)| (*t - tau).abs() < 1e-12).map(|&(_, r)| r)
}

// ------------------------------------------------------------------ subcommands
fn cmd_certify() -> i32 {
    let mut rows = String::new();
    let mut ok = true;
    println!("# test (c), Validation::certified() {:?}", Validation::certified());
    println!("# tau   R                 |R-R_exact|   leaves      cap_hits  seconds");
    for (i, &(tau, r_ex)) in [(1.0, 0.877785076145848), (5.0, 0.801867822859093), (20.0, 0.800000002009972)].iter().enumerate() {
        let mut w = three_body_collinear(tau, 1e-4, 0).unwrap();
        let t = Instant::now();
        w.apply_options(|o| o.validation = Some(Validation::certified())).unwrap();
        let secs = t.elapsed().as_secs_f64();
        let r = screening_ratio(&w);
        let err = (r - r_ex).abs();
        let pass = err <= 1e-6 && w.force.status.cap_hit_count == 0;
        ok &= pass;
        println!("{tau:5} {r:.15} {err:.3e}  {:10}  {:3}  {secs:6.2}  {}", w.force.status.leaf_count, w.force.status.cap_hit_count, if pass { "PASS" } else { "FAIL" });
        if i > 0 { rows.push(','); }
        let _ = write!(rows, "{{\"tau\":{tau},\"R\":{r:.15},\"R_exact\":{r_ex:.15},\"abs_err\":{err:.6e},\"leaves\":{},\"cap_hits\":{},\"seconds\":{secs:.3},\"pass_1e-6\":{pass}}}",
                       w.force.status.leaf_count, w.force.status.cap_hit_count);
    }
    println!("{{\"provenance\":{},\"test\":\"c-certified\",\"preset\":\"{:?}\",\"rows\":[{rows}],\"all_pass\":{ok}}}", provenance(), Validation::certified());
    if ok { 0 } else { 1 }
}

fn cmd_scan(f: &HashMap<String, String>) -> i32 {
    let qmode = f.get("mode").map(String::as_str).unwrap_or("standard");
    let val = validation_of(qmode);
    let taus: Vec<f64> = f.get("taus").map(|s| s.split(',').map(|x| x.parse().unwrap_or_else(|_| die("bad --taus"))).collect())
        .unwrap_or_else(|| vec![1e-3, 1e-2, 0.1, 0.3, 1.0, 3.0, 5.0, 10.0, 20.0, 50.0]);
    let mut csv = String::from("tau,mode,R,R_exact,abs_err,leaves,cap_hits,seconds\n");
    for &tau in &taus {
        let mut w = three_body_collinear(tau, 1e-4, 0).unwrap();
        let t = Instant::now();
        w.apply_options(|o| o.validation = val).unwrap();
        let secs = t.elapsed().as_secs_f64();
        let r = screening_ratio(&w);
        let (rex, err) = match exact_r(tau) { Some(x) => (format!("{x:.15}"), format!("{:.6e}", (r - x).abs())), None => (String::new(), String::new()) };
        let _ = writeln!(csv, "{tau},{qmode},{r:.15},{rex},{err},{},{},{secs:.3}", w.force.status.leaf_count, w.force.status.cap_hit_count);
    }
    emit_csv(f, &csv);
    println!("{{\"provenance\":{},\"command\":\"scan\",\"mode\":\"{qmode}\",\"n_points\":{}}}", provenance(), taus.len());
    0
}

fn emit_csv(f: &HashMap<String, String>, csv: &str) {
    match f.get("out").filter(|s| !s.is_empty()) {
        Some(path) => std::fs::write(path, csv).unwrap_or_else(|e| die(&format!("cannot write {path}: {e}"))),
        None => eprint!("{csv}"),
    }
}

fn cmd_orbit(f: &HashMap<String, String>) -> i32 {
    let tau = num(f, "tau", 1e-6);
    let steps_per = num(f, "steps", 1000.0) as u32;
    let orbits = num(f, "orbits", 5.0) as u32;
    let stride = num(f, "stride", 10.0).max(1.0) as u64;
    let vg = num(f, "vg", 1e3);
    let mode = mode_of(f.get("mode").map(String::as_str).unwrap_or("v0"));
    let s = two_body_orbit(tau, steps_per, mode, vg).unwrap_or_else(|e| die(&format!("setup failed: {e:?}")));
    let (omega, period, h) = (s.omega, s.period, s.h);
    let mut w = s.world;
    let mut csv = String::from("step,t,separation,E,dE_rel,dL_rel,dP_rel,Rp_shadow\n");
    let n = steps_per as u64 * orbits as u64;
    let t0 = Instant::now();
    let line = |w: &World, step: u64, csv: &mut String| {
        let sep = norm(&sub(&w.p.x[1], &w.p.x[0]));
        let _ = writeln!(csv, "{step},{:.12e},{sep:.15e},{:.15e},{:.3e},{:.3e},{:.3e},{:.3e}", w.time(), w.ledger.e_last, w.ledger.de, w.ledger.dl, w.ledger.dp, w.ledger.rp_shadow);
    };
    line(&w, 0, &mut csv);
    for k in 1..=n {
        if let Err(e) = advance(&mut w) { eprintln!("step {k} failed: {e:?}"); return 3; }
        if k % stride == 0 || k == n { line(&w, k, &mut csv); }
    }
    emit_csv(f, &csv);
    println!("{{\"provenance\":{},\"command\":\"orbit\",\"tau_c\":{tau},\"mode\":\"{:?}\",\"v_g\":{vg},\"omega_expected\":{omega:.15e},\"period_expected\":{period:.15e},\"h\":{h:.6e},\
\"steps\":{n},\"max_dE_rel\":{:.3e},\"max_dL_rel\":{:.3e},\"max_dP_rel\":{:.3e},\"max_Rp_shadow\":{:.3e},\"energy_label\":\"{:?}\",\"fp_nonconverged_steps\":{},\"seconds\":{:.2}}}",
             provenance(), mode, w.ledger.max_de, w.ledger.max_dl, w.ledger.max_dp, w.ledger.max_rp_shadow, w.ledger.label,
             w.flags.fp_nonconverged_steps, t0.elapsed().as_secs_f64());
    0
}

fn cmd_drag(f: &HashMap<String, String>) -> i32 {
    let hg = num(f, "hgamma", 0.005);
    let decays = num(f, "decays", 5.0);
    let r = run_drag_decay(hg, decays).unwrap_or_else(|e| die(&format!("drag failed: {e:?}")));
    println!("{{\"provenance\":{},\"command\":\"drag\",\"h_gamma\":{},\"gamma_ref\":{:.15e},\"gamma_fit\":{:.15e},\"rel_err\":{:.3e},\
\"scheme_bias_expected\":{:.3e},\"steps\":{},\"fp_nonconverged\":{}}}",
             provenance(), r.h_gamma, r.gamma_ref, r.gamma_fit, r.rel_err, hg * hg / 12.0, r.steps, r.nonconverged);
    0
}

fn cmd_skymap(f: &HashMap<String, String>) -> i32 {
    let case = f.get("case").map(String::as_str).unwrap_or("c2");
    let tau = num(f, "tau", 5.0);
    let target = num(f, "target", 0.0) as usize;
    let fov = num(f, "fov", 3.0);
    let focus = match f.get("focus").map(String::as_str).unwrap_or("1") { "all" => skymap::SkyFocus::All, s => skymap::SkyFocus::Occluder(s.parse().unwrap_or_else(|_| die("bad --focus"))) };
    let a = sphere_radius(1.0, RHO0);
    let xs: Vec<V3> = match case {
        "c" => vec![[0.0; 3], [10.0, 0.0, 0.0], [20.0, 0.0, 0.0]],
        "c2" => vec![[0.0; 3], [10.0, 0.0, 0.0], [12.0, 2.0, 0.0]],
        "c3" => vec![[0.0; 3], [10.0, 0.0, 0.0], [12.0, 1.4, 1.4]],
        o => die(&format!("unknown --case {o} (c|c2|c3)")),
    }.iter().map(|p| scale(p, a)).collect();
    let mut g = Globals::defaults();
    g.kappa0 = kappa0_for_tau(tau, 1.0, RHO0);
    let mut o = default_options(Mode::V0);
    if f.contains_key("validation") { o.validation = Some(Validation::default_validation()); }
    let w = World::new(body_particles(xs, vec![[0.0; 3]; 3]), g, o, SimConfig::fixed(1e-4)).unwrap();
    let m = skymap::sky_deficit_map(&w, target, focus, fov).unwrap_or_else(|e| die(&e));
    let prefix = f.get("out").filter(|s| !s.is_empty()).cloned().unwrap_or_else(|| die("--out prefix is required"));
    for (suffix, data) in [("D", &m.d_total), ("DNA", &m.d_na)] {
        let mut s = String::new();
        for row in 0..m.nlat {
            let line: Vec<String> = (0..m.nlon).map(|c| { let v = data[row * m.nlon + c]; if v.is_nan() { "nan".into() } else { format!("{v:.6e}") } }).collect();
            s.push_str(&line.join(",")); s.push('\n');
        }
        std::fs::write(format!("{prefix}_{suffix}.csv"), s).unwrap_or_else(|e| die(&format!("write failed: {e}")));
    }
    let discs: Vec<String> = m.discs.iter().map(|d| format!("{{\"j\":{},\"theta\":{:.9e},\"centre\":[{:.9e},{:.9e},{:.9e}]}}", d.j, d.theta, d.centre[0], d.centre[1], d.centre[2])).collect();
    let (proj, c, e1, e2, hf) = match m.proj { skymap::SkyProjection::Zoom { c, e1, e2, half_fov } => ("zoom", c, e1, e2, half_fov), _ => ("mollweide", [0.0; 3], [0.0; 3], [0.0; 3], 0.0) };
    println!("{{\"provenance\":{},\"command\":\"skymap\",\"case\":\"{case}\",\"tau_c\":{tau},\"target\":{target},\"projection\":\"{proj}\",\"half_fov\":{hf:.9e},\"c\":{c:?},\"e1\":{e1:?},\"e2\":{e2:?},\
\"nlon\":{},\"nlat\":{},\"d_max\":{:.6e},\"na_min\":{:.6e},\"d_bar\":{:.6e},\"na_mean\":{:.6e},\"discs\":[{}]}}",
             provenance(), m.nlon, m.nlat, m.d_max, m.na_min, m.d_bar, m.na_mean, discs.join(","));
    0
}

// ------------------------------------------------------------------ reproducibility digest
struct Fnv(u64);
impl Fnv {
    fn new() -> Fnv { Fnv(0xcbf29ce484222325) }
    fn f64(&mut self, x: f64) { for b in x.to_bits().to_le_bytes() { self.0 = (self.0 ^ b as u64).wrapping_mul(0x100000001b3); } }
    fn v3s(&mut self, v: &[V3]) { for p in v { for &c in p { self.f64(c); } } }
}
fn force_digest(w: &World) -> u64 {
    let mut h = Fnv::new();
    h.v3s(&w.force.f_total); h.v3s(&w.force.f_pair); h.v3s(&w.force.f_corr);
    h.0
}
fn cmd_repro(f: &HashMap<String, String>) -> i32 {
    let mut parts: Vec<(String, u64)> = Vec::new();
    // 1. thin-limit orbit: forces, then state after 200 steps
    let mut s = two_body_orbit(1e-6, 1000, Mode::V0, 1e3).unwrap();
    parts.push(("orbit_forces".into(), force_digest(&s.world)));
    for _ in 0..200 { advance(&mut s.world).unwrap(); }
    let mut h = Fnv::new(); h.v3s(&s.world.p.x); h.v3s(&s.world.p.v); parts.push(("orbit_state_200".into(), h.0));
    // 2. saturated collinear screening, standard and validation quadrature
    for (name, val) in [("screen_std", None), ("screen_val", Some(Validation::default_validation()))] {
        let mut w = three_body_collinear(20.0, 1e-4, 0).unwrap();
        w.apply_options(|o| o.validation = val).unwrap();
        parts.push((name.into(), force_digest(&w)));
    }
    // 3. partial (non-nested) overlap, genuinely 3D (oracle case geometry), standard quadrature
    let a = sphere_radius(1.0, RHO0);
    let mut g = Globals::defaults(); g.kappa0 = kappa0_for_tau(5.0, 1.0, RHO0);
    let xs: Vec<V3> = [[0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [12.0, 1.4, 1.4]].iter().map(|p| scale(p, a)).collect();
    let w = World::new(body_particles(xs, vec![[0.0; 3]; 3]), g, default_options(Mode::V0), SimConfig::fixed(1e-4)).unwrap();
    parts.push(("overlap3d_std".into(), force_digest(&w)));
    // 4. velocity-dependent modes
    for (name, mode, vg) in [("orbit_v1", Mode::V1, 1e8), ("orbit_v2", Mode::V2, 1e10)] {
        let mut s = two_body_orbit(1e-3, 200, mode, vg).unwrap();
        for _ in 0..20 { advance(&mut s.world).unwrap(); }
        let mut h = Fnv::new(); h.v3s(&s.world.p.x); h.v3s(&s.world.p.v); h.v3s(&s.world.force.f_total);
        parts.push((name.into(), h.0));
    }
    let mut all = Fnv::new();
    for (_, d) in &parts { for b in d.to_le_bytes() { all.0 = (all.0 ^ b as u64).wrapping_mul(0x100000001b3); } }
    let combined = format!("{:016x}", all.0);
    for (n, d) in &parts { println!("{n:18} {d:016x}"); }
    println!("combined           {combined}");
    println!("{{\"provenance\":{},\"command\":\"repro-hash\",\"combined\":\"{combined}\"}}", provenance());
    if let Some(expect) = f.get("check").filter(|s| !s.is_empty()) {
        if expect.to_lowercase() != combined { eprintln!("MISMATCH: expected {expect}, got {combined}"); return 1; }
        eprintln!("match");
    }
    0
}


// ------------------------------------------------------------------ scenario files
fn jf(x: f64) -> String { if x.is_finite() { format!("{x:e}") } else { "null".to_string() } }
fn jv3(v: &V3) -> String { format!("[{},{},{}]", jf(v[0]), jf(v[1]), jf(v[2])) }
fn jarr<T, F: Fn(&T) -> String>(v: &[T], f: F) -> String { format!("[{}]", v.iter().map(f).collect::<Vec<_>>().join(",")) }
fn load_scenario(args: &[String]) -> shadowgrav_core::scenario::Scenario {
    let path = args.iter().find(|a| !a.starts_with("--")).unwrap_or_else(|| die("missing scenario file"));
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| die(&format!("cannot read {path}: {e}")));
    shadowgrav_core::scenario::Scenario::from_json_text(&text).unwrap_or_else(|e| die(&format!("{path}: {e}")))
}
fn cmd_validate(args: &[String]) -> i32 {
    let sc = load_scenario(args);
    match sc.build() {
        Ok(w) => { println!("{{\"valid\":true,\"bodies\":{},\"mode\":\"{:?}\"}}", w.n(), sc.mode); 0 }
        Err(e) => { println!("{{\"valid\":false,\"error\":\"{}\"}}", esc(&e)); 1 }
    }
}
fn cmd_run(args: &[String], f: &HashMap<String, String>) -> i32 {
    use shadowgrav_core::diagnostics::{snapshot_with, SnapshotOptions};
    let mut sc = load_scenario(args);
    if f.contains_key("steps") { sc.steps = num(f, "steps", 0.0) as u64; }
    let mut w = sc.build().unwrap_or_else(|e| die(&format!("scenario rejected: {e}")));
    let mut csv = String::from("step,time,body,x,y,z,vx,vy,vz,fx,fy,fz\n");
    let dump = |w: &World, csv: &mut String| {
        for k in 0..w.n() {
            let _ = writeln!(csv, "{},{:e},{},{:e},{:e},{:e},{:e},{:e},{:e},{:e},{:e},{:e}", w.step_count, w.time(), k,
                w.p.x[k][0], w.p.x[k][1], w.p.x[k][2], w.p.v[k][0], w.p.v[k][1], w.p.v[k][2],
                w.force.f_total[k][0], w.force.f_total[k][1], w.force.f_total[k][2]);
        }
    };
    let t0 = Instant::now();
    dump(&w, &mut csv);
    let mut err: Option<String> = None;
    for s in 1..=sc.steps {
        if let Err(e) = advance(&mut w) { err = Some(format!("step {s}: {e:?}")); break; }
        if s % sc.stride == 0 || s == sc.steps { dump(&w, &mut csv); }
    }
    let secs = t0.elapsed().as_secs_f64();
    let opts = SnapshotOptions { reference: if sc.want_reference { Some((100, 200)) } else { None }, tau_max: sc.want_tau_max };
    let snap = snapshot_with(&w, &opts);
    let n = w.n();
    let bodies = (0..n).map(|k| {
        let d = snap.targets.iter().find(|t| t.k == k);
        format!("{{\"index\":{k},\"x\":{},\"v\":{},\"m\":{},\"a\":{},\"tau_c\":{},\"f_total\":{},\"f_pair\":{},\"f_corr\":{},\"delta_na\":{},\"delta_sat\":{}}}",
            jv3(&w.p.x[k]), jv3(&w.p.v[k]), jf(w.p.m[k]), jf(w.dv.a[k]), jf(w.dv.tau_c[k]),
            jv3(&w.force.f_total[k]), jv3(&w.force.f_pair[k]), jv3(&w.force.f_corr[k]),
            d.map(|t| jf(t.delta_na)).unwrap_or("null".into()), d.map(|t| jf(t.delta_sat)).unwrap_or("null".into()))
    }).collect::<Vec<_>>().join(",");
    let st = &w.force.status;
    let labels = jarr(&snap.labels, |l| format!("\"{}\"", esc(l)));
    let reference = match &snap.reference {
        None => "null".to_string(),
        Some(r) => format!("{{\"checked\":{},\"max_rel_diff\":{},\"reference_est_err\":{},\"summary\":\"{}\"}}",
            r.outcome.is_ok(), r.max_rel_diff().map(jf).unwrap_or("null".into()), r.reference_est_err().map(jf).unwrap_or("null".into()), esc(&r.summary())),
    };
    let tau_max = match &snap.tau_max {
        None => "null".to_string(),
        Some(v) => jarr(v, |t| format!("{{\"index\":{},\"lower_bound\":{}}}", t.k, jf(t.lower_bound))),
    };
    
    let warnings = jarr(&shadowgrav_core::diagnostics::spec_range_warnings(&w), |s| format!("\"{}\"", esc(s)));
    let aggregates = {
        let a = shadowgrav_core::diagnostics::aggregates(&w);
        let r3 = |r: &Option<shadowgrav_core::diagnostics::Range3>| match r {
            Some(r) => format!("{{\"min\":{},\"mean\":{},\"max\":{}}}", jf(r.min), jf(r.mean), jf(r.max)), None => "null".to_string() };
        format!("{{\"tau_c\":{},\"s\":{},\"d_bar\":{},\"p_heat_total\":{}}}", r3(&a.tau_c), r3(&a.s), r3(&a.d_bar),
                a.p_heat_total.map(jf).unwrap_or("null".into()))
    };
    // Automatic scientific summary for recognised geometries (test (c) collinear screening).
    let scientific = {
        let n = w.n();
        let is_collinear3 = n == 3
            && (w.p.x[0][1].abs() + w.p.x[0][2].abs() + w.p.x[1][1].abs() + w.p.x[1][2].abs()
                + w.p.x[2][1].abs() + w.p.x[2][2].abs()) < 1e-12
            && ((w.p.x[1][0] - w.p.x[0][0]).abs() - (w.p.x[2][0] - w.p.x[1][0]).abs()).abs() < 1e-9 * w.p.x[1][0].abs().max(1.0);
        // R -> 0.8 + O(exp(-tau_c)) only for EQUAL bodies in the saturated regime; at smaller tau_c the exact R(tau) is different
        // (R(1) = 0.8778, R(5) = 0.8019), so comparing with 0.8 there would report a correct run as OUTSIDE.
        let equal = w.p.m.iter().all(|&m| m == w.p.m[0]) && w.p.rho.iter().all(|&r| r == w.p.rho[0]);
        let saturated = w.dv.tau_c.iter().all(|&t| t >= 20.0);
        if is_collinear3 && equal && saturated && norm(&w.force.f_pair[0]) > 1e-300 {
            let r = norm(&w.force.f_total[0]) / norm(&w.force.f_pair[0]);
            let err = (r - 0.8).abs();
            let tier = sc.quadrature.as_str();
            let (_tol, status) = match tier {
                "standard" => (5e-3, if err <= 5e-3 { "WITHIN Standard (provisional 5e-3)" } else { "OUTSIDE Standard" }),
                "validation" => (5e-5, if err <= 5e-5 { "WITHIN Validation expectation" } else { "OUTSIDE Validation" }),
                "xhi" => (5e-6, if err <= 5e-6 { "WITHIN Fine/xhi expectation" } else { "OUTSIDE Fine" }),
                "certified" => (1e-6, if err <= 1e-6 { "WITHIN Certified (Spec 1e-6)" } else { "OUTSIDE Certified" }),
                _ => (f64::NAN, "unknown tier"),
            };
            format!("{{\"geometry\":\"collinear_screening_test_c\",\"R\":{},\"abs_R_minus_0p8\":{},\"tier\":\"{}\",\"status\":\"{}\"}}",
                jf(r), jf(err), esc(tier), esc(status))
        } else {
            "null".to_string()
        }
    };
println!("{{\"provenance\":{},\"command\":\"run\",\"title\":\"{}\",\"mode\":\"{:?}\",\"quadrature\":\"{}\",\
\"steps_done\":{},\"time\":{},\"seconds\":{:.3},\"error\":{},\
\"status\":{{\"leaf_count\":{},\"cap_hit_count\":{},\"max_corr_over_f\":{},\"thin_valid\":{},\"v2_valid\":{},\"overlap_present\":{},\"thin_fast\":{}}},\
\"labels\":{},\"energy\":{},\"momentum\":{},\"angular_momentum\":{},\"energy_label\":\"{:?}\",\"reference\":{},\"tau_max\":{},\"scientific\":{},\"warnings\":{},\"aggregates\":{},\"bodies\":[{}]}}",
        provenance(), esc(&sc.title), sc.mode, esc(&sc.quadrature), w.step_count, jf(w.time()), secs,
        err.as_ref().map(|e| format!("\"{}\"", esc(e))).unwrap_or("null".into()),
        st.leaf_count, st.cap_hit_count, jf(st.max_corr_over_f), st.thin_valid, st.v2_valid, st.overlap_present, st.thin_fast,
        labels, jf(snap.conservation.e), jv3(&snap.conservation.p), jv3(&snap.conservation.l), snap.conservation.label, reference, tau_max, scientific, warnings, aggregates, bodies);
    if let Some(out) = f.get("out").filter(|s| !s.is_empty()) {
        std::fs::write(out, csv).unwrap_or_else(|e| die(&format!("cannot write {out}: {e}")));
    }
    if err.is_some() { 1 } else { 0 }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = args.first().map(String::as_str).unwrap_or("help");
    let flags = parse_flags(&args[args.len().min(1)..]);
    let code = match cmd {
        "provenance" => { println!("{}", provenance()); 0 }
        "certify" => cmd_certify(),
        "scan" => cmd_scan(&flags),
        "orbit" => cmd_orbit(&flags),
        "drag" => cmd_drag(&flags),
        "skymap" => cmd_skymap(&flags),
        "repro-hash" => cmd_repro(&flags),
        "run" => cmd_run(&args[1..], &flags),
        "validate" => cmd_validate(&args[1..]),
        _ => { eprintln!("{}", include_str!("sgrun.rs").lines().take_while(|l| l.starts_with("//!")).map(|l| &l[3..]).collect::<Vec<_>>().join("\n")); if cmd == "help" { 0 } else { 2 } }
    };
    std::process::exit(code);
}
