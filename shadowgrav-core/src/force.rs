//! Driver: eq. 4 assembly (+ eq. 8 drag, Addendum I.2 aberration). Pure function.
use crate::geom::*;
use crate::mathx;
use crate::overlap::*;
use crate::pair::*;
use crate::sum::Neum3;
use crate::types::*;
use crate::velocity::f_dr;
#[cfg(feature = "parallel")]
use rayon::prelude::*;
use std::f64::consts::PI;

fn finite(v: &V3) -> bool {
    v.iter().all(|x| x.is_finite())
}

/// Everything one target needs, read-only (so targets can run on any thread).
struct Shared<'a> {
    p: &'a Particles,
    dv: &'a Derived,
    g: &'a Globals,
    o: &'a Options,
    q: &'a Quad,
    eps: &'a [V3],
    bodies: &'a [usize],
    v2: bool,
    vel: bool,
    pi_flux: f64,
    ell_cap: u32,
}
#[derive(Default)]
struct Scratch {
    nodes: Vec<Node>,
    nodes2: Vec<Node>,
}
struct TargetOut {
    fp: V3,
    fc: V3,
    fdr: V3,
    fab: V3,
    ft: V3,
    pairs: Vec<PairMoment>,
    leaves: Vec<LeafRecord>,
    leaf_count: u64,
    cap_hits: u64,
    overlap: bool,
    interior: bool,
}

pub fn evaluate_forces(
    p: &Particles,
    dv: &Derived,
    g: &Globals,
    o: &Options,
) -> Result<ForceOutput, CoreError> {
    p.validate()?;
    g.validate()?;
    o.validate()?;
    let n = p.x.len();
    dv.validate_shape(n)?;
    let v2 = o.mode == Mode::V2;
    let vel = o.mode != Mode::V0;
    if v2 && g.alpha_bg != 0.0 {
        return Err(CoreError::Gating("V2 requires alpha_bg = 0"));
    }
    if o.thin_fast {
        return crate::thin_fast::evaluate(p, dv, g, o);
    } // NON-CONFORMING preview path
    let pi_flux = g.pi_flux();
    // Probes are static by Contract (N6): eps = 0 in every mode.
    let eps: Vec<V3> = (0..n)
        .map(|i| {
            if vel && p.kind[i] == Kind::Body {
                scale(&sub(&p.v[i], &g.w), 1.0 / g.v_g)
            } else {
                [0.0; 3]
            }
        })
        .collect();
    // Spec 6.12 "V2-valid (max |eps| <= 0.1)" is a property of the velocities, so it is evaluated in EVERY mode (in V0/V1 it says
    // whether switching to V2 would be admissible); only V2 refuses to run when it fails.
    let mut emax: f64 = 0.0;
    for i in 0..n {
        if p.kind[i] == Kind::Body {
            emax = emax.max(norm(&scale(&sub(&p.v[i], &g.w), 1.0 / g.v_g)));
        }
    }
    if v2 && emax > 0.1 {
        return Err(CoreError::Gating("V2 requires max |eps| <= 0.1"));
    }

    let bodies: Vec<usize> = (0..n).filter(|&i| p.kind[i] == Kind::Body).collect();
    let q = Quad::new(g.n_gl);
    let ell_cap = o.validation.map(|v| v.ell_cap).unwrap_or(g.ell_cap);
    let sh = Shared {
        p,
        dv,
        g,
        o,
        q: &q,
        eps: &eps,
        bodies: &bodies,
        v2,
        vel,
        pi_flux,
        ell_cap,
    };

    // Targets are independent: each evaluate is a pure function of read-only inputs, results are collected in index
    // order, so parallel == serial bit for bit (tests/perf_equiv.rs).
    let results: Vec<Result<TargetOut, CoreError>> = run_targets(&sh, n);

    let mut out = ForceOutput {
        f_pair: vec![[0.0; 3]; n],
        f_corr: vec![[0.0; 3]; n],
        f_dr: vec![[0.0; 3]; n],
        f_ab: vec![[0.0; 3]; n],
        f_total: vec![[0.0; 3]; n],
        status: Status::default(),
        sinks: if o.instrumented {
            Some(Sinks::default())
        } else {
            None
        },
    };
    out.status.thin_valid = bodies.iter().all(|&i| dv.tau_c[i] < 0.1);
    out.status.v2_valid = emax <= 0.1;
    for (k, r) in results.into_iter().enumerate() {
        let t = r?; // first error in target order (deterministic)
        out.status.leaf_count += t.leaf_count;
        out.status.cap_hit_count += t.cap_hits;
        out.status.overlap_present |= t.overlap;
        out.status.interior_target_present |= t.interior;
        let fsh = norm(&add(&t.fp, &t.fc));
        if fsh > 0.0 {
            out.status.max_corr_over_f = out.status.max_corr_over_f.max(norm(&t.fc) / fsh);
        }
        out.f_pair[k] = t.fp;
        out.f_corr[k] = t.fc;
        out.f_dr[k] = t.fdr;
        out.f_ab[k] = t.fab;
        out.f_total[k] = t.ft;
        if let Some(s) = out.sinks.as_mut() {
            s.pairs.extend(t.pairs);
            s.leaves.extend(t.leaves);
        }
    }
    Ok(out)
}

fn run_targets(sh: &Shared, n: usize) -> Vec<Result<TargetOut, CoreError>> {
    #[cfg(feature = "parallel")]
    {
        if sh.o.parallel {
            return (0..n)
                .into_par_iter()
                .map_init(Scratch::default, |s, k| eval_target(sh, k, s))
                .collect();
        }
    }
    let mut s = Scratch::default();
    (0..n).map(|k| eval_target(sh, k, &mut s)).collect()
}

