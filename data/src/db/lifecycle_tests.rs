use super::*;
use crate::ArenabuddyRepository;

#[sqlx::test(migrations = false)]
async fn constructing_and_using_a_repository_does_not_initialize_schema(pool: PgPool) {
    let database = Database::from_pool(pool.clone());
    let repo = database.repository(CardsDatabase::default());
    let repository: &dyn ArenabuddyRepository = &repo;
    assert!(repository.list_drafts().await.is_err());
    let history: Option<String> = sqlx::query_scalar("SELECT to_regclass('_sqlx_migrations')::text")
        .fetch_one(&pool)
        .await
        .expect("schema lookup");
    assert_eq!(history, None);
    database.migrate().await.expect("explicit migration");
    assert!(
        repository
            .list_drafts()
            .await
            .expect("query after migration")
            .is_empty()
    );
}

#[sqlx::test(migrations = "./migrations/postgres")]
async fn migrations_preserve_existing_data_and_repository_clones_share_the_pool(pool: PgPool) {
    let owner: sqlx::types::Uuid = sqlx::query_scalar(
        "INSERT INTO app_user (discord_id, username) VALUES ('existing', 'existing user') RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .expect("existing data");
    let database = Database::from_pool(pool.clone());
    database.migrate().await.expect("first restart");
    database.migrate().await.expect("second restart");
    let username: String = sqlx::query_scalar("SELECT username FROM app_user WHERE id = $1")
        .bind(owner)
        .fetch_one(&pool)
        .await
        .expect("preserved owner");
    assert_eq!(username, "existing user");
    let repo = database.repository(CardsDatabase::default());
    let clone = repo.clone();
    drop(repo);
    drop(database);
    assert!(
        clone
            .list_drafts()
            .await
            .expect("external pool survives owner drop")
            .is_empty()
    );
    assert!(!pool.is_closed());
}

#[sqlx::test(migrations = false)]
async fn migration_errors_are_returned_to_startup(pool: PgPool) {
    sqlx::query("CREATE TABLE match (incompatible_column INTEGER)")
        .execute(&pool)
        .await
        .expect("incompatible schema");
    let database = Database::from_pool(pool);
    assert!(matches!(database.migrate().await, Err(Error::MigrationError(_))));
}

#[sqlx::test(migrations = "./migrations/postgres")]
async fn explicit_close_closes_repository_pools_without_stopping_external_postgres(pool: PgPool) {
    let options = (*pool.connect_options()).clone();
    let database = Database::from_pool(pool.clone());
    let repo = database.repository(CardsDatabase::default());
    database.close().await.expect("close");
    assert!(pool.is_closed());
    assert!(repo.list_drafts().await.is_err());
    let independent = PgPool::connect_with(options).await.expect("server still running");
    assert_eq!(
        sqlx::query_scalar::<_, i32>("SELECT 1")
            .fetch_one(&independent)
            .await
            .expect("query"),
        1
    );
    independent.close().await;
}

#[tokio::test]
async fn embedded_startup_preserves_an_existing_pid_file() {
    let dir = tempfile::tempdir().expect("directory");
    std::fs::create_dir(dir.path().join("data")).expect("data directory");
    let pid = dir.path().join("data/postmaster.pid");
    std::fs::write(&pid, "existing process marker").expect("PID file");
    assert!(matches!(Database::start_embedded_at(dir.path().into()).await,
        Err(Error::IoError(error)) if error.kind() == std::io::ErrorKind::AlreadyExists));
    assert_eq!(
        std::fs::read_to_string(&pid).expect("PID retained"),
        "existing process marker"
    );
    assert!(!dir.path().join("postgres_install").exists());
}

#[tokio::test]
async fn embedded_startup_refuses_an_owned_directory_before_setup() {
    let dir = tempfile::tempdir().expect("directory");
    let lock = File::create(dir.path().join("arenabuddy.lock")).expect("lock file");
    lock.try_lock().expect("lock");
    assert!(Database::start_embedded_at(dir.path().into()).await.is_err());
    assert!(!dir.path().join("postgres_install").exists());
}

#[tokio::test]
#[ignore = "starts bundled PostgreSQL; run explicitly for lifecycle changes"]
async fn embedded_restart_preserves_data_and_repository_drops_do_not_stop_postgres() {
    let dir = tempfile::tempdir().expect("directory");
    let database = Database::start_embedded_at(dir.path().into())
        .await
        .expect("embedded startup");
    database.migrate().await.expect("migrate");
    let owner: sqlx::types::Uuid =
        sqlx::query_scalar("INSERT INTO app_user (discord_id, username) VALUES ('saved', 'saved user') RETURNING id")
            .fetch_one(&database.pool)
            .await
            .expect("save user");
    let repo = database.repository(CardsDatabase::default());
    let clone = repo.clone();
    drop(repo);
    assert!(clone.list_drafts().await.expect("repository clone").is_empty());
    drop(clone);
    assert!(Database::start_embedded_at(dir.path().into()).await.is_err());
    assert_eq!(
        sqlx::query_scalar::<_, i32>("SELECT 1")
            .fetch_one(&database.pool)
            .await
            .expect("owned server still running"),
        1
    );
    database.close().await.expect("stop");
    assert!(!dir.path().join("data/postmaster.pid").exists());
    let reopened = Database::start_embedded_at(dir.path().into()).await.expect("restart");
    reopened.migrate().await.expect("migrate again");
    let username: String = sqlx::query_scalar("SELECT username FROM app_user WHERE id = $1")
        .bind(owner)
        .fetch_one(&reopened.pool)
        .await
        .expect("saved user");
    assert_eq!(username, "saved user");
    reopened.close().await.expect("stop restarted instance");
    assert!(dir.path().join("data/PG_VERSION").exists());
}
