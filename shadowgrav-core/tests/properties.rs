//! WS6: property tests and robustness (fuzz-style) tests, std only (no proptest / cargo-fuzz in the offline build).
//!
//! Properties are checked on seeded pseudo-random geometries (xorshift64*, fixed seeds, so a failure reproduces exactly) and each
//! test prints the worst measured value next to its bound. Robustness: hostile numbers and byte-mutated scenario files must produce
//! `Err` or a finite result, never a panic, a hang or a non-finite force.
//! These are NOT a substitute for coverage-guided fuzzing (cargo-fuzz) or sanitizers; see WS6_VALIDATION.md.
use shadowgrav_core::driver::*;
use shadowgrav_core::geom::*;
use shadowgrav_core::scenario::Scenario;
use shadowgrav_core::state::*;
use shadowgrav_core::*;

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545F4914F6CDD1D)
    }
    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
    fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.unit()
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
    fn direction(&mut self) -> V3 {
        loop {
            let v = [
                self.range(-1.0, 1.0),
                self.range(-1.0, 1.0),
                self.range(-1.0, 1.0),
            ];
            let n = norm(&v);
            if n > 0.1 && n <= 1.0 {
                return scale(&v, 1.0 / n);
            }
        }
    }
}

fn world(pos: &[V3], tau: f64, validation: Option<Validation>) -> Result<World, SimError> {
    let a = sphere_radius(1.0, RHO0);
    let mut g = Globals::defaults();
    g.kappa0 = kappa0_for_tau(tau, 1.0, RHO0);
    let xs: Vec<V3> = pos.iter().map(|p| scale(p, a)).collect();
    let mut o = default_options(Mode::V0);
    o.validation = validation;
    World::new(
        body_particles(xs.clone(), vec![[0.0; 3]; xs.len()]),
        g,
        o,
        SimConfig::fixed(1e-4),
    )
}

/// n bodies at least `min_sep` radii apart inside a cube of side `box_side` radii.
fn random_cluster(r: &mut Rng, n: usize, box_side: f64, min_sep: f64) -> Vec<V3> {
    let mut pos: Vec<V3> = Vec::new();
    while pos.len() < n {
        let c = [
            r.range(0.0, box_side),
            r.range(0.0, box_side),
            r.range(0.0, box_side),
        ];
        if pos.iter().all(|p| norm(&sub(p, &c)) > min_sep) {
            pos.push(c);
        }
    }
    pos
}

fn max_rel(a: &[V3], b: &[V3]) -> f64 {
    let s = a.iter().map(norm).fold(0.0, f64::max);
    a.iter()
        .zip(b)
        .map(|(x, y)| norm(&sub(x, y)))
        .fold(0.0, f64::max)
        / s
}

fn rotation(r: &mut Rng) -> impl Fn(&V3) -> V3 {
    let k = r.direction();
    let th = r.range(0.2, 3.0);
    let (c, s) = (th.cos(), th.sin());
    move |v: &V3| {
        // Rodrigues
        let kv = cross(&k, v);
        let kd = dot(&k, v);
        add(
            &add(&scale(v, c), &scale(&kv, s)),
            &scale(&k, kd * (1.0 - c)),
        )
    }
}

