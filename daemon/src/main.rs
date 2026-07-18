use std::{path::PathBuf, time::Duration};

use anyhow::Result;
use arenabuddy_daemon::{
    DaemonConfig, default_arenabuddy_process_names, default_mtga_process_names, resolve_arenabuddy_path, run,
};
use clap::Parser;

#[derive(Debug, Parser)]
#[command(
    name = "arenabuddy-daemon",
    about = "Watch for Magic Arena and start ArenaBuddy when it is running"
)]
struct Cli {
    #[arg(
        long,
        value_name = "PATH",
        help = "ArenaBuddy executable or macOS .app bundle path. Defaults to ARENABUDDY_APP_PATH, then a sibling binary"
    )]
    arenabuddy: Option<PathBuf>,

    #[arg(
        long = "mtga-process",
        value_name = "NAME",
        help = "Magic Arena process name to watch for. Can be provided multiple times"
    )]
    mtga_process_names: Vec<String>,

    #[arg(
        long = "arenabuddy-process",
        value_name = "NAME",
        help = "ArenaBuddy process name used to avoid duplicate launches. Can be provided multiple times"
    )]
    arenabuddy_process_names: Vec<String>,

    #[arg(long, default_value_t = 5, help = "Seconds between process scans")]
    poll_interval_seconds: u64,

    #[arg(
        long,
        default_value_t = 30,
        help = "Seconds to wait after a launch attempt before trying again"
    )]
    launch_cooldown_seconds: u64,

    #[arg(long, help = "Run one process scan and exit")]
    once: bool,

    #[arg(long, help = "Log launch decisions without starting ArenaBuddy")]
    dry_run: bool,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();
    let config = DaemonConfig {
        arenabuddy_path: resolve_arenabuddy_path(cli.arenabuddy.as_deref())?,
        mtga_process_names: defaults_if_empty(cli.mtga_process_names, default_mtga_process_names()),
        arenabuddy_process_names: defaults_if_empty(cli.arenabuddy_process_names, default_arenabuddy_process_names()),
        poll_interval: Duration::from_secs(cli.poll_interval_seconds),
        launch_cooldown: Duration::from_secs(cli.launch_cooldown_seconds),
        run_once: cli.once,
        dry_run: cli.dry_run,
    };

    run(&config)
}

fn defaults_if_empty(values: Vec<String>, defaults: Vec<String>) -> Vec<String> {
    if values.is_empty() { defaults } else { values }
}