fn eval_target(sh: &Shared, k: usize, scr: &mut Scratch) -> Result<TargetOut, CoreError> {
    let (p, dv, g, o, q) = (sh.p, sh.dv, sh.g, sh.o, sh.q);
    let (v2, vel, pi_flux, ell_cap) = (sh.v2, sh.vel, sh.pi_flux, sh.ell_cap);
    let eps_k = sh.eps[k];
    let gamma = g.xi * pi_flux * dv.a_eff[k] / (4.0 * PI); // Gamma_k (eq. 3)
    let mut occ: Vec<Occ> = Vec::with_capacity(sh.bodies.len());
    let mut sum_pair = Neum3::default();
    let mut sum_b = Neum3::default();
    let mut pairs: Vec<PairMoment> = Vec::new();
    let mut interior = false;

    // ---- pair stage: first sum of eq. 4 ----
    for &j in sh.bodies {
        if j == k {
            continue;
        }
        let c = sub(&p.x[j], &p.x[k]);
        let d = norm(&c);
        let a = dv.a[j];
        let (chat, theta) = if d < 1e-12 * a {
            ([0.0, 0.0, 1.0], PI)
        }
        // Spec 1.6
        else {
            (
                scale(&c, 1.0 / d),
                if d > a {
                    mathx::atan2(a, ((d - a) * (d + a)).sqrt())
                } else {
                    PI
                },
            )
        }; // 2.3
        if d <= a {
            interior = true;
        }
        let pj = pair_p(
            PairSpec {
                d,
                a,
                beta: dv.beta[j],
                alpha: g.alpha_bg,
                a_eff: dv.a_eff[j],
                force_gl: o.force_gl,
            },
            q,
            &mut scr.nodes,
        );
        sum_pair.add_scaled(pj, &chat);
        if o.instrumented {
            let m = pair_moments(d, a, dv.beta[j], g.alpha_bg, q, &mut scr.nodes2);
            pairs.push(PairMoment {
                k: k as u32,
                j: j as u32,
                p: pj,
                omega: m.omega,
                tau_bar: m.tau_int / (4.0 * PI),
            });
        }
        let eps_j = if v2 { sh.eps[j] } else { [0.0; 3] };
        let e_j = norm(&eps_j);
        if v2 && d >= 1e-12 * a {
            // Spec 1.6: pair term = 0 for coincident centres
            make_m_nodes(d, a, dv.beta[j], q, &mut scr.nodes);
            let (e, p0) = pair_e(&scr.nodes, dv.beta[j], &chat, &eps_k, &eps_j, g.n_az);
            sum_b.add(&e);
            sum_b.add_scaled(-p0, &chat); // B = E - P0 chat, same nodes (N1)
        }
        let (sh_c, sh_theta) = if v2 {
            // I.4 shifted disc
            let qe = dot(&chat, &eps_j);
            let w = (qe * qe + 1.0 - e_j * e_j).sqrt();
            let lam = qe + w;
            (
                normalize(&sub(&scale(&chat, lam), &eps_j)),
                (theta / (1.0 - e_j)).min(PI),
            )
        } else {
            (chat, theta)
        };
        occ.push(Occ {
            c,
            chat,
            d,
            a,
            beta: dv.beta[j],
            theta,
            theta_eff: theta.min(1.0),
            eps: eps_j,
            e: e_j,
            sh_c,
            sh_theta,
        });
    }

    // ---- overlap correction: second sum of eq. 4 ----
    let cand_idx = candidates_with(&occ, v2, o.brute_candidates);
    let mut acc = Acc::new(ell_cap);
    let mut leaves: Vec<LeafRecord> = Vec::new();
    let overlap = !cand_idx.is_empty();
    if overlap {
        let cand: Vec<Occ> = cand_idx.iter().map(|&i| occ[i].clone()).collect();
        let cx = Ctx {
            occ: &cand,
            alpha: g.alpha_bg,
            v2,
            eps_k,
            nside0: g.nside0,
            ell_cap,
            f: g.f,
            val: o.validation,
            budget: o.leaf_budget,
            record: o.instrumented,
            k: k as u32,
        };
        let all: Vec<u32> = (0..cand.len() as u32).collect();
        let nb = 12 * g.nside0 * g.nside0; // 768 base cells for nside0=8
        for pix in 0..nb {
            visit(&cx, &mut acc, 0, pix, &all, &all)?;
        }
        leaves = std::mem::take(&mut acc.rec);
    }

    // ---- assembly ----
    let fp = scale(&sum_pair.value(), gamma);
    let fc = scale(&acc.corr.value(), gamma);
    let fdr = if vel {
        f_dr(g.xi, pi_flux, dv.a_eff[k], &eps_k)
    } else {
        [0.0; 3]
    };
    let fab = if v2 {
        let mut t = sum_b;
        t.add(&acc.ab.value());
        scale(&t.value(), gamma)
    } else {
        [0.0; 3]
    };
    let mut tot = Neum3::default();
    tot.add(&fp);
    tot.add(&fc);
    tot.add(&fdr);
    tot.add(&fab);
    let ft = tot.value();
    if !(finite(&fp) && finite(&fc) && finite(&fdr) && finite(&fab) && finite(&ft)) {
        return Err(CoreError::NonFinite { target: k }); // Spec 1.6: abort the step
    }
    Ok(TargetOut {
        fp,
        fc,
        fdr,
        fab,
        ft,
        pairs,
        leaves,
        leaf_count: acc.leaves,
        cap_hits: acc.cap_hits,
        overlap,
        interior,
    })
}