/// Permutation (summation order only), translation and rotation on 12 random clusters at three optical depths.
#[test]
fn random_clusters_obey_permutation_translation_and_rotation_covariance() {
    let mut r = Rng(0x5EED_0001_2026_1107);
    let (mut wp, mut wt, mut wr) = (0f64, 0f64, 0f64);
    for case in 0..12 {
        let n = 3 + r.below(4);
        let tau = [0.5, 2.0, 5.0][case % 3];
        let pos = random_cluster(&mut r, n, 9.0, 2.3);
        let w1 = world(&pos, tau, None).unwrap();
        // permutation: cyclic shift
        let perm: Vec<usize> = (0..n).map(|i| (i + 1 + case % (n - 1)) % n).collect();
        let pos_p: Vec<V3> = perm.iter().map(|&i| pos[i]).collect();
        let w2 = world(&pos_p, tau, None).unwrap();
        let f2: Vec<V3> = (0..n)
            .map(|j| w2.force.f_total[perm.iter().position(|&i| i == j).unwrap()])
            .collect();
        wp = wp.max(max_rel(&w1.force.f_total, &f2));
        // translation by a multiple of the radius
        let off = scale(&r.direction(), r.range(20.0, 200.0));
        let pos_t: Vec<V3> = pos.iter().map(|p| add(p, &off)).collect();
        let w3 = world(&pos_t, tau, None).unwrap();
        wt = wt.max(max_rel(&w1.force.f_total, &w3.force.f_total));
        // rotation about a random axis
        let rot = rotation(&mut r);
        let pos_r: Vec<V3> = pos.iter().map(&rot).collect();
        let w4 = world(&pos_r, tau, None).unwrap();
        let f1r: Vec<V3> = w1.force.f_total.iter().map(&rot).collect();
        wr = wr.max(max_rel(&f1r, &w4.force.f_total));
    }
    eprintln!("[properties] random clusters (standard tier): permutation {wp:e}, translation {wt:e}, rotation {wr:e}");
    assert!(wp < 1e-9, "permutation {wp:e}");
    assert!(wt < 5e-3, "translation {wt:e}");
    assert!(wr < 5e-3, "rotation {wr:e}");
}

/// For occluders collinear with the target, every disc is centred on the axis, cos(psi) >= 0 inside each disc and D grows pointwise
/// when an occluder is added; hence |F_0| is nondecreasing as bodies are added and never exceeds the sum of the pair forces.
/// Tolerance: validation-tier quadrature error (measured below).
#[test]
fn collinear_stacks_are_monotone_in_occluders_and_bounded_by_the_pair_sum() {
    let mut r = Rng(0x5EED_0002_2026_1107);
    let tol = 5e-4;
    let (mut worst_mono, mut worst_sum) = (f64::NEG_INFINITY, f64::NEG_INFINITY);
    for case in 0..10 {
        let tau = [0.5, 2.0, 5.0, 20.0][case % 4];
        let axis = r.direction();
        let mut d = r.range(3.0, 6.0);
        let mut pos: Vec<V3> = vec![[0.0; 3]];
        let mut pair = Vec::new();
        let mut totals = Vec::new();
        for _ in 0..3 {
            pos.push(scale(&axis, d));
            let p2 = world(
                &[[0.0; 3], scale(&axis, d)],
                tau,
                Some(Validation::default_validation()),
            )
            .unwrap();
            pair.push(norm(&p2.force.f_total[0]));
            let w = world(&pos, tau, Some(Validation::default_validation())).unwrap();
            totals.push(norm(&w.force.f_total[0]));
            d += r.range(2.5, 6.0);
        }
        for k in 1..totals.len() {
            let mono = (totals[k - 1] - totals[k]) / totals[k - 1];
            worst_mono = worst_mono.max(mono);
            assert!(
                mono < tol,
                "case {case}: adding body {k} reduced |F_0| by {mono:e} (tau {tau})"
            );
        }
        for k in 0..totals.len() {
            let s: f64 = pair[..=k].iter().sum();
            let over = (totals[k] - s) / s;
            worst_sum = worst_sum.max(over);
            assert!(
                over < tol,
                "case {case}: |F_0| exceeds the pair sum by {over:e} (tau {tau}, k {k})"
            );
        }
    }
    eprintln!("[properties] collinear stacks: worst decrease {worst_mono:e}, worst excess over pair sum {worst_sum:e} (tolerance {tol:e})");
}

