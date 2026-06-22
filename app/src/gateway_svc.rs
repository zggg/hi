use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use hi_core::{
    expand_path, logs_directory, resolve_locale, t, ChannelsConfig, Config, MessageId,
};

const PID_FILE: &str = "run/gateway.pid";

fn current_locale() -> hi_core::Locale {
    Config::load()
        .map(|c| c.resolved_locale())
        .unwrap_or_else(|_| resolve_locale(None))
}

fn hi_home() -> PathBuf {
    expand_path("~/.hi")
}

pub fn logs_dir() -> PathBuf {
    logs_directory()
}

/// Latest rotated log file under `~/.hi/logs/` (prefix `hi.log`).
pub fn log_path() -> PathBuf {
    let dir = logs_directory();
    let _ = fs::create_dir_all(&dir);
    if let Ok(read) = fs::read_dir(&dir) {
        let mut files: Vec<PathBuf> = read
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| {
                p.file_name()
                    .is_some_and(|n| n.to_string_lossy().starts_with("hi.log"))
            })
            .collect();
        files.sort();
        if let Some(path) = files.pop() {
            return path;
        }
    }
    dir.join("hi.log")
}

pub fn pid_path() -> PathBuf {
    hi_home().join(PID_FILE)
}

fn read_pid() -> Option<u32> {
    fs::read_to_string(pid_path())
        .ok()
        .and_then(|s| s.trim().parse().ok())
}

fn write_pid(pid: u32) -> anyhow::Result<()> {
    let path = pid_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, format!("{pid}\n"))?;
    Ok(())
}

pub fn remove_pid_file() {
    let _ = fs::remove_file(pid_path());
}

fn process_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        Command::new("kill")
            .args(["-0", &pid.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}")])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .map(|o| {
                String::from_utf8_lossy(&o.stdout)
                    .lines()
                    .any(|line| line.contains(&pid.to_string()))
            })
            .unwrap_or(false)
    }
}

pub fn clean_stale_pid() {
    if let Some(pid) = read_pid() {
        if !process_alive(pid) {
            remove_pid_file();
        }
    }
}

pub fn running_pid() -> Option<u32> {
    clean_stale_pid();
    read_pid().filter(|&pid| process_alive(pid))
}

pub fn start() -> anyhow::Result<()> {
    let locale = current_locale();
    if let Some(pid) = running_pid() {
        anyhow::bail!(
            "{}",
            t(locale, MessageId::GatewayStatusRunning, &[pid.to_string()])
        );
    }

    let exe = std::env::current_exe()?;
    fs::create_dir_all(logs_dir())?;

    let pid = spawn_detached(&exe)?;
    write_pid(pid)?;

    println!("{}", t(locale, MessageId::GatewayStarted, &[pid.to_string()]));
    println!(
        "{}",
        t(
            locale,
            MessageId::GatewayLogsDir,
            &[logs_dir().display().to_string()],
        )
    );
    println!("{}", t(locale, MessageId::GatewayStopHint, &[]));
    Ok(())
}

#[cfg(unix)]
fn spawn_detached(exe: &Path) -> anyhow::Result<u32> {
    let exe = shell_escape(&exe.display().to_string());
    let script = format!("nohup {exe} gateway run >/dev/null 2>&1 & echo $!");
    let output = Command::new("sh")
        .arg("-c")
        .arg(&script)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        let locale = current_locale();
        anyhow::bail!(
            "{}",
            t(locale, MessageId::GatewayStartFailed, &[err.to_string()])
        );
    }
    let pid_str = String::from_utf8(output.stdout)?.trim().to_string();
    let locale = current_locale();
    pid_str
        .parse()
        .map_err(|_| {
            anyhow::anyhow!(
                "{}",
                t(
                    locale,
                    MessageId::GatewayPidParseFailed,
                    std::slice::from_ref(&pid_str),
                )
            )
        })
}

