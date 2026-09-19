use std::{fs::File, path::PathBuf};

use arenabuddy_core::cards::CardsDatabase;
use postgresql_embedded::PostgreSQL;
use sqlx::PgPool;
use tracing::info;

use super::MatchDB;
use crate::{Error, Result};

/// Owns database startup, migrations, and shutdown independently of repositories.
///
/// Keep this owner alive while repositories use an embedded database. Dropping
/// it stops the embedded process; dropping a repository does not. External pools
/// remain usable through repository clones until explicitly closed.
#[derive(Debug)]
pub struct Database {
    pool: PgPool,
    embedded: Option<PostgreSQL>,
    // Declared after the process so the lock outlives its drop handler.
    _embedded_lock: Option<File>,
}

impl Database {
    /// Connects to an existing PostgreSQL database at `url` without migrating it.
    ///
    /// # Errors
    /// Returns an error if the connection cannot be established.
    pub async fn connect(url: &str) -> Result<Self> {
        Ok(Self::from_pool(PgPool::connect(url).await?))
    }

    /// Wraps a caller-managed pool without connecting or changing its schema.
    pub fn from_pool(pool: PgPool) -> Self {
        Self {
            pool,
            embedded: None,
            _embedded_lock: None,
        }
    }

    /// Starts persistent embedded PostgreSQL in the existing application directory.
    ///
    /// # Errors
    /// Returns an error if the directory is unavailable or startup fails.
    pub async fn start_embedded() -> Result<Self> {
        Self::start_embedded_at(Self::embedded_data_dir()?).await
    }

    /// Starts persistent PostgreSQL under `directory` without applying migrations.
    ///
    /// The directory contains the installation, cluster, password, and ownership
    /// lock. An existing PID file requires operator inspection; startup never
    /// removes it or stops another instance to recover from a failure.
    ///
    /// # Errors
    /// Returns an error if the directory is in use, a PID file exists, or setup,
    /// startup, database creation, or connection fails.
    pub async fn start_embedded_at(directory: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&directory)?;
        let lock = File::options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(directory.join("arenabuddy.lock"))?;
        lock.try_lock().map_err(std::io::Error::other)?;
        if directory.join("data/postmaster.pid").try_exists()? {
            return Err(std::io::Error::new(std::io::ErrorKind::AlreadyExists,
                "embedded PostgreSQL has a PID file; stop the existing application or inspect the cluster before restarting").into());
        }
        info!("Using embedded PostgreSQL at: {}", directory.display());
        let settings = postgresql_embedded::Settings {
            installation_dir: directory.join("postgres_install"),
            data_dir: directory.join("data"),
            password_file: directory.join("password.txt"),
            temporary: false,
            password: "arenabuddy_local".to_string(),
            ..Default::default()
        };
        let mut embedded = PostgreSQL::new(settings);
        embedded.setup().await?;
        embedded.start().await?;
        if !embedded.database_exists("arenabuddy").await? {
            embedded.create_database("arenabuddy").await?;
        }
        let pool = PgPool::connect(&embedded.settings().url("arenabuddy")).await?;
        Ok(Self {
            pool,
            embedded: Some(embedded),
            _embedded_lock: Some(lock),
        })
    }

    /// Returns the platform-specific directory used by existing desktop installs.
    ///
    /// # Errors
    /// Returns an error if the home directory or operating system is unsupported.
    pub fn embedded_data_dir() -> Result<PathBuf> {
        let home = dirs::home_dir().ok_or_else(|| {
            Error::IoError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Could not determine home directory",
            ))
        })?;
        match std::env::consts::OS {
            "macos" => Ok(home.join("Library/Application Support/com.gazure.dev.arenabuddy.app/postgres")),
            "windows" => Ok(home.join("AppData/Roaming/com.gazure.dev.arenabuddy.app/postgres")),
            "linux" => Ok(home.join(".local/share/com.gazure.dev.arenabuddy.app/postgres")),
            os => Err(std::io::Error::new(std::io::ErrorKind::Unsupported, format!("Unsupported OS: {os}")).into()),
        }
    }

    /// Applies pending application migrations to this pool.
    ///
    /// Call this during startup or deployment, before serving repository traffic.
    /// Repeated calls use `SQLx` migration history and locking.
    ///
    /// # Errors
    /// Returns an error if a migration fails or its recorded checksum differs.
    pub async fn migrate(&self) -> Result<()> {
        sqlx::migrate!("./migrations/postgres").run(&self.pool).await?;
        Ok(())
    }

    /// Returns a repository sharing this pool and using `cards` for card lookups.
    ///
    /// This does not apply migrations or extend an embedded process's lifetime.
    pub fn repository(&self, cards: CardsDatabase) -> MatchDB {
        MatchDB::from_pool(self.pool.clone(), cards)
    }

    /// Closes all clones of this pool, then stops the owned embedded process.
    ///
    /// Stop application work before calling this method. It waits for checked-out
    /// connections to return. It never stops an external PostgreSQL server.
    ///
    /// # Errors
    /// Returns an error if the embedded process cannot be stopped.
    pub async fn close(self) -> Result<()> {
        self.pool.close().await;
        if let Some(embedded) = &self.embedded {
            embedded.stop().await?;
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "lifecycle_tests.rs"]
mod tests;
