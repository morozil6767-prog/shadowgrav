//! Reproducible snapshot export: strict rectangular CSV data plus a JSON metadata sidecar.
use crate::protocol::{Badge, FrameSnapshot};
use std::fmt::Write as FmtWrite;
use std::io::Write;
use std::path::{Path, PathBuf};

const CSV_HEADER: &str = concat!(
    "index,is_probe,mass,radius,tau_c,S,",
    "x0,x1,x2,v0,v1,v2,",
    "ft0,ft1,ft2,fp0,fp1,fp2,fc0,fc1,fc2,fd0,fd1,fd2,fa0,fa1,fa2"
);

/// Write a plain RFC-4180-style numeric CSV and a same-stem `.meta.json` sidecar.
/// The CSV is rectangular and directly loadable by pandas, NumPy, R, and spreadsheet tools;
/// machine-readable run metadata stays out of the data table.
pub fn export_snapshot(path: &Path, snap: &FrameSnapshot) -> std::io::Result<()> {
    let mut csv = std::fs::File::create(path)?;
    writeln!(csv, "{CSV_HEADER}")?;
    for (i, b) in snap.bodies.iter().enumerate() {
        write!(csv, "{i},{}", u8::from(b.is_probe))?;
        for value in [
            b.mass, b.radius, b.tau_c, b.s,
            b.x[0], b.x[1], b.x[2], b.v[0], b.v[1], b.v[2],
            b.f_total[0], b.f_total[1], b.f_total[2],
            b.f_pair[0], b.f_pair[1], b.f_pair[2],
            b.f_corr[0], b.f_corr[1], b.f_corr[2],
            b.f_dr[0], b.f_dr[1], b.f_dr[2],
            b.f_ab[0], b.f_ab[1], b.f_ab[2],
        ] {
            write!(csv, ",{value:.16e}")?;
        }
        writeln!(csv)?;
    }
    csv.flush()?;

    let meta_path = metadata_path(path);
    let mut meta = std::fs::File::create(meta_path)?;
    meta.write_all(metadata_json(path, snap).as_bytes())?;
    meta.flush()?;
    Ok(())
}

pub fn metadata_path(csv_path: &Path) -> PathBuf {
    csv_path.with_extension("meta.json")
}

fn json_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c <= '\u{1f}' => { let _ = write!(out, "\\u{:04x}", c as u32); }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn json_num(value: f64) -> String {
    if value.is_finite() { format!("{value:.17e}") } else { "null".to_string() }
}

fn json_opt_num(value: Option<f64>) -> String {
    value.map(json_num).unwrap_or_else(|| "null".to_string())
}

fn json_strings(values: &[String]) -> String {
    let values = values.iter().map(|v| json_string(v)).collect::<Vec<_>>().join(", ");
    format!("[{values}]")
}

