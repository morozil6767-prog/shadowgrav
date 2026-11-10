//! Separated PDF and Addendum 2 period statuses. Does not change `integrator.rs` or its tolerances.
//! The T/1000 x 5-orbit assertions lock the present split (PDF not met, Addendum 2 met). They are not a
//! physics change and they do not adopt either bound as the only acceptance rule.
use shadowgrav_core::diagnostics::EnergyLabel;
use shadowgrav_core::driver::{orbit_criteria, run_orbit, two_body_orbit, OrbitReport};
use shadowgrav_core::Mode;

fn by_name<'a>(
    rows: &'a [shadowgrav_core::driver::OrbitCriterion],
    name: &str,
) -> &'a shadowgrav_core::driver::OrbitCriterion {
    rows.iter()
        .find(|c| c.name == name)
        .unwrap_or_else(|| panic!("missing {name}"))
}

#[test]
fn criteria_flags_are_independent_on_a_synthetic_report() {
    let mut r = OrbitReport {
        steps: 10,
        omega_expected: 1.0,
        omega_h: 0.01,
        period_expected: 1.0,
        period_measured: 1.0,
        period_rel_err: 1.0e-5,
        max_de: 0.0,
        secular_de: 0.0,
        max_dl: 0.0,
        max_dp: 0.0,
        max_rp_total: 0.0,
        max_rp_shadow: 0.0,
        max_delta_na: 0.0,
        max_fp_iters: 0,
        fp_nonconverged: 0,
        label: EnergyLabel::Conserved,
    };
    // 0.36*(0.01)^2 = 3.6e-5, so 1e-5 fails 5e-6 and passes the addendum bound.
    let rows = orbit_criteria(&r);
    assert!(!by_name(&rows, "PDF 7(a) |T_measured-T|/T <= 5e-6").pass);
    assert!(by_name(&rows, "Addendum2 |T_measured-T|/T <= 0.36*(omega*h)^2").pass);
    r.period_rel_err = 1.0e-6;
    let rows = orbit_criteria(&r);
    assert!(by_name(&rows, "PDF 7(a) |T_measured-T|/T <= 5e-6").pass);
    assert!(by_name(&rows, "Addendum2 |T_measured-T|/T <= 0.36*(omega*h)^2").pass);
    r.period_rel_err = 1.0e-3;
    let rows = orbit_criteria(&r);
    assert!(!by_name(&rows, "PDF 7(a) |T_measured-T|/T <= 5e-6").pass);
    assert!(!by_name(&rows, "Addendum2 |T_measured-T|/T <= 0.36*(omega*h)^2").pass);
}

#[test]
fn existing_t1000_five_orbit_pdf_fails_and_addendum_passes() {
    let r = run_orbit(two_body_orbit(1e-6, 1000, Mode::V0, 1e3).unwrap(), 5).unwrap();
    let rows = orbit_criteria(&r);
    let pdf = by_name(&rows, "PDF 7(a) |T_measured-T|/T <= 5e-6");
    let add = by_name(&rows, "Addendum2 |T_measured-T|/T <= 0.36*(omega*h)^2");
    assert!(r.steps == 5_000, "steps {}", r.steps);
    // h = T/1000 and T = 2π/ω, so ω*h = 2π/1000. This is not h = 1/1000.
    assert!(
        (r.omega_h - 2.0 * std::f64::consts::PI / 1000.0).abs() < 1e-12,
        "omega*h={}",
        r.omega_h
    );
    assert!(
        !pdf.pass,
        "PDF 5e-6 unexpectedly met: period_rel_err={:.6e}",
        r.period_rel_err
    );
    assert!(
        add.pass,
        "Addendum 2 bound missed: period_rel_err={:.6e} bound={:.6e}",
        r.period_rel_err, add.bound
    );
    assert!(
        pdf.pass != add.pass,
        "the two period statuses are not separated"
    );
    assert!(by_name(&rows, "PDF 7(a) max |dE/E| <= (omega*h)^2").pass);
    assert!(by_name(&rows, "PDF 7(a) |dL/L| <= 1e-12").pass);
    assert!(by_name(&rows, "PDF 7(a) R_P <= 1e-12").pass);
    assert!(by_name(&rows, "PDF 7(a) Delta^NA == 0").pass);
}