#[cfg(windows)]
fn spawn_detached(exe: &Path) -> anyhow::Result<u32> {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    const DETACHED_PROCESS: u32 = 0x0000_0008;

    let child = Command::new(exe)
        .args(["gateway", "run"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS)
        .spawn()?;
    Ok(child.id())
}

fn shell_escape(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

pub fn stop() -> anyhow::Result<()> {
    let locale = current_locale();
    let Some(pid) = read_pid() else {
        println!("{}", t(locale, MessageId::GatewayNotRunning, &[]));
        return Ok(());
    };
    if !process_alive(pid) {
        remove_pid_file();
        println!("{}", t(locale, MessageId::GatewayNotRunning, &[]));
        return Ok(());
    }

    signal_stop(pid)?;
    for _ in 0..20 {
        if !process_alive(pid) {
            remove_pid_file();
            println!("{}", t(locale, MessageId::GatewayStopped, &[pid.to_string()]));
            return Ok(());
        }
        thread::sleep(Duration::from_millis(250));
    }

    force_stop(pid)?;
    remove_pid_file();
    println!(
        "{}",
        t(locale, MessageId::GatewayForceStopped, &[pid.to_string()])
    );
    Ok(())
}

fn signal_stop(pid: u32) -> anyhow::Result<()> {
    let locale = current_locale();
    #[cfg(unix)]
    {
        let status = Command::new("kill")
            .arg(pid.to_string())
            .status()?;
        if !status.success() {
            anyhow::bail!(
                "{}",
                t(locale, MessageId::GatewayStopSignalFailed, &[pid.to_string()])
            );
        }
    }
    #[cfg(not(unix))]
    {
        let status = Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T"])
            .status()?;
        if !status.success() {
            anyhow::bail!(
                "{}",
                t(locale, MessageId::GatewayStopFailed, &[pid.to_string()])
            );
        }
    }
    Ok(())
}

fn force_stop(pid: u32) -> anyhow::Result<()> {
    #[cfg(unix)]
    {
        let _ = Command::new("kill")
            .args(["-9", &pid.to_string()])
            .status()?;
    }
    #[cfg(not(unix))]
    {
        let _ = Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .status()?;
    }
    Ok(())
}

pub fn restart() -> anyhow::Result<()> {
    stop()?;
    start()
}

pub fn reload() -> anyhow::Result<()> {
    let locale = current_locale();
    let Some(pid) = running_pid() else {
        anyhow::bail!("{}", t(locale, MessageId::GatewayNotRunning, &[]));
    };
    #[cfg(unix)]
    {
        signal_reload(pid)?;
        println!("{}", t(locale, MessageId::GatewayReloadSent, &[pid.to_string()]));
        Ok(())
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
        anyhow::bail!("{}", t(locale, MessageId::GatewayReloadUnixOnly, &[]));
    }
}

/// TUI 切换模型等场景：gateway 在跑则发 SIGUSR1，未跑则静默跳过。
pub fn notify_reload() {
    #[cfg(unix)]
    {
        if let Some(pid) = running_pid() {
            if let Err(e) = signal_reload(pid) {
                tracing::debug!(error = %e, pid, "gateway reload notify skipped");
            }
        }
    }
}

#[cfg(unix)]
fn signal_reload(pid: u32) -> anyhow::Result<()> {
    let locale = current_locale();
    let status = Command::new("kill")
        .args(["-USR1", &pid.to_string()])
        .status()?;
    if !status.success() {
        anyhow::bail!(
            "{}",
            t(locale, MessageId::GatewayReloadSignalFailed, &[pid.to_string()])
        );
    }
    Ok(())
}

pub fn status() -> anyhow::Result<()> {
    let config = Config::load().map_err(|e| anyhow::anyhow!(e.to_string()))?;
    let locale = config.resolved_locale();
    let channels = ChannelsConfig::load().map_err(|e| anyhow::anyhow!(e.to_string()))?;

    println!(
        "{}",
        t(
            locale,
            MessageId::GatewayPidFile,
            &[pid_path().display().to_string()],
        )
    );
    println!(
        "{}",
        t(
            locale,
            MessageId::GatewayLogsDir,
            &[logs_dir().display().to_string()],
        )
    );
    println!(
        "{}",
        t(
            locale,
            MessageId::GatewayWorkspace,
            std::slice::from_ref(&config.workspace),
        )
    );
    if let Ok(endpoints) = channels.enabled_endpoints() {
        let names: Vec<_> = endpoints.iter().map(|e| e.id.as_str()).collect();
        println!(
            "{}",
            t(locale, MessageId::GatewayChannels, &[names.join(", ")])
        );
    } else {
        println!("{}", t(locale, MessageId::GatewayChannelsNone, &[]));
    }

    if let Some(pid) = running_pid() {
        println!(
            "{}",
            t(locale, MessageId::GatewayStatusRunning, &[pid.to_string()])
        );
    } else {
        println!("{}", t(locale, MessageId::GatewayStatusStopped, &[]));
        return Ok(());
    }

    if log_path().exists() {
        if let Ok(text) = fs::read_to_string(log_path()) {
            if let Some(last) = text.lines().rev().find(|l| !l.trim().is_empty()) {
                println!(
                    "{}",
                    t(
                        locale,
                        MessageId::GatewayRecentLogLine,
                        &[last.to_string()],
                    )
                );
            }
        }
    }
    Ok(())
}

/// Remove pid file when the foreground gateway process exits.
///
/// Author: gz
pub struct PidGuard;

impl PidGuard {
    pub fn new() -> Self {
        Self
    }
}

impl Drop for PidGuard {
    fn drop(&mut self) {
        if read_pid().is_some_and(|pid| pid == std::process::id()) {
            remove_pid_file();
        }
    }
}

#[cfg(test)]
#[path = "../test/unit/gateway_svc.rs"]
mod tests;
