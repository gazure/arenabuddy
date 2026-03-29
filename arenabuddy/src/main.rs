#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app_name = std::env::var("APP_NAME").unwrap_or_else(|_| "arenabuddy".to_string());
    arenabuddy::launch(app_name)?;
    Ok(())
}
