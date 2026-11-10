//! NON-CONFORMING "thin-fast" preview path (Spec 4.4): Barnes-Hut monopole tree, uniform-sphere pair field, NO overlap
//! correction. Allowed only for V0, alpha_bg = 0, all tau_c < 1e-2, opening angle <= 0.5. Every use must be labelled
//! non-conforming (Status::thin_fast; the UI badge turns red).
//!
//! Force on k:  F_k = G_eff m^g_k sum_j m^g_j f(c_kj),   G_eff = xi Pi kappa0^2 / (4 pi)  (= G_N when calibrated),
//!   f(c) = c/d^3 (d > a_j, exact for every tau at alpha_bg = 0: Gamma_k P_kj = G_eff m^g_k m^g_j / d^2),
//!   f(c) = c/a_j^3 (d <= a_j, thin uniform-sphere field).
//! Far groups are replaced by their m^g-weighted centroid when cell_size < theta * distance (target can never be inside
//! an accepted cell because theta <= 0.5 < 1/sqrt(3)).
use crate::geom::*;
use crate::types::*;
#[cfg(feature = "parallel")]
use rayon::prelude::*;
use std::f64::consts::PI;

const NONE: u32 = u32::MAX;
const TAU_THIN_MAX: f64 = 1e-2;

struct TNode {
    com: V3,
    m: f64,
    size: f64,
    kids: [u32; 8],
    lo: u32,
    hi: u32,
    leaf: bool,
}

fn octant(p: &V3, c: &V3) -> usize {
    (p[0] > c[0]) as usize | (((p[1] > c[1]) as usize) << 1) | (((p[2] > c[2]) as usize) << 2)
}

struct BuildCell {
    center: V3,
    half: f64,
    depth: u32,
}

fn build(
    nodes: &mut Vec<TNode>,
    order: &mut [u32],
    base: usize,
    pos: &[V3],
    mg: &[f64],
    cell: BuildCell,
) -> u32 {
    let BuildCell {
        center,
        half,
        depth,
    } = cell;
    let (mut m, mut com) = (0.0, [0.0; 3]);
    for &i in order.iter() {
        let i = i as usize;
        m += mg[i];
        for (c, slot) in com.iter_mut().enumerate() {
            *slot += mg[i] * pos[i][c];
        }
    }
    if m > 0.0 {
        for c in com.iter_mut() {
            *c /= m;
        }
    }
    let id = nodes.len() as u32;
    nodes.push(TNode {
        com,
        m,
        size: 2.0 * half,
        kids: [NONE; 8],
        lo: base as u32,
        hi: (base + order.len()) as u32,
        leaf: true,
    });
    if order.len() <= 1 || depth >= 48 {
        return id;
    } // depth cap: coincident points stay a multi-body leaf
    order.sort_unstable_by_key(|&i| octant(&pos[i as usize], &center));
    let mut kids = [NONE; 8];
    let mut start = 0usize;
    for (o, kid) in kids.iter_mut().enumerate() {
        let mut end = start;
        while end < order.len() && octant(&pos[order[end] as usize], &center) == o {
            end += 1;
        }
        if end > start {
            let q = half / 2.0;
            let cc = [
                center[0] + if o & 1 == 1 { q } else { -q },
                center[1] + if o & 2 == 2 { q } else { -q },
                center[2] + if o & 4 == 4 { q } else { -q },
            ];
            *kid = build(
                nodes,
                &mut order[start..end],
                base + start,
                pos,
                mg,
                BuildCell {
                    center: cc,
                    half: q,
                    depth: depth + 1,
                },
            );
        }
        start = end;
    }
    nodes[id as usize].kids = kids;
    nodes[id as usize].leaf = false;
    id
}

#[derive(Copy, Clone)]
struct FieldBodies<'a> {
    pos: &'a [V3],
    a: &'a [f64],
    mg: &'a [f64],
}

