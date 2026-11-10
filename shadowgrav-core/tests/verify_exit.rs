//! Spawns the verify binary. Does not change the 5e-6 bound or the T/2000 default.
use std::process::Command;

fn run(force_t1000: bool) -> std::process::Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_verify"));
    if force_t1000 {
        cmd.env("SHADOWGRAV_VERIFY_PDF_STEPS", "1000");
    } else {
        cmd.env_remove("SHADOWGRAV_VERIFY_PDF_STEPS");
    }
    cmd.output().expect("spawn verify")
}

#[test]
fn default_t2000_exits_zero() {
    let out = run(false);
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        out.status.success(),
        "verify status {:?} {text}",
        out.status.code()
    );
    assert!(
        text.contains("h = T/2000"),
        "shipped path did not use T/2000: {text}"
    );
    assert!(
        !text.contains("PDF criterion failed:"),
        "PDF failure on the shipped path: {text}"
    );
}

#[test]
fn t1000_scored_path_exits_nonzero_and_names_the_pdf_period_line() {
    let out = run(true);
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        !out.status.success(),
        "T/1000 scored path should exit non-zero, got {:?} {text}",
        out.status.code()
    );
    assert!(
        text.contains("h = T/1000"),
        "probe did not score T/1000: {text}"
    );
    assert!(
        text.contains("PDF criterion failed: PDF 7(a) |T_measured-T|/T <= 5e-6"),
        "missing failing PDF period line: {text}"
    );
}
