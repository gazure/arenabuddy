use super::*;

fn database(pool: PgPool) -> PostgresMatchDB {
    PostgresMatchDB::from_pool(pool, CardsDatabase::default())
}

async fn user(db: &PostgresMatchDB, name: &str) -> Uuid {
    sqlx::query_scalar("INSERT INTO app_user (discord_id, username) VALUES ($1, $1) RETURNING id")
        .bind(name)
        .fetch_one(&db.pool)
        .await
        .expect("create user")
}

async fn upload(db: &PostgresMatchDB, id: Uuid, owner: Option<Uuid>, marker: i32) -> Result<()> {
    let id = id.to_string();
    let mtga_match = MTGAMatchBuilder::default()
        .id(id.clone())
        .controller_seat_id(1)
        .controller_player_name("controller")
        .opponent_player_name("opponent")
        .created_at(Utc::now())
        .format(Some(format!("format-{marker}")))
        .build()
        .expect("match");
    db.upsert_match_data(
        &mtga_match,
        &[Deck::new("deck".into(), 1, vec![marker], vec![])],
        &[Mulligan::new(&id, 1, 7, marker.to_string(), "Play", "unknown", "keep")],
        &[MatchResult::new(&id, 0, marker, "MatchScope_Match")],
        &[ArenaId::from(marker)],
        &[GameEventLog {
            game_number: marker,
            events: vec![],
        }],
        owner,
    )
    .await
}

async fn snapshot(db: &PostgresMatchDB, id: Uuid) -> Vec<String> {
    let mut rows = vec![
        sqlx::query_scalar("SELECT to_jsonb(m)::text FROM match m WHERE id = $1")
            .bind(id)
            .fetch_one(&db.pool)
            .await
            .expect("match snapshot"),
    ];
    for query in [
        "SELECT COALESCE(jsonb_agg(to_jsonb(t) ORDER BY to_jsonb(t)::text), '[]'::jsonb)::text FROM deck t WHERE match_id = $1",
        "SELECT COALESCE(jsonb_agg(to_jsonb(t) ORDER BY to_jsonb(t)::text), '[]'::jsonb)::text FROM mulligan t WHERE match_id = $1",
        "SELECT COALESCE(jsonb_agg(to_jsonb(t) ORDER BY to_jsonb(t)::text), '[]'::jsonb)::text FROM match_result t WHERE match_id = $1",
        "SELECT COALESCE(jsonb_agg(to_jsonb(t) ORDER BY to_jsonb(t)::text), '[]'::jsonb)::text FROM opponent_deck t WHERE match_id = $1",
        "SELECT COALESCE(jsonb_agg(to_jsonb(t) ORDER BY to_jsonb(t)::text), '[]'::jsonb)::text FROM match_event_log t WHERE match_id = $1",
    ] {
        rows.push(
            sqlx::query_scalar(query)
                .bind(id)
                .fetch_one(&db.pool)
                .await
                .expect("child snapshot"),
        );
    }
    rows
}

#[sqlx::test(migrations = "./migrations/postgres")]
async fn owner_can_retry_and_update(pool: PgPool) {
    let db = database(pool);
    let owner = user(&db, "owner").await;
    let id = Uuid::new_v4();
    upload(&db, id, Some(owner), 1).await.expect("insert");
    let original = snapshot(&db, id).await;
    upload(&db, id, Some(owner), 1).await.expect("retry");
    assert_eq!(snapshot(&db, id).await, original);
    upload(&db, id, Some(owner), 2).await.expect("update");
    assert_ne!(snapshot(&db, id).await, original);
    assert_eq!(
        db.list_decklists(&id.to_string()).await.expect("decks")[0].mainboard(),
        &[2]
    );
}

#[sqlx::test(migrations = "./migrations/postgres")]
async fn other_owner_and_unscoped_writer_cannot_modify_any_records(pool: PgPool) {
    let db = database(pool);
    let owner = user(&db, "owner").await;
    let other = user(&db, "other").await;
    let id = Uuid::new_v4();
    upload(&db, id, Some(owner), 1).await.expect("insert");
    let original = snapshot(&db, id).await;
    for attempted_owner in [Some(other), None] {
        assert!(matches!(
            upload(&db, id, attempted_owner, 2).await,
            Err(Error::MatchOwnershipConflict)
        ));
        assert_eq!(snapshot(&db, id).await, original);
    }
    assert!(db.list_matches(Some(other)).await.expect("list").is_empty());
    assert!(
        db.get_match(&id.to_string(), Some(other))
            .await
            .expect("get")
            .0
            .id()
            .is_empty()
    );
    db.delete_match(&id.to_string(), Some(other)).await.expect("delete");
    assert_eq!(snapshot(&db, id).await, original);
}

#[sqlx::test(migrations = "./migrations/postgres")]
async fn unowned_data_requires_explicit_assignment_and_local_updates_still_work(pool: PgPool) {
    let db = database(pool);
    let owner = user(&db, "owner").await;
    let id = Uuid::new_v4();
    upload(&db, id, None, 1).await.expect("local insert");
    upload(&db, id, None, 2).await.expect("local update");
    let original = snapshot(&db, id).await;
    assert!(matches!(
        upload(&db, id, Some(owner), 3).await,
        Err(Error::MatchOwnershipConflict)
    ));
    assert_eq!(snapshot(&db, id).await, original);

    sqlx::query("UPDATE match SET user_id = $1 WHERE id = $2 AND user_id IS NULL")
        .bind(owner)
        .bind(id)
        .execute(&db.pool)
        .await
        .expect("operator assigns legacy match");
    upload(&db, id, Some(owner), 3)
        .await
        .expect("assigned owner can upload");
}

#[sqlx::test(migrations = "./migrations/postgres")]
async fn concurrent_first_uploads_have_only_one_owner(pool: PgPool) {
    let db = database(pool);
    let first = user(&db, "first").await;
    let second = user(&db, "second").await;
    let id = Uuid::new_v4();
    let (a, b) = tokio::join!(upload(&db, id, Some(first), 1), upload(&db, id, Some(second), 2));
    let (winner, marker) = match (a, b) {
        (Ok(()), Err(Error::MatchOwnershipConflict)) => (first, 1),
        (Err(Error::MatchOwnershipConflict), Ok(())) => (second, 2),
        results => panic!("expected one successful owner, got {results:?}"),
    };
    let owner: Uuid = sqlx::query_scalar("SELECT user_id FROM match WHERE id = $1")
        .bind(id)
        .fetch_one(&db.pool)
        .await
        .expect("owner");
    assert_eq!(owner, winner);
    assert_eq!(
        db.list_decklists(&id.to_string()).await.expect("decks")[0].mainboard(),
        &[marker]
    );
    let event_logs = db.list_event_logs(&id.to_string()).await.expect("events");
    assert_eq!(event_logs.len(), 1);
    assert_eq!(event_logs[0].game_number, marker);
}
