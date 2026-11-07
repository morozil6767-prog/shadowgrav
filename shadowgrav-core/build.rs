// Records provenance (git commit, rustc version) at build time. No dependencies; every failure degrades to "unknown".
use std::process::Command;
fn run(cmd: &str, args: &[&str]) -> String {
    Command::new(cmd).args(args).output().ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".into())
}
fn main() {
    let commit = run("git", &["rev-parse", "--short=12", "HEAD"]);
    let dirty = if run("git", &["status", "--porcelain"]) == "unknown" { "clean-or-unknown" } else { "dirty" };
    let rustc = run(&std::env::var("RUSTC").unwrap_or_else(|_| "rustc".into()), &["-V"]);
    println!("cargo:rustc-env=SG_GIT_COMMIT={commit}");
    println!("cargo:rustc-env=SG_GIT_DIRTY={dirty}");
    println!("cargo:rustc-env=SG_RUSTC={rustc}");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../.git/HEAD");
}
