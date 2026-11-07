//! Timing of one force evaluation on a dense random configuration.
//!   cargo run --release -p shadowgrav-core --bin bench -- 50 100 200 400 [--tau 1e-3] [--side 1.0] [--seed 1] [--slow-max 200]
//! Configuration: N spheres (m = 1, rho = RHO0, a = 0.01) uniformly random in a cube of side L (default 1), rejection so
//! that centres are >= 2a apart, at rest, V0, alpha_bg = 0, standard quadrature (f = 0.15). Variants:
//!   serial+brute  = the pre-performance-phase code path (O(m^2) graph per target, one thread)
//!   serial+hash, parallel+brute, parallel+hash (rayon), and thin-fast (NON-CONFORMING, only if tau_c < 1e-2).
//! The conforming variants must agree bitwise; the table prints that check. RAYON_NUM_THREADS limits the thread count.
use shadowgrav_core::driver::*;
use shadowgrav_core::geom::*;
use shadowgrav_core::*;
use std::time::Instant;

struct Rng(u64);
impl Rng { fn u(&mut self) -> f64 { self.0 ^= self.0 << 13; self.0 ^= self.0 >> 7; self.0 ^= self.0 << 17; (self.0 >> 11) as f64 / (1u64 << 53) as f64 } }

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (mut ns, mut tau, mut side, mut seed, mut slow_max) = (Vec::new(), 1e-3, 1.0, 1u64, 200usize);
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--tau" => { tau = args[i + 1].parse().unwrap(); i += 2; }
            "--side" => { side = args[i + 1].parse().unwrap(); i += 2; }
            "--seed" => { seed = args[i + 1].parse().unwrap(); i += 2; }
            "--slow-max" => { slow_max = args[i + 1].parse().unwrap(); i += 2; }
            s => { ns.push(s.parse::<usize>().expect("N")); i += 1; }
        }
    }
    if ns.is_empty() { ns = vec![50, 100, 200, 400]; }
    #[cfg(feature = "parallel")]
    println!("threads: {}", rayon::current_num_threads());
    #[cfg(not(feature = "parallel"))]
    println!("threads: 1 (feature `parallel` off)");
    println!("tau_c = {tau}, box side = {side}, seed = {seed}, a = {:.4}", sphere_radius(1.0, RHO0));
    println!("{:>5} {:>22} {:>12} {:>10} {:>9}", "N", "variant", "ms/eval", "leaves", "bitwise=");
    for &n in &ns {
        let a = sphere_radius(1.0, RHO0);
        let mut rng = Rng(seed.wrapping_mul(0x9E3779B97F4A7C15) | 1);
        let mut xs: Vec<V3> = Vec::new();
        while xs.len() < n {
            let c = [side * rng.u(), side * rng.u(), side * rng.u()];
            if xs.iter().all(|x| norm(&sub(x, &c)) > 2.0 * a) { xs.push(c); }
        }
        let mut g = Globals::defaults();
        g.kappa0 = kappa0_for_tau(tau, 1.0, RHO0);
        let p = body_particles(xs, vec![[0.0; 3]; n]);
        let dv = derive(&p, &g).unwrap();
        let mut reference: Option<Vec<V3>> = None;
        for &(name, par, brute) in &[("serial+brute (old)", false, true), ("serial+hash", false, false),
                                     ("parallel+brute", true, true), ("parallel+hash (new)", true, false)] {
            if !par && n > slow_max && name.contains("brute") { println!("{:>5} {:>22} {:>12}", n, name, "skipped"); continue; }
            let mut o = default_options(Mode::V0);
            o.parallel = par; o.brute_candidates = brute;
            let t0 = Instant::now();
            let f = match evaluate_forces(&p, &dv, &g, &o) { Ok(f) => f, Err(e) => { println!("{:>5} {:>22} error {:?}", n, name, e); continue; } };
            let ms = t0.elapsed().as_secs_f64() * 1e3;
            let same = match &reference { None => { reference = Some(f.f_total.clone()); "ref".to_string() }
                                          Some(r) => (r == &f.f_total).to_string() };
            println!("{:>5} {:>22} {:>12.2} {:>10} {:>9}", n, name, ms, f.status.leaf_count, same);
        }
        if tau < 1e-2 {
            let mut o = default_options(Mode::V0);
            o.thin_fast = true;
            let t0 = Instant::now();
            if let Ok(f) = evaluate_forces(&p, &dv, &g, &o) {
                println!("{:>5} {:>22} {:>12.2} {:>10} {:>9}", n, "thin-fast NON-CONFORMING", t0.elapsed().as_secs_f64() * 1e3, 0, "-");
                let _ = f;
            }
        }
    }
}
