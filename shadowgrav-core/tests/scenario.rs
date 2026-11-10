//! Scenario-file front end: strict parsing, and that the shipped example scenarios reproduce the numbers they claim.
use shadowgrav_core::geom::norm;
use shadowgrav_core::scenario::{parse_json, Json, Scenario};

fn base(extra: &str) -> String {
    format!(
        r#"{{"format":"shadowgrav-scenario/1","globals":{{"tau_c":1}},"bodies":[{{"x":[0,0,0]}},{{"x":[10,0,0]}}]{extra}}}"#
    )
}
fn example(name: &str) -> Scenario {
    let p = format!("{}/scenarios/{name}.json", env!("CARGO_MANIFEST_DIR"));
    Scenario::from_json_text(&std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{p}: {e}")))
        .unwrap()
}

#[test]
fn json_reader_basics_and_errors() {
    assert_eq!(
        parse_json("[1, -2.5e-3, \"a\\n\", true, null]").unwrap(),
        Json::Arr(vec![
            Json::Num(1.0),
            Json::Num(-2.5e-3),
            Json::Str("a\n".into()),
            Json::Bool(true),
            Json::Null
        ])
    );
    for bad in [
        "",
        "{",
        "[1,]",
        "{\"a\":1,}",
        "{\"a\":1 \"b\":2}",
        "[1] x",
        "{\"a\":1,\"a\":2}",
        "1e999",
        "NaN",
        "{1:2}",
    ] {
        assert!(parse_json(bad).is_err(), "should reject {bad:?}");
    }
}
#[test]
fn unknown_and_missing_keys_are_errors_not_defaults() {
    assert!(Scenario::from_json_text(&base("")).is_ok());
    let e = Scenario::from_json_text(&base(r#","quadratue":"certified""#)).unwrap_err(); // typo
    assert!(e.contains("unknown key") && e.contains("quadratue"), "{e}");
    let e = Scenario::from_json_text(&base(r#","mode":"v3""#)).unwrap_err();
    assert!(e.contains("mode"), "{e}");
    assert!(
        Scenario::from_json_text(r#"{"globals":{"tau_c":1},"bodies":[{"x":[0,0,0]}]}"#)
            .unwrap_err()
            .contains("format")
    );
    assert!(Scenario::from_json_text(r#"{"format":"shadowgrav-scenario/1","globals":{"tau_c":1,"kappa0":1e-7},"bodies":[{"x":[0,0,0]}]}"#).is_err());
    assert!(Scenario::from_json_text(
        r#"{"format":"shadowgrav-scenario/1","globals":{"tau_c":1},"bodies":[]}"#
    )
    .is_err());
    assert!(Scenario::from_json_text(
        r#"{"format":"shadowgrav-scenario/1","globals":{"tau_c":1},"bodies":[{"x":[0,0]}]}"#
    )
    .is_err());
    assert!(Scenario::from_json_text(&base(r#","run":{"steps":1.5}"#)).is_err());
}
#[test]
fn engine_validation_reaches_the_user_as_an_error() {
    // coincident-position world is allowed (tested elsewhere); a negative mass is not
    let s = Scenario::from_json_text(r#"{"format":"shadowgrav-scenario/1","globals":{"tau_c":1},"bodies":[{"x":[0,0,0],"m":-1},{"x":[10,0,0]}]}"#).unwrap();
    assert!(s.build().is_err());
}
#[test]
fn thin_two_body_example_is_newtonian_to_the_thin_limit_error() {
    let w = example("two_body_thin").build().unwrap();
    let a0 = (3.0 / (4.0 * std::f64::consts::PI * 2.39e5_f64)).cbrt();
    let newton = 1.0 / (10.0 * a0).powi(2);
    let rel = (norm(&w.force.f_total[0]) - newton).abs() / newton;
    assert!(rel < 5e-5, "relative deviation from G m^2/r^2: {rel:e}");
    assert_eq!(
        w.force.f_total[0][0], -w.force.f_total[1][0],
        "Newton's third law (pair force) must be exact for V0"
    );
}
#[test]
fn collinear_example_gives_the_screening_ratio_of_test_c() {
    let w = example("collinear_screening").build().unwrap();
    let r = norm(&w.force.f_total[0]) / norm(&w.force.f_pair[0]);
    assert!(
        (r - 0.8).abs() < 5e-5,
        "R = {r} (validation tier measured 1.6e-5 from 0.8)"
    );
}
#[test]
fn every_shipped_example_parses_and_builds() {
    let dir = format!("{}/scenarios", env!("CARGO_MANIFEST_DIR"));
    let mut n = 0;
    for e in std::fs::read_dir(&dir).unwrap() {
        let p = e.unwrap().path();
        if p.extension().map(|x| x == "json").unwrap_or(false) {
            let s = Scenario::from_json_text(&std::fs::read_to_string(&p).unwrap())
                .unwrap_or_else(|e| panic!("{p:?}: {e}"));
            s.build().unwrap_or_else(|e| panic!("{p:?}: {e}"));
            n += 1;
        }
    }
    assert!(n >= 3, "expected the shipped examples, found {n}");
}

/// A shipped scenario must RUN, not just parse: drag_decay once aborted at step 1 (h*gamma ~ 5000, unstable).
#[test]
fn every_shipped_scenario_runs_to_completion() {
    use shadowgrav_core::advance;
    let dir = format!("{}/scenarios", env!("CARGO_MANIFEST_DIR"));
    for e in std::fs::read_dir(&dir).unwrap() {
        let p = e.unwrap().path();
        if p.extension().map(|x| x == "json").unwrap_or(false) {
            let s = Scenario::from_json_text(&std::fs::read_to_string(&p).unwrap()).unwrap();
            let mut w = s.build().unwrap();
            for k in 0..s.steps {
                advance(&mut w).unwrap_or_else(|e| panic!("{p:?}: step {}: {e:?}", k + 1));
            }
        }
    }
}
#[test]
fn drag_decay_example_falls_to_one_over_e() {
    use shadowgrav_core::advance;
    let s = example("drag_decay");
    let mut w = s.build().unwrap();
    for _ in 0..s.steps {
        advance(&mut w).unwrap();
    }
    let v = w.p.v[0][0];
    let e1 = (-1.0_f64).exp();
    assert!(
        (v - e1).abs() / e1 < 2e-3,
        "speed {v} vs 1/e = {e1} (h*gamma = 0.005, measured 7.5e-4 off)"
    );
}