fn metadata_json(csv_path: &Path, snap: &FrameSnapshot) -> String {
    let mut out = String::new();
    let file_name = csv_path.file_name().and_then(|s| s.to_str()).unwrap_or("snapshot.csv");
    // `badge_n_red` counts reason entries, not bytes or characters in their text.
    let split = snap.badge_n_red.min(snap.badge_reasons.len());
    let (red, caveats) = snap.badge_reasons.split_at(split);
    let _ = writeln!(out, "{{");
    let _ = writeln!(out, "  \"schema\": \"shadowgrav.snapshot.v2\",");
    let _ = writeln!(out, "  \"csv_file\": {},", json_string(file_name));
    let columns = CSV_HEADER.split(',').map(str::to_string).collect::<Vec<_>>();
    let _ = writeln!(out, "  \"csv_columns\": {},", json_strings(&columns));
    let _ = writeln!(out, "  \"units\": {{\"mass\": \"M\", \"radius\": \"L\", \"tau_c\": \"1\", \"S\": \"1\", \"position\": \"L\", \"velocity\": \"L/T\", \"force\": \"M*L/T^2\"}},");
    let _ = writeln!(out, "  \"sequence\": {},", snap.seq);
    let _ = writeln!(out, "  \"load_generation\": {},", snap.load_gen);
    let _ = writeln!(out, "  \"time\": {},", json_num(snap.time));
    let _ = writeln!(out, "  \"step\": {},", snap.step);
    let _ = writeln!(out, "  \"running\": {},", snap.running);
    let _ = writeln!(out, "  \"rate_steps_per_second\": {},", json_num(snap.rate));
    let _ = writeln!(out, "  \"measured_steps_per_second\": {},", json_num(snap.steps_per_sec));
    let _ = writeln!(out, "  \"particle_count\": {},", snap.bodies.len());
    if let Some(p) = snap.preset {
        let _ = writeln!(out, "  \"preset\": {{\"kind\": {}, \"tau_c\": {}, \"steps_per_orbit\": {}, \"n\": {}}},",
            json_string(&format!("{:?}", p.kind)), json_num(p.tau_c), p.steps_per_orbit, p.n);
    } else {
        let _ = writeln!(out, "  \"preset\": null,");
    }
    let p = &snap.params;
    let _ = writeln!(out, "  \"parameters\": {{");
    let _ = writeln!(out, "    \"mode\": {}, \"quadrature_tier\": {},", json_string(&format!("{:?}", p.mode)), json_string(p.quad_tier.label()));
    let _ = writeln!(out, "    \"G_N\": {}, \"kappa0\": {}, \"xi\": {}, \"v_g\": {}, \"Pi\": {},",
        json_num(p.g_n), json_num(p.kappa0), json_num(p.xi), json_num(p.v_g), json_num(p.pi_flux));
    let _ = writeln!(out, "    \"calibrated\": {}, \"alpha_bg\": {}, \"h\": {}, \"adaptive_step\": {},",
        p.calibrated, json_num(p.alpha_bg), json_num(p.h), p.adaptive);
    let _ = writeln!(out, "    \"f\": {}, \"ell_cap\": {}, \"instrumented\": {}, \"leaf_budget\": {},",
        json_num(p.f), p.ell_cap, p.instrumented, p.leaf_budget);
    let _ = writeln!(out, "    \"thin_fast_nonconforming\": {}, \"parallel\": {}", p.thin_fast, p.parallel);
    let _ = writeln!(out, "  }},");

    let d = &snap.diag;
    let _ = writeln!(out, "  \"diagnostics\": {{");
    let _ = writeln!(out, "    \"energy\": {}, \"kinetic_energy\": {}, \"potential_energy\": {}, \"energy_nonconservative\": {},",
        json_num(d.e), json_num(d.ke), json_num(d.pe), d.energy_nonconservative);
    let _ = writeln!(out, "    \"dE\": {}, \"max_dE\": {}, \"dP\": {}, \"max_dP\": {}, \"dL\": {}, \"max_dL\": {},",
        json_num(d.de), json_num(d.max_de), json_num(d.dp), json_num(d.max_dp), json_num(d.dl), json_num(d.max_dl));
    let _ = writeln!(out, "    \"R_P_shadow\": {}, \"R_P_total\": {}, \"max_R_P_total\": {},",
        json_num(d.rp_shadow), json_num(d.rp_total), json_num(d.max_rp_total));
    let _ = writeln!(out, "    \"leaf_count\": {}, \"cap_hits\": {}, \"max_correction_over_force\": {},",
        d.leaf_count, d.cap_hits, json_num(d.max_corr_over_f));
    let _ = writeln!(out, "    \"fixed_point_iterations\": {}, \"fixed_point_residual\": {}, \"fixed_point_converged\": {}, \"force_evaluations\": {},",
        d.fp_iters, json_num(d.fp_residual), d.fp_converged, d.force_evals);
    let _ = writeln!(out, "    \"tau_c_min\": {}, \"tau_c_mean\": {}, \"tau_c_max\": {}, \"S_min\": {}, \"S_mean\": {},",
        json_num(d.tau_min), json_num(d.tau_mean), json_num(d.tau_max), json_num(d.s_min), json_num(d.s_mean));
    let _ = writeln!(out, "    \"thin_valid\": {}, \"V2_valid\": {}, \"overlap_present\": {}, \"interior_present\": {},",
        d.thin_valid, d.v2_valid, d.overlap_present, d.interior_present);
    let _ = writeln!(out, "    \"extras\": [");
    for (i, (key, value)) in d.extras.iter().enumerate() {
        let comma = if i + 1 == d.extras.len() { "" } else { "," };
        let _ = writeln!(out, "      {{\"key\": {}, \"value\": {}}}{}", json_string(key), json_string(value), comma);
    }
    let _ = writeln!(out, "    ],");
    let _ = writeln!(out, "    \"target_metrics\": [");
    for (i, t) in d.targets.iter().enumerate() {
        let comma = if i + 1 == d.targets.len() { "" } else { "," };
        let _ = writeln!(out, "      {{\"k\": {}, \"delta_NA\": {}, \"delta_sat\": {}, \"R_newton\": {}, \"R_thin\": {}, \"gamma\": {}}}{}",
            t.k, json_num(t.delta_na), json_num(t.delta_sat), json_num(t.r_newton), json_num(t.r_thin), json_opt_num(t.gamma), comma);
    }
    let _ = writeln!(out, "    ],");
    let _ = writeln!(out, "    \"pair_metrics\": [");
    for (i, pair) in d.pairs.iter().enumerate() {
        let comma = if i + 1 == d.pairs.len() { "" } else { "," };
        let _ = writeln!(out, "      {{\"k\": {}, \"j\": {}, \"G_eff\": {}, \"expected\": {}}}{}",
            pair.k, pair.j, json_num(pair.g_eff), json_opt_num(pair.expected), comma);
    }
    let _ = writeln!(out, "    ],");
    if let Some(r) = &d.ref_check {
        let _ = writeln!(out, "    \"reference_check\": {{\"max_relative_difference\": {}, \"estimated_error\": {}, \"exceeds_estimate\": {}, \"text\": {}}},",
            json_opt_num(r.max_rel_diff), json_opt_num(r.est_err), r.exceeds_est.map(|v| v.to_string()).unwrap_or_else(|| "null".to_string()), json_string(&r.text));
    } else {
        let _ = writeln!(out, "    \"reference_check\": null,");
    }
    let _ = writeln!(out, "    \"tau_additive_max_lower_bound\": {}", json_opt_num(d.tau_add_max));
    let _ = writeln!(out, "  }},");

    let _ = writeln!(out, "  \"conformance\": {{");
    let _ = writeln!(out, "    \"badge\": {}, \"red_reasons\": {}, \"caveats\": {}",
        json_string(&format!("{:?}", snap.badge)), json_strings(red), json_strings(caveats));
    let _ = writeln!(out, "  }},");
    let _ = writeln!(out, "  \"error\": {}", snap.error.as_ref().map(|e| json_string(e)).unwrap_or_else(|| "null".to_string()));
    let _ = writeln!(out, "}}");
    out
}