fn field(
    nodes: &[TNode],
    order: &[u32],
    bodies: &FieldBodies<'_>,
    k: usize,
    xk: &V3,
    theta: f64,
) -> V3 {
    let FieldBodies { pos, a, mg } = *bodies;
    let mut acc = [0.0; 3];
    let mut stack: Vec<u32> = Vec::with_capacity(96);
    stack.push(0);
    while let Some(id) = stack.pop() {
        let n = &nodes[id as usize];
        if n.leaf {
            for t in n.lo..n.hi {
                let j = order[t as usize] as usize;
                if j == k {
                    continue;
                }
                let c = sub(&pos[j], xk);
                let d = norm(&c);
                if d < 1e-12 * a[j] {
                    continue;
                } // Spec 1.6
                let w = if d > a[j] {
                    mg[j] / (d * d * d)
                } else {
                    mg[j] / (a[j] * a[j] * a[j])
                };
                for q in 0..3 {
                    acc[q] += w * c[q];
                }
            }
        } else {
            let c = sub(&n.com, xk);
            let d = norm(&c);
            if d > 0.0 && n.size < theta * d {
                let w = n.m / (d * d * d);
                for q in 0..3 {
                    acc[q] += w * c[q];
                }
            } else {
                for &kid in &n.kids {
                    if kid != NONE {
                        stack.push(kid);
                    }
                }
            }
        }
    }
    acc
}

pub fn evaluate(
    p: &Particles,
    dv: &Derived,
    g: &Globals,
    o: &Options,
) -> Result<ForceOutput, CoreError> {
    if o.mode != Mode::V0 {
        return Err(CoreError::Gating("thin-fast requires V0"));
    }
    if g.alpha_bg != 0.0 {
        return Err(CoreError::Gating("thin-fast requires alpha_bg = 0"));
    }
    if !(o.thin_theta > 0.0 && o.thin_theta <= 0.5) {
        return Err(CoreError::Config(
            "thin-fast opening angle must be in (0, 0.5]",
        ));
    }
    let n = p.x.len();
    let bodies: Vec<u32> = (0..n)
        .filter(|&i| p.kind[i] == Kind::Body)
        .map(|i| i as u32)
        .collect();
    if bodies
        .iter()
        .any(|&i| crate::mathx::not_lt(dv.tau_c[i as usize], TAU_THIN_MAX))
    {
        return Err(CoreError::Gating("thin-fast requires all tau_c < 1e-2"));
    }
    let geff = g.xi * g.pi_flux() * g.kappa0 * g.kappa0 / (4.0 * PI);
    let mut order = bodies.clone();
    let mut nodes: Vec<TNode> = Vec::new();
    if !order.is_empty() {
        let (mut lo, mut hi) = ([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]);
        for &i in &order {
            for c in 0..3 {
                lo[c] = lo[c].min(p.x[i as usize][c]);
                hi[c] = hi[c].max(p.x[i as usize][c]);
            }
        }
        let center = [
            0.5 * (lo[0] + hi[0]),
            0.5 * (lo[1] + hi[1]),
            0.5 * (lo[2] + hi[2]),
        ];
        let half = (0..3).map(|c| 0.5 * (hi[c] - lo[c])).fold(0.0, f64::max) * 1.000001 + 1e-300;
        build(
            &mut nodes,
            &mut order,
            0,
            &p.x,
            &dv.m_g,
            BuildCell {
                center,
                half,
                depth: 0,
            },
        );
    }
    let bodies_view = FieldBodies {
        pos: &p.x,
        a: &dv.a,
        mg: &dv.m_g,
    };
    let one = |k: usize| -> V3 {
        if nodes.is_empty() {
            return [0.0; 3];
        }
        let f = field(&nodes, &order, &bodies_view, k, &p.x[k], o.thin_theta);
        scale(&f, geff * dv.m_g[k])
    };
    #[cfg(feature = "parallel")]
    let f_tot: Vec<V3> = if o.parallel {
        (0..n).into_par_iter().map(&one).collect()
    } else {
        (0..n).map(one).collect()
    };
    #[cfg(not(feature = "parallel"))]
    let f_tot: Vec<V3> = (0..n).map(one).collect();
    for (k, f) in f_tot.iter().enumerate() {
        if !f.iter().all(|x| x.is_finite()) {
            return Err(CoreError::NonFinite { target: k });
        }
    }
    let status = Status {
        thin_fast: true,
        thin_valid: true,
        v2_valid: true,
        ..Default::default()
    };
    Ok(ForceOutput {
        f_pair: f_tot.clone(),
        f_corr: vec![[0.0; 3]; n],
        f_dr: vec![[0.0; 3]; n],
        f_ab: vec![[0.0; 3]; n],
        f_total: f_tot,
        status,
        sinks: None,
    })
}
