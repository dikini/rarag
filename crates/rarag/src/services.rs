use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::cli::{ServiceCommand, ServiceOperation, ServiceTarget};

const MANAGED_HEADER: &str = "# Managed by rarag service install\n";
const DAEMON_UNIT_NAME: &str = "raragd.service";
const MCP_UNIT_NAME: &str = "rarag-mcp.service";

#[derive(Debug, Clone, Serialize)]
pub struct ServiceReport {
    pub operation: String,
    pub commands: Vec<String>,
    pub changed_files: Vec<String>,
}

pub fn execute(command: &ServiceCommand) -> Result<ServiceReport, String> {
    run(command, false)
}

pub fn plan(command: &ServiceCommand) -> Result<ServiceReport, String> {
    run(command, true)
}

pub fn print_human(report: &ServiceReport) {
    println!("operation={}", report.operation);
    if !report.changed_files.is_empty() {
        println!("changed_files={}", report.changed_files.join(","));
    }
    for cmd in &report.commands {
        println!("{cmd}");
    }
}

fn run(command: &ServiceCommand, dry_run: bool) -> Result<ServiceReport, String> {
    let operation_name = operation_name(&command.operation).to_string();
    let mut report = ServiceReport {
        operation: operation_name,
        commands: Vec::new(),
        changed_files: Vec::new(),
    };

    match &command.operation {
        ServiceOperation::Install { force } => install(*force, dry_run, &mut report)?,
        ServiceOperation::Start { target } => lifecycle("start", *target, dry_run, &mut report)?,
        ServiceOperation::Stop { target } => lifecycle("stop", *target, dry_run, &mut report)?,
        ServiceOperation::Restart { target } => {
            lifecycle("restart", *target, dry_run, &mut report)?
        }
        ServiceOperation::Reload { target } => reload(*target, dry_run, &mut report)?,
    }

    Ok(report)
}

fn operation_name(operation: &ServiceOperation) -> &'static str {
    match operation {
        ServiceOperation::Install { .. } => "install",
        ServiceOperation::Start { .. } => "start",
        ServiceOperation::Stop { .. } => "stop",
        ServiceOperation::Restart { .. } => "restart",
        ServiceOperation::Reload { .. } => "reload",
    }
}

fn install(force: bool, dry_run: bool, report: &mut ServiceReport) -> Result<(), String> {
    let unit_dir = unit_dir()?;
    if dry_run {
        report
            .commands
            .push(format!("mkdir -p {}", unit_dir.to_string_lossy()));
    } else {
        fs::create_dir_all(&unit_dir).map_err(|err| err.to_string())?;
    }

    let daemon_path = unit_dir.join(DAEMON_UNIT_NAME);
    let mcp_path = unit_dir.join(MCP_UNIT_NAME);
    maybe_write_unit(
        &daemon_path,
        &daemon_unit_contents(),
        force,
        dry_run,
        report,
    )?;
    maybe_write_unit(&mcp_path, &mcp_unit_contents(), force, dry_run, report)?;

    systemctl(&["daemon-reload"], dry_run, report)?;
    systemctl(&["enable", "--now", DAEMON_UNIT_NAME], dry_run, report)?;
    systemctl(&["enable", "--now", MCP_UNIT_NAME], dry_run, report)?;
    Ok(())
}

fn lifecycle(
    operation: &str,
    target: ServiceTarget,
    dry_run: bool,
    report: &mut ServiceReport,
) -> Result<(), String> {
    for unit in units_for_target(target) {
        systemctl(&[operation, unit], dry_run, report)?;
    }
    Ok(())
}

fn reload(target: ServiceTarget, dry_run: bool, report: &mut ServiceReport) -> Result<(), String> {
    if target == ServiceTarget::Mcp {
        return Err("reload only supports raragd or all".to_string());
    }
    systemctl(&["kill", "-s", "HUP", DAEMON_UNIT_NAME], dry_run, report)?;
    Ok(())
}

fn maybe_write_unit(
    path: &Path,
    desired: &str,
    force: bool,
    dry_run: bool,
    report: &mut ServiceReport,
) -> Result<(), String> {
    let mut should_write = true;
    if path.exists() {
        let existing = fs::read_to_string(path).map_err(|err| err.to_string())?;
        if existing == desired {
            should_write = false;
        } else if !existing.starts_with(MANAGED_HEADER) {
            return Err(format!(
                "refusing to overwrite unmanaged unit file {}",
                path.display()
            ));
        } else if !force {
            return Err(format!(
                "managed unit file differs at {}; rerun with --force",
                path.display()
            ));
        }
    }

    if should_write {
        if dry_run {
            report
                .commands
                .push(format!("write {}", path.to_string_lossy()));
        } else {
            fs::write(path, desired).map_err(|err| err.to_string())?;
        }
        report.changed_files.push(path.display().to_string());
    }

    Ok(())
}

fn systemctl(args: &[&str], dry_run: bool, report: &mut ServiceReport) -> Result<(), String> {
    let command_preview = format!("systemctl --user {}", args.join(" "));
    report.commands.push(command_preview);
    if dry_run {
        return Ok(());
    }

    let output = Command::new("systemctl")
        .arg("--user")
        .args(args)
        .output()
        .map_err(|err| err.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if stderr.is_empty() {
            Err(format!("systemctl failed: {}", args.join(" ")))
        } else {
            Err(format!("systemctl failed: {stderr}"))
        }
    }
}

fn units_for_target(target: ServiceTarget) -> &'static [&'static str] {
    match target {
        ServiceTarget::All => &[DAEMON_UNIT_NAME, MCP_UNIT_NAME],
        ServiceTarget::Daemon => &[DAEMON_UNIT_NAME],
        ServiceTarget::Mcp => &[MCP_UNIT_NAME],
    }
}

fn unit_dir() -> Result<PathBuf, String> {
    let home = std::env::var("HOME").map_err(|_| "HOME is not set".to_string())?;
    Ok(PathBuf::from(home).join(".config/systemd/user"))
}

fn daemon_unit_contents() -> String {
    format!(
        "{MANAGED_HEADER}[Unit]\nDescription=rarag daemon\nAfter=network-online.target\nWants=network-online.target\n\n[Service]\nType=simple\nExecStart=%h/.cargo/bin/raragd serve --config %h/.config/rarag/rarag.toml\nRestart=on-failure\nRestartSec=2\nEnvironmentFile=-%h/.config/rarag/daemon.env\n\n[Install]\nWantedBy=default.target\n"
    )
}

fn mcp_unit_contents() -> String {
    format!(
        "{MANAGED_HEADER}[Unit]\nDescription=rarag MCP server\nAfter=raragd.service\nRequires=raragd.service\n\n[Service]\nType=simple\nExecStart=%h/.cargo/bin/rarag-mcp serve --config %h/.config/rarag/rarag.toml\nRestart=on-failure\nRestartSec=2\nEnvironmentFile=-%h/.config/rarag/daemon.env\n\n[Install]\nWantedBy=default.target\n"
    )
}
