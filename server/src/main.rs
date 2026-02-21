#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    arenabuddy_server::run().await
}
