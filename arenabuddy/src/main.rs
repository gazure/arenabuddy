#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "server")]
    {
        arenabuddy::server_start()?;
    }
    #[cfg(not(feature = "server"))]
    {
        arenabuddy::launch_frontend()?;
    }
    Ok(())
}