/// Reciprocity of the pair part: for exactly two bodies F_0 = -F_1 for every separation and optical depth.
#[test]
fn two_body_force_is_exactly_reciprocal_over_random_separations() {
    let mut r = Rng(0x5EED_0003_2026_1107);
    let mut worst = 0f64;
    for _ in 0..40 {
        let tau = 10f64.powf(r.range(-3.0, 1.4));
        let d = r.range(2.05, 40.0);
        let w = world(&[[0.0; 3], scale(&r.direction(), d)], tau, None).unwrap();
        let s = add(&w.force.f_total[0], &w.force.f_total[1]);
        worst = worst.max(norm(&s) / norm(&w.force.f_total[0]));
    }
    eprintln!("[properties] two-body reciprocity: worst |F0+F1|/|F0| = {worst:e}");
    assert!(worst < 1e-12, "{worst:e}");
}

// ------------------------------------------------------------------------------------------------ robustness

/// Hostile numbers through the public constructors: Err or a finite, valid result. A panic fails the test.
#[test]
fn hostile_numbers_are_rejected_or_finite_never_a_panic() {
    let a = sphere_radius(1.0, RHO0);
    let hostile = [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        0.0,
        -0.0,
        -1.0,
        1e-300,
        1e300,
        f64::MAX,
        f64::MIN_POSITIVE,
        5e-324,
    ];
    let mut tried = 0;
    let mut accepted = 0;
    for &h in &hostile {
        // tau_c / kappa0
        for variant in 0..3 {
            let mut g = Globals::defaults();
            g.kappa0 = match variant {
                0 => h,
                1 => kappa0_for_tau(1.0, 1.0, RHO0) * h,
                _ => kappa0_for_tau(1.0, 1.0, RHO0) + h,
            };
            let w = World::new(
                body_particles(vec![[0.0; 3], [10.0 * a, 0.0, 0.0]], vec![[0.0; 3]; 2]),
                g,
                default_options(Mode::V0),
                SimConfig::fixed(1e-4),
            );
            tried += 1;
            if let Ok(w) = w {
                accepted += 1;
                assert!(
                    w.force.f_total.iter().flatten().all(|x| x.is_finite()),
                    "kappa0 variant {variant} h {h:e}"
                );
            }
        }
        // each particle field in turn
        for field in 0..5 {
            let mut p = body_particles(
                vec![[0.0; 3], [10.0 * a, 0.0, 0.0], [0.0, 12.0 * a, 0.0]],
                vec![[0.0; 3]; 3],
            );
            match field {
                0 => p.x[1][0] = h,
                1 => p.v[1][1] = h,
                2 => p.m[2] = h,
                3 => p.rho[0] = h,
                _ => p.delta[1] = h,
            }
            let mut g = Globals::defaults();
            g.kappa0 = kappa0_for_tau(2.0, 1.0, RHO0);
            let w = World::new(p, g, default_options(Mode::V0), SimConfig::fixed(1e-4));
            tried += 1;
            if let Ok(w) = w {
                accepted += 1;
                assert!(
                    w.force.f_total.iter().flatten().all(|x| x.is_finite()),
                    "field {field} h {h:e}"
                );
            }
        }
        // time step
        let mut g = Globals::defaults();
        g.kappa0 = kappa0_for_tau(2.0, 1.0, RHO0);
        let w = World::new(
            body_particles(vec![[0.0; 3], [10.0 * a, 0.0, 0.0]], vec![[0.0; 3]; 2]),
            g,
            default_options(Mode::V0),
            SimConfig::fixed(h),
        );
        tried += 1;
        if w.is_ok() {
            accepted += 1;
        }
    }
    eprintln!("[properties] hostile numbers: {tried} constructions, {accepted} accepted (all with finite forces), {} rejected", tried - accepted);
    assert!(
        accepted < tried,
        "nothing hostile was rejected: the validators are not engaged"
    );
}

