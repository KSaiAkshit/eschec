use std::process::Command;

fn git(args: &[&str]) -> Option<String> {
    Command::new("git")
        .args(args)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
}

fn main() {
    // Rebuild when git state changes
    println!("cargo:rerun-if-changed=./.git/HEAD");
    println!("cargo:rerun-if-changed=./.git/refs");

    let pkg_version = std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.0.0".into());

    let commit = git(&["rev-parse", "--short=12", "HEAD"]).unwrap_or("unknown".into());

    let describe = git(&["describe", "--tags", "--always", "--dirty"]).unwrap_or(commit.clone());

    let full_version = format!("{} ({})", pkg_version, describe,);

    println!("cargo:rustc-env=APP_VERSION={full_version}");
}
