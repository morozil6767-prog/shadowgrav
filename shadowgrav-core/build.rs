// Records provenance (git commit, rustc version) at build time. No dependencies; every failure degrades to "unknown".
use std::process::Command;
fn run(cmd: &str, args: &[&str]) -> String {
    Command::new(cmd)
        .args(args)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".into())
}
fn main() {
    let commit = run("git", &["rev-parse", "--short=12", "HEAD"]);
    // Three distinct states. `run` folds "git failed" and "empty output" into the same "unknown", so ask git directly here:
    // a successful empty `status --porcelain` is a clean tree; non-empty output is dirty; no git / not a repo is unknown.
    let dirty = match Command::new("git")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .args(["status", "--porcelain"])
        .output()
    {
        Ok(o) if o.status.success() => {
            if o.stdout.iter().all(|b| b.is_ascii_whitespace()) {
                "clean"
            } else {
                "dirty"
            }
        }
        _ => "unknown",
    };
    let rustc = run(
        &std::env::var("RUSTC").unwrap_or_else(|_| "rustc".into()),
        &["-V"],
    );
    println!("cargo:rustc-env=SG_GIT_COMMIT={commit}");
    println!("cargo:rustc-env=SG_GIT_DIRTY={dirty}");
    println!("cargo:rustc-env=SG_RUSTC={rustc}");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../.git/HEAD");
    // The stamp is a build-time snapshot. Cargo reruns this script when HEAD or the index changes (commit, add, reset). Editing a tracked
    // file without staging it, or adding an untracked one, does not re-stamp. GIT_OPTIONAL_LOCKS=0 (above) stops `git status` from
    // rewriting the very index file watched here, which would otherwise force a rerun on the next build.
    println!("cargo:rerun-if-changed=../.git/index");
}
