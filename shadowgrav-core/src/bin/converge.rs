//! Convergence study for test (c): |R - R_exact| versus refinement, against the independent oracle
//! (oracle/oracle_screening.py, 30-digit axisymmetric integral). A measurement, not a verdict.
//!   cargo run --release -p shadowgrav-core --bin converge            (default sweep, ~1 minute)
//!   cargo run --release -p shadowgrav-core --bin converge -- quick    (stops below 20M leaves)
use shadowgrav_core::driver::*;
use shadowgrav_core::types::Validation;
use std::time::Instant;

/// Exact R(tau) from oracle_screening.py.
const EXACT: [(f64, f64); 3] = [
    (1.0, 0.877785076145848),
    (5.0, 0.801867822859093),
    (20.0, 0.800000002009972),
];

fn main() {
    let quick = std::env::args().any(|a| a == "quick");
    println!("# tau  f_v  f_s  ell_cap  leaves  cap_hits  seconds  R  |R-R_exact|");
    let sweeps: Vec<(f64, f64, u32)> = vec![
        (0.15, 0.15, 12),
        (0.075, 0.075, 12),
        (0.0375, 0.0375, 13),
        (0.02, 4e-4, 14),
        (0.01, 4e-4, 14),
        (0.005, 4e-4, 14),
        (0.002, 4e-4, 14),
        (0.0015, 4e-4, 15),
        (0.001, 4e-4, 15),
        (0.0007, 4e-4, 15),
        (0.0005, 4e-4, 15),
        (0.0007, 4e-4, 16),
        (0.0005, 4e-4, 16),
    ];
    let mut prev: Option<(f64, f64)> = None;
    for &(tau, r_ex) in EXACT.iter().rev() {
        for &(fv, fs, cap) in &sweeps {
            let mut w = three_body_collinear(tau, 1e-4, 0).unwrap();
            let t = Instant::now();
            if fv == fs && fv >= 0.0375 {
                // "standard" rows: the standard quadrature with these f (no validation block)
                w.apply_options(|o| o.validation = None).unwrap();
                let mut g = w.g.clone();
                g.f = fv;
                g.ell_cap = cap;
                w.apply_globals(|gg| {
                    *gg = g.clone();
                })
                .unwrap();
            } else {
                w.apply_options(|o| {
                    o.validation = Some(Validation {
                        f_v: fv,
                        f_s: fs,
                        ell_cap: cap,
                    })
                })
                .unwrap();
            }
            let secs = t.elapsed().as_secs_f64();
            let r = screening_ratio(&w);
            let leaves = w.force.status.leaf_count;
            if quick && leaves > 20_000_000 {
                continue;
            }
            let err = (r - r_ex).abs();
            let slope = match prev {
                Some((l0, e0)) if tau == 20.0 && leaves as f64 > l0 && e0 > 0.0 && err > 0.0 => {
                    format!(
                        "  local order {:.2}",
                        (e0 / err).ln() / (leaves as f64 / l0).ln()
                    )
                }
                _ => String::new(),
            };
            println!("{tau:4} {fv:8} {fs:7.0e} {cap:3} {leaves:10} {:7} {secs:7.2} {r:.12} {err:.3e}{slope}", w.force.status.cap_hit_count);
            if tau == 20.0 {
                prev = Some((leaves as f64, err));
            }
        }
        prev = None;
    }
}
