use std::process;

#[tokio::main]
async fn main() {
    if let Err(e) = arenabuddy_cli::run().await {
        eprintln!("Error: {e}");
        process::exit(1);
    }
}
