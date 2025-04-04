use tokio::{fs, io};

/// # Errors
///
/// Errors when any fs operations return an error
pub async fn clean() -> io::Result<()> {
    let dir = "scrape_data";
    if let Ok(mut entries) = fs::read_dir(dir).await {
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_file() && path.extension().is_some_and(|ext| ext == "json") {
                fs::remove_file(path).await?;
            }
        }
        Ok(())
    } else {
        Ok(())
    }
}
