use std::path::PathBuf;

const DEFAULT_GRPC_URL: &str = "https://arenabuddy.grantazure.com";

/// Returns the platform-specific application data directory for `ArenaBuddy`.
pub fn app_data_dir() -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    let dir = match std::env::consts::OS {
        "macos" => home.join("Library/Application Support/com.gazure.dev.arenabuddy.app"),
        "windows" => home.join("AppData/Roaming/com.gazure.dev.arenabuddy.app"),
        "linux" => home.join(".local/share/com.gazure.dev.arenabuddy.app"),
        _ => return None,
    };
    Some(dir)
}

/// Returns the gRPC server URL from `ARENABUDDY_GRPC_URL` env var, or the default.
pub fn grpc_url() -> String {
    std::env::var("ARENABUDDY_GRPC_URL").unwrap_or_else(|_| DEFAULT_GRPC_URL.to_string())
}
