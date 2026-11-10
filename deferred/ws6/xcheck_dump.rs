//! Cross-check dump (WS6): evaluates the engine's V0, alpha_bg = 0 force on an arbitrary body set and prints the
//! dimensionless vectors F/Gamma_k (total, pair, correction) so that `oracle/xcheck_random.py` can compare them with an
//! independent sky integral. Reads stdin:   line 1: `<tier> <tau0>`   tier = standard|validation|xhi|certified|graded,
//! tau0 = tau_c of a body with m = 1, rho = RHO0 (fixes kappa0);   line 2: n;   then n lines `x y z m rho`.
//! Prints `GLOBALS`, one `BODY` line per body and one `STATUS` line, all with 17 significant digits; on an engine
//! error prints `ERROR <debug text>` and exits 0 (a refusal is a result, not a crash).
use shadowgrav_core::driver::*;
use shadowgrav_core::state::*;
use shadowgrav_core::*;
use std::f64::consts::PI;
use std::io::Read;

fn main() {
    let mut text = String::new();
    std::io::stdin().read_to_string(&mut text).expect("stdin");
    let mut it = text.split_whitespace();
    let tier = it.next().expect("tier").to_string();
    let tau0: f64 = it.next().expect("tau0").parse().expect("tau0");
    let n: usize = it.next().expect("n").parse().expect("n");
    let mut p = body_particles(vec![[0.0; 3]; n], vec![[0.0; 3]; n]);
    for i in 0..n {
        let mut f = || -> f64 { it.next().expect("field").parse().expect("number") };
        p.x[i] = [f(), f(), f()];
        p.m[i] = f();
        p.rho[i] = f();
    }
    let mut g = Globals::defaults();
    g.kappa0 = kappa0_for_tau(tau0, 1.0, RHO0);
    let mut o = default_options(Mode::V0);
    o.leaf_budget = 200_000_000;
    o.validation = match tier.as_str() {
        "standard" => None,
        "validation" => Some(Validation::default_validation()),
        "xhi" => Some(Validation::xhi()),
        "certified" => Some(Validation::certified()),
        "graded" => Some(Validation::graded()),
        other => {
            println!("ERROR unknown tier {other}");
            return;
        }
    };
    let w = match World::new(p, g, o, SimConfig::fixed(1e-4)) {
        Ok(w) => w,
        Err(e) => {
            println!("ERROR {e:?}");
            return;
        }
    };
    println!(
        "GLOBALS {:.17e} {:.17e} {:.17e}",
        w.g.kappa0,
        w.g.xi,
        w.g.pi_flux()
    );
    for k in 0..n {
        let gam = w.g.xi * w.g.pi_flux() * w.dv.a_eff[k] / (4.0 * PI);
        let t = &w.force.f_total[k];
        let pr = &w.force.f_pair[k];
        let c = &w.force.f_corr[k];
        println!(
            "BODY {k} {:.17e} {:.17e} {:.17e} {:.17e} {:.17e} {:.17e} {:.17e} {:.17e} {:.17e} {:.17e} {:.17e} {:.17e} {:.17e} {:.17e}",
            w.dv.a[k], w.dv.beta[k], w.dv.tau_c[k], w.dv.a_eff[k],
            t[0] / gam, t[1] / gam, t[2] / gam,
            pr[0] / gam, pr[1] / gam, pr[2] / gam,
            c[0] / gam, c[1] / gam, c[2] / gam,
            gam
        );
    }
    let s = &w.force.status;
    println!(
        "STATUS {} {} {}",
        s.leaf_count, s.cap_hit_count, s.overlap_present
    );
}
