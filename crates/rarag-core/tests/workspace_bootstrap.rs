use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("workspace root")
        .to_path_buf()
}

fn run_cargo(args: &[&str]) -> std::process::Output {
    Command::new("cargo")
        .args(args)
        .current_dir(repo_root())
        .output()
        .expect("cargo command to run")
}

#[test]
fn all_workspace_members_build() {
    let output = run_cargo(&["check", "-p", "rarag", "-p", "raragd", "-p", "rarag-mcp", "--quiet"]);
    assert!(
        output.status.success(),
        "cargo check failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn binaries_start_with_help() {
    for package in ["rarag", "raragd", "rarag-mcp"] {
        let output = run_cargo(&["run", "--quiet", "-p", package, "--", "--help"]);
        assert!(
            output.status.success(),
            "cargo run failed for {package}\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains(package), "expected help output to mention {package}, got: {stdout}");
    }
}