pub fn default_export_name(snap: &FrameSnapshot) -> String {
    let badge = match snap.badge {
        Badge::Green => "green",
        Badge::Amber => "amber",
        Badge::Red => "red",
    };
    format!("shadowgrav_snap_step{}_t{:.6}_{}_seq{}.csv", snap.step, snap.time, badge, snap.seq)
}

pub fn conformance_summary(snap: &FrameSnapshot) -> String {
    let mut s = String::new();
    s.push_str(&format!("badge={:?}\n", snap.badge));
    s.push_str(&format!("mode={:?}\n", snap.params.mode));
    s.push_str(&format!("quad_tier={}\n", snap.params.quad_tier.label()));
    s.push_str(&format!("thin_fast={}\n", snap.params.thin_fast));
    if snap.params.thin_fast {
        s.push_str("WARNING: thin-fast is NON-CONFORMING (Barnes-Hut preview; no overlap correction).\n");
    }
    for r in &snap.badge_reasons { s.push_str(r); s.push('\n'); }
    for (k, v) in &snap.diag.extras { s.push_str(&format!("{k} = {v}\n")); }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{BodyView, DiagView, ParamView, QuadTier};
    use shadowgrav_core::Mode;
    use std::sync::Arc;

    fn fixture() -> FrameSnapshot {
        FrameSnapshot {
            seq: 4, load_gen: 2, preset: None, running: false, rate: 1.0, steps_per_sec: 2.0,
            time: 0.25, step: 10,
            bodies: vec![BodyView {
                x: [1.0, 2.0, 3.0], v: [0.1, 0.2, 0.3], mass: 2.0, radius: 0.01,
                tau_c: 0.5, s: 0.9, is_probe: false,
                f_total: [1.0, 0.0, 0.0], f_pair: [0.9, 0.0, 0.0], f_corr: [0.1, 0.0, 0.0],
                f_dr: [0.0; 3], f_ab: [0.0; 3],
            }],
            params: ParamView {
                mode: Mode::V0, kappa0: 2e-7, xi: 1.0, v_g: 1e3, alpha_bg: 0.0,
                g_n: 1.0, pi_flux: 3e14, calibrated: true, h: 1e-3, adaptive: false,
                f: 0.15, quad_tier: QuadTier::Standard, instrumented: true,
                leaf_budget: 0, ell_cap: 12, thin_fast: false, parallel: false,
            },
            diag: Arc::new(DiagView::default()), badge: Badge::Green,
            badge_reasons: vec![
                "red \"reason\"\nline".to_string(),
                "caveat λ — unicode-safe split".to_string(),
            ], badge_n_red: 1, error: None,
        }
    }

    #[test]
    fn export_writes_rectangular_csv_and_json_metadata_sidecar() {
        let root = std::env::temp_dir().join(format!("shadowgrav-export-test-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&root);
        let csv_path = root.join("snapshot.csv");
        let json_path = metadata_path(&csv_path);
        export_snapshot(&csv_path, &fixture()).expect("export");
        let csv = std::fs::read_to_string(&csv_path).expect("csv contents");
        let mut lines = csv.lines();
        let header = lines.next().expect("header");
        let row = lines.next().expect("body row");
        assert_eq!(header.split(',').count(), 27);
        assert_eq!(row.split(',').count(), 27);
        assert!(header.starts_with("index,is_probe,mass,radius,tau_c,S,"));
        let json = std::fs::read_to_string(&json_path).expect("metadata contents");
        assert!(json.contains("shadowgrav.snapshot.v2"));
        assert!(json.contains("\"csv_columns\""));
        assert!(json.contains("M*L/T^2"));
        assert!(json.contains("\\\"reason\\\"\\nline"));
        assert!(json.contains("caveat λ — unicode-safe split"));
        assert!(json.contains("\"red_reasons\": [\"red "));
        assert!(json.contains("\"caveats\": [\"caveat λ"));
        assert!(json.contains("\"kappa0\""));
        let _ = std::fs::remove_file(csv_path);
        let _ = std::fs::remove_file(json_path);
        let _ = std::fs::remove_dir(root);
    }
}