fn mutate(r: &mut Rng, src: &str) -> String {
    const TOKENS: [&str; 22] = [
        "NaN",
        "Infinity",
        "-Infinity",
        "1e999",
        "-1e999",
        "1e-999",
        "0",
        "-1",
        "null",
        "true",
        "[",
        "]",
        "{",
        "}",
        "\"",
        ",",
        ":",
        "[[]]",
        "\"\"",
        "00",
        "0x10",
        "1e",
    ];
    let mut b: Vec<u8> = src.as_bytes().to_vec();
    for _ in 0..1 + r.below(3) {
        if b.is_empty() {
            break;
        }
        match r.below(5) {
            0 => {
                let i = r.below(b.len());
                b[i] = (r.next() & 0x7f) as u8;
            }
            1 => {
                let i = r.below(b.len());
                let l = 1 + r.below(12.min(b.len() - i));
                b.drain(i..i + l);
            }
            2 => {
                let i = r.below(b.len() + 1);
                let t = TOKENS[r.below(TOKENS.len())].as_bytes().to_vec();
                for (k, c) in t.into_iter().enumerate() {
                    b.insert(i + k, c);
                }
            }
            3 => {
                let i = r.below(b.len());
                b.truncate(i);
            }
            _ => {
                // overwrite a numeric-looking run with a hostile token
                if let Some(i) = b
                    .iter()
                    .position(|c| c.is_ascii_digit())
                    .map(|p| p + r.below(b.len() - p))
                {
                    let t = TOKENS[r.below(TOKENS.len())].as_bytes();
                    let j = i.min(b.len());
                    let end = (j + 1 + r.below(4)).min(b.len());
                    b.splice(j..end, t.iter().copied());
                }
            }
        }
    }
    String::from_utf8_lossy(&b).into_owned()
}

/// Byte-level mutation of every shipped scenario file: parse and build must return a value, never panic; a built world has finite forces.
/// The seed list is fixed; the first failing input is printed in full.
#[test]
fn mutated_scenario_files_never_panic_the_parser_or_the_builder() {
    let dir = format!("{}/scenarios", env!("CARGO_MANIFEST_DIR"));
    let mut sources = Vec::new();
    for e in std::fs::read_dir(&dir).unwrap() {
        let p = e.unwrap().path();
        if p.extension().and_then(|s| s.to_str()) == Some("json") {
            sources.push((
                p.display().to_string(),
                std::fs::read_to_string(&p).unwrap(),
            ));
        }
    }
    sources.sort();
    assert!(
        sources.len() >= 6,
        "expected the shipped scenarios in {dir}"
    );
    let mut r = Rng(0x5EED_0004_2026_1107);
    let (mut parsed, mut built, mut total) = (0usize, 0usize, 0usize);
    let per_file = 1500;
    for (name, src) in &sources {
        for _ in 0..per_file {
            let m = mutate(&mut r, src);
            total += 1;
            let res = std::panic::catch_unwind(|| match Scenario::from_json_text(&m) {
                Ok(sc) => {
                    // keep the build cheap: only build small standard-tier worlds
                    if sc.bodies.len() <= 8 && sc.validation.is_none() {
                        match sc.build() {
                            Ok(w) => (1, w.force.f_total.iter().flatten().all(|x| x.is_finite())),
                            Err(_) => (1, true),
                        }
                    } else {
                        (1, true)
                    }
                }
                Err(_) => (0, true),
            });
            match res {
                Ok((p, finite)) => {
                    parsed += p;
                    if p == 1 {
                        built += 1;
                    }
                    assert!(finite, "{name}: non-finite force from mutated input:\n{m}");
                }
                Err(_) => panic!("{name}: PANIC on mutated input:\n{m}"),
            }
        }
    }
    eprintln!("[properties] scenario fuzz: {total} mutated inputs over {} files, {parsed} still parsed, {} rejected", sources.len(), total - parsed);
    let _ = built;
    assert!(
        parsed > 0 && parsed < total,
        "mutation did not exercise both outcomes"
    );
}
