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

#[derive(Debug, Clone)]
pub struct ServiceInstallContext {
    daemon_binary_path: PathBuf,
    mcp_binary_path: PathBuf,
    config_path: PathBuf,
}

impl ServiceInstallContext {
    pub fn discover(config_source_path: Option<PathBuf>) -> Result<Self, String> {
        let current_executable = std::env::current_exe().map_err(|err| {
            format!("failed to resolve current executable path for service install: {err}")
        })?;
        let daemon_binary_path = resolve_binary_path(&current_executable, "raragd")?;
        let mcp_binary_path = resolve_binary_path(&current_executable, "rarag-mcp")?;
        let config_path = resolve_config_path(config_source_path)?;
        Ok(Self {
            daemon_binary_path,
            mcp_binary_path,
            config_path,
        })
    }
}

pub fn execute(
    command: &ServiceCommand,
    context: &ServiceInstallContext,
) -> Result<ServiceReport, String> {
    run(command, context, false)
}

pub fn plan(
    command: &ServiceCommand,
    context: &ServiceInstallContext,
) -> Result<ServiceReport, String> {
    run(command, context, true)
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

fn run(
    command: &ServiceCommand,
    context: &ServiceInstallContext,
    dry_run: bool,
) -> Result<ServiceReport, String> {
    let operation_name = operation_name(&command.operation).to_string();
    let mut report = ServiceReport {
        operation: operation_name,
        commands: Vec::new(),
        changed_files: Vec::new(),
    };

    match &command.operation {
        ServiceOperation::Install { force } => install(*force, context, dry_run, &mut report)?,
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

fn install(
    force: bool,
    context: &ServiceInstallContext,
    dry_run: bool,
    report: &mut ServiceReport,
) -> Result<(), String> {
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
        &daemon_unit_contents(context),
        force,
        dry_run,
        report,
    )?;
    maybe_write_unit(
        &mcp_path,
        &mcp_unit_contents(context),
        force,
        dry_run,
        report,
    )?;

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

fn daemon_unit_contents(context: &ServiceInstallContext) -> String {
    let daemon_binary = context.daemon_binary_path.to_string_lossy();
    let config_path = context.config_path.to_string_lossy();
    let env_file = env_file_path(&context.config_path);
    let env_file = env_file.to_string_lossy();
    format!(
        "{MANAGED_HEADER}[Unit]\nDescription=rarag daemon\nAfter=network-online.target\nWants=network-online.target\n\n[Service]\nType=simple\nExecStart={daemon_binary} serve --config {config_path}\nRestart=on-failure\nRestartSec=2\nEnvironmentFile=-{env_file}\n\n[Install]\nWantedBy=default.target\n"
    )
}

fn mcp_unit_contents(context: &ServiceInstallContext) -> String {
    let mcp_binary = context.mcp_binary_path.to_string_lossy();
    let config_path = context.config_path.to_string_lossy();
    let env_file = env_file_path(&context.config_path);
    let env_file = env_file.to_string_lossy();
    format!(
        "{MANAGED_HEADER}[Unit]\nDescription=rarag MCP server\nAfter=raragd.service\nRequires=raragd.service\n\n[Service]\nType=simple\nExecStart={mcp_binary} serve --config {config_path}\nRestart=on-failure\nRestartSec=2\nEnvironmentFile=-{env_file}\n\n[Install]\nWantedBy=default.target\n"
    )
}

fn env_file_path(config_path: &Path) -> PathBuf {
    config_path
        .parent()
        .map(|parent| parent.join("daemon.env"))
        .unwrap_or_else(|| PathBuf::from("daemon.env"))
}

fn resolve_binary_path(current_executable: &Path, binary_name: &str) -> Result<PathBuf, String> {
    if let Some(parent) = current_executable.parent() {
        let sibling = parent.join(binary_name);
        if sibling.exists() {
            return Ok(sibling);
        }
    }

    if let Some(path_env) = std::env::var_os("PATH") {
        for path_entry in std::env::split_paths(&path_env) {
            let candidate = path_entry.join(binary_name);
            if candidate.exists() {
                return Ok(candidate);
            }
        }
    }

    Err(format!(
        "unable to resolve {binary_name}; expected sibling of {} or PATH lookup",
        current_executable.display()
    ))
}

fn resolve_config_path(config_source_path: Option<PathBuf>) -> Result<PathBuf, String> {
    if let Some(path) = config_source_path {
        return Ok(path);
    }

    if let Ok(xdg_config_home) = std::env::var("XDG_CONFIG_HOME") {
        return Ok(PathBuf::from(xdg_config_home).join("rarag/rarag.toml"));
    }

    if let Ok(home) = std::env::var("HOME") {
        return Ok(PathBuf::from(home).join(".config/rarag/rarag.toml"));
    }

    Err("unable to resolve config path; set HOME or XDG_CONFIG_HOME or pass --config".to_string())
}

#[cfg(test)]
mod tests {
    use super::{ServiceInstallContext, daemon_unit_contents, resolve_binary_path};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn install_uses_resolved_binary_paths() {
        let context = ServiceInstallContext {
            daemon_binary_path: PathBuf::from("/opt/rarag/bin/raragd"),
            mcp_binary_path: PathBuf::from("/opt/rarag/bin/rarag-mcp"),
            config_path: PathBuf::from("/tmp/configs/rarag.toml"),
        };
        let unit = daemon_unit_contents(&context);
        assert!(
            unit.contains("ExecStart=/opt/rarag/bin/raragd serve --config /tmp/configs/rarag.toml")
        );
    }

    #[test]
    fn install_uses_resolved_config_path() {
        let context = ServiceInstallContext {
            daemon_binary_path: PathBuf::from("/usr/bin/raragd"),
            mcp_binary_path: PathBuf::from("/usr/bin/rarag-mcp"),
            config_path: PathBuf::from("/work/custom/rarag.toml"),
        };
        let unit = daemon_unit_contents(&context);
        assert!(unit.contains("ExecStart=/usr/bin/raragd serve --config /work/custom/rarag.toml"));
        assert!(unit.contains("EnvironmentFile=-/work/custom/daemon.env"));
    }

    #[test]
    fn resolve_binary_path_prefers_sibling() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let bin_dir = std::env::temp_dir()
            .join(format!("rarag-services-test-{unique}"))
            .join("bin");
        fs::create_dir_all(&bin_dir).expect("create bin dir");
        let current_exe = bin_dir.join("rarag");
        fs::write(&current_exe, "").expect("write cli");
        let sibling_daemon = bin_dir.join("raragd");
        fs::write(&sibling_daemon, "").expect("write daemon");

        let resolved = resolve_binary_path(&current_exe, "raragd").expect("resolve binary");
        assert_eq!(resolved, sibling_daemon);
        let _ = fs::remove_dir_all(bin_dir.parent().expect("test root"));
    }
}
