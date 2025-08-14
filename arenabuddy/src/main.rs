#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "server")]
    {
        arenabuddy::launch_server()?;
    }
    #[cfg(not(feature = "server"))]
    {
        arenabuddy::launch_frontend();
    }
    Ok(())
}
