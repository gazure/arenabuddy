use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use anyhow::{Context, Result, anyhow};
use sysinfo::{ProcessesToUpdate, System};
use tracing::{debug, info};

pub const ARENABUDDY_APP_PATH_ENV: &str = "ARENABUDDY_APP_PATH";

#[derive(Debug, Clone)]
pub struct DaemonConfig {
    pub arenabuddy_path: PathBuf,
    pub mtga_process_names: Vec<String>,
    pub arenabuddy_process_names: Vec<String>,
    pub poll_interval: Duration,
    pub launch_cooldown: Duration,
    pub run_once: bool,
    pub dry_run: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessInfo {
    pub name: String,
    pub executable_name: Option<String>,
}

pub fn default_mtga_process_names() -> Vec<String> {
    vec!["MTGA.exe".to_owned(), "MTGA".to_owned()]
}

pub fn default_arenabuddy_process_names() -> Vec<String> {
    vec![
        "arenabuddy".to_owned(),
        "Arenabuddy".to_owned(),
        "arenabuddy.exe".to_owned(),
        "Arenabuddy.exe".to_owned(),
    ]
}

pub fn resolve_arenabuddy_path(configured_path: Option<&Path>) -> Result<PathBuf> {
    if let Some(path) = configured_path {
        return existing_path(path);
    }

    if let Some(path) = std::env::var_os(ARENABUDDY_APP_PATH_ENV) {
        return existing_path(Path::new(&path));
    }

    let current_executable = std::env::current_exe().context("failed to resolve current daemon executable")?;
    let executable_dir = current_executable
        .parent()
        .ok_or_else(|| anyhow!("daemon executable does not have a parent directory"))?;

    sibling_arenabuddy_candidates(executable_dir)
        .into_iter()
        .find(|path| path.exists())
        .ok_or_else(|| {
            anyhow!(
                "could not find ArenaBuddy next to the daemon; pass --arenabuddy <path> or set {ARENABUDDY_APP_PATH_ENV}"
            )
        })
}

pub fn sibling_arenabuddy_candidates(executable_dir: &Path) -> Vec<PathBuf> {
    vec![
        #[cfg(target_os = "macos")]
        executable_dir.join("Arenabuddy.app"),
        executable_dir.join(platform_executable_name("arenabuddy")),
        executable_dir.join(platform_executable_name("Arenabuddy")),
    ]
}

pub fn process_name_matches(actual: &str, expected: &str) -> bool {
    let actual = normalize_process_name(actual);
    let expected = normalize_process_name(expected);
    strip_exe_suffix(&actual) == strip_exe_suffix(&expected)
}

pub fn has_matching_process(processes: &[ProcessInfo], target_names: &[String]) -> bool {
    processes.iter().any(|process| {
        target_names.iter().any(|target| {
            process_name_matches(&process.name, target)
                || process
                    .executable_name
                    .as_deref()
                    .is_some_and(|name| process_name_matches(name, target))
        })
    })
}

pub fn run(config: &DaemonConfig) -> Result<()> {
    validate_config(config)?;

    let keep_running = Arc::new(AtomicBool::new(true));
    let signal_flag = Arc::clone(&keep_running);
    ctrlc::set_handler(move || signal_flag.store(false, Ordering::SeqCst))?;

    let mut system = System::new();
    let mut last_launch = None;

    info!(
        "watching for MTGA processes {:?}; ArenaBuddy process names {:?}",
        config.mtga_process_names, config.arenabuddy_process_names
    );
    info!("will launch ArenaBuddy from {}", config.arenabuddy_path.display());

    while keep_running.load(Ordering::SeqCst) {
        scan_once(config, &mut system, &mut last_launch)?;

        if config.run_once {
            break;
        }

        std::thread::sleep(config.poll_interval);
    }

    Ok(())
}

fn validate_config(config: &DaemonConfig) -> Result<()> {
    if config.poll_interval.is_zero() {
        return Err(anyhow!("poll interval must be greater than zero"));
    }

    if config.mtga_process_names.is_empty() {
        return Err(anyhow!("at least one MTGA process name is required"));
    }

    if config.arenabuddy_process_names.is_empty() {
        return Err(anyhow!("at least one ArenaBuddy process name is required"));
    }

    Ok(())
}

fn scan_once(config: &DaemonConfig, system: &mut System, last_launch: &mut Option<Instant>) -> Result<()> {
    system.refresh_processes(ProcessesToUpdate::All, true);
    let processes = collect_processes(system);
    let mtga_running = has_matching_process(&processes, &config.mtga_process_names);
    let arenabuddy_running = has_matching_process(&processes, &config.arenabuddy_process_names);

    debug!(
        "scan complete: mtga_running={mtga_running}, arenabuddy_running={arenabuddy_running}, process_count={}",
        processes.len()
    );

    if !mtga_running {
        return Ok(());
    }

    if arenabuddy_running {
        debug!("MTGA is running and ArenaBuddy is already running");
        return Ok(());
    }

    if last_launch.is_some_and(|launched_at| launched_at.elapsed() < config.launch_cooldown) {
        debug!("MTGA is running, but ArenaBuddy launch is cooling down");
        return Ok(());
    }

    launch_arenabuddy(config)?;
    *last_launch = Some(Instant::now());
    Ok(())
}

fn collect_processes(system: &System) -> Vec<ProcessInfo> {
    system
        .processes()
        .values()
        .map(|process| ProcessInfo {
            name: process.name().to_string_lossy().into_owned(),
            executable_name: process
                .exe()
                .and_then(Path::file_name)
                .map(|name| name.to_string_lossy().into_owned()),
        })
        .collect()
}

fn launch_arenabuddy(config: &DaemonConfig) -> Result<()> {
    if config.dry_run {
        info!("dry run: would launch {}", config.arenabuddy_path.display());
        return Ok(());
    }

    info!("MTGA is running; launching {}", config.arenabuddy_path.display());
    let mut command = arenabuddy_command(&config.arenabuddy_path);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .with_context(|| format!("failed to launch {}", config.arenabuddy_path.display()))?;

    Ok(())
}

#[cfg(target_os = "macos")]
fn arenabuddy_command(path: &Path) -> Command {
    if is_app_bundle(path) {
        let mut command = Command::new("open");
        command.arg(path);
        return command;
    }

    Command::new(path)
}

#[cfg(not(target_os = "macos"))]
fn arenabuddy_command(path: &Path) -> Command {
    Command::new(path)
}

#[cfg(target_os = "macos")]
fn is_app_bundle(path: &Path) -> bool {
    path.extension()
        .and_then(std::ffi::OsStr::to_str)
        .is_some_and(|extension| extension.eq_ignore_ascii_case("app"))
}

fn existing_path(path: &Path) -> Result<PathBuf> {
    if path.exists() {
        Ok(path.to_path_buf())
    } else {
        Err(anyhow!("ArenaBuddy executable does not exist: {}", path.display()))
    }
}

fn normalize_process_name(name: &str) -> String {
    name.trim().trim_matches('"').to_ascii_lowercase()
}

fn strip_exe_suffix(name: &str) -> &str {
    name.strip_suffix(".exe").unwrap_or(name)
}

fn platform_executable_name(name: &str) -> String {
    if cfg!(target_os = "windows") {
        format!("{name}.exe")
    } else {
        name.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{
        ProcessInfo, has_matching_process, platform_executable_name, process_name_matches,
        sibling_arenabuddy_candidates,
    };

    #[test]
    fn process_matching_ignores_case_and_exe_suffix() {
        assert!(process_name_matches("MTGA.exe", "mtga"));
        assert!(process_name_matches("mtga", "MTGA.EXE"));
        assert!(process_name_matches("Arenabuddy", "arenabuddy.exe"));
        assert!(!process_name_matches("MTGAInstaller.exe", "MTGA.exe"));
    }

    #[test]
    fn process_matching_checks_executable_name() {
        let processes = vec![ProcessInfo {
            name: "wine64-preloader".to_owned(),
            executable_name: Some("MTGA.exe".to_owned()),
        }];
        let targets = vec!["mtga".to_owned()];

        assert!(has_matching_process(&processes, &targets));
    }

    #[test]
    fn sibling_candidates_include_platform_binary_name() {
        let candidates = sibling_arenabuddy_candidates(Path::new("/tmp"));
        let expected = Path::new("/tmp").join(platform_executable_name("arenabuddy"));

        assert!(candidates.contains(&expected));
    }
}
