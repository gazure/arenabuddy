use arenabuddy_core::{
    cards::CardsDatabase,
    models::{ArenaId, MTGAMatch, MatchResult, Mulligan},
};
use sqlx::PgPool;

use super::*;
use crate::ArenabuddyRepository;

fn database(pool: PgPool) -> PostgresMatchDB {
    PostgresMatchDB {
        pool,
        _db: None,
        cards: CardsDatabase::default(),
    }
}

fn data() -> MatchData {
    let id = Uuid::new_v4().to_string();
    MatchData {
        mtga_match: MTGAMatch::new(&id, 1, "player", "opponent"),
        decks: vec![Deck::new("Found Deck".into(), 1, vec![123], vec![456])],
        mulligans: vec![Mulligan::new(&id, 1, 7, "[123]", "Play", "unknown", "keep")],
        results: vec![MatchResult::new(&id, 0, 1, "MatchScope_Match")],
        opponent_deck: OpponentDeck::new(vec![ArenaId::new(456)]),
        event_logs: vec![GameEventLog {
            game_number: 1,
            events: vec![],
        }],
    }
}

async fn job(db: &PostgresMatchDB, owner: &str) -> MatchUploadJob {
    db.claim_match_upload(owner).await.expect("claim").expect("job")
}

#[sqlx::test(migrations = "./migrations/postgres")]
async fn queue_failure_rolls_back_the_entire_match(pool: PgPool) {
    let db = database(pool);
    sqlx::raw_sql(
        "CREATE FUNCTION reject_upload() RETURNS trigger LANGUAGE plpgsql AS $$
         BEGIN RAISE EXCEPTION 'simulated queue failure'; END $$;
         CREATE TRIGGER reject_upload BEFORE INSERT ON match_upload_outbox
         FOR EACH ROW EXECUTE FUNCTION reject_upload();",
    )
    .execute(&db.pool)
    .await
    .expect("failure trigger");
    let data = data();
    assert!(
        db.save_match_for_upload(&data, Some(&Uuid::new_v4().to_string()))
            .await
            .is_err()
    );
    assert!(db.list_matches(None).await.expect("matches").is_empty());
    assert!(db.list_decklists(data.mtga_match.id()).await.expect("decks").is_empty());
    assert!(
        db.match_upload_status(data.mtga_match.id())
            .await
            .expect("status")
            .is_none()
    );
}

#[sqlx::test(migrations = "./migrations/postgres")]
async fn jobs_survive_reconnection_and_expired_leases_ignore_stale_acknowledgments(pool: PgPool) {
    let db = database(pool);
    let owner = Uuid::new_v4().to_string();
    let data = data();
    db.save_match_for_upload(&data, Some(&owner)).await.expect("save");
    let original = job(&db, &owner).await;
    let replacement = database(
        PgPool::connect_with((*db.pool.connect_options()).clone())
            .await
            .expect("reconnect"),
    );
    assert!(replacement.claim_match_upload(&owner).await.expect("leased").is_none());
    sqlx::query("UPDATE match_upload_outbox SET lease_until = now() - interval '1 second'")
        .execute(&db.pool)
        .await
        .expect("simulate expired lease");
    let recovered = job(&replacement, &owner).await;
    assert_ne!(recovered.lease_token, original.lease_token);
    assert_eq!(recovered.payload, original.payload);
    db.finish_match_upload(&original, None, 0, false)
        .await
        .expect("stale ack");
    assert_eq!(
        db.match_upload_status(data.mtga_match.id())
            .await
            .expect("status")
            .expect("job")
            .state,
        "pending"
    );
    replacement
        .finish_match_upload(&recovered, None, 0, false)
        .await
        .expect("ack");
    assert_eq!(
        db.match_upload_status(data.mtga_match.id())
            .await
            .expect("status")
            .expect("job")
            .state,
        "sent"
    );
    replacement.pool.close().await;
}

#[sqlx::test(migrations = "./migrations/postgres")]
async fn unchanged_replays_do_not_reupload_and_stale_completion_preserves_new_payload(pool: PgPool) {
    let db = database(pool);
    let owner = Uuid::new_v4().to_string();
    let mut data = data();
    db.save_match_for_upload(&data, Some(&owner)).await.expect("save");
    let first = job(&db, &owner).await;
    db.finish_match_upload(&first, None, 0, false).await.expect("ack");
    db.save_match_for_upload(&data, Some(&owner)).await.expect("replay");
    assert!(db.claim_match_upload(&owner).await.expect("sent").is_none());

    data.mtga_match = MTGAMatch::new(data.mtga_match.id(), 1, "player", "opponent");
    db.save_match_for_upload(&data, Some(&owner))
        .await
        .expect("new timestamp fallback");
    assert!(
        db.claim_match_upload(&owner)
            .await
            .expect("timestamp does not requeue")
            .is_none()
    );

    data.decks[0] = Deck::new("Found Deck".into(), 1, vec![789], vec![]);
    db.save_match_for_upload(&data, Some(&owner))
        .await
        .expect("changed replay");
    let in_flight = job(&db, &owner).await;
    data.decks[0] = Deck::new("Found Deck".into(), 1, vec![999], vec![]);
    db.save_match_for_upload(&data, Some(&owner))
        .await
        .expect("newer replay");
    assert!(db.claim_match_upload(&owner).await.expect("retain lease").is_none());
    db.finish_match_upload(&in_flight, None, 0, false)
        .await
        .expect("old completion");
    let next = job(&db, &owner).await;
    assert!(next.revision > in_flight.revision);
    let snapshot: MatchData = serde_json::from_str(&next.payload).expect("payload");
    assert_eq!(snapshot.decks[0].mainboard(), &[999]);
}

#[sqlx::test(migrations = "./migrations/postgres")]
async fn jobs_are_bound_to_accounts_and_unassigned_jobs_require_explicit_queueing(pool: PgPool) {
    let db = database(pool);
    let a = Uuid::new_v4().to_string();
    let b = Uuid::new_v4().to_string();
    let data = data();
    db.save_match_for_upload(&data, None).await.expect("signed out save");
    assert!(db.claim_match_upload(&a).await.expect("unassigned").is_none());
    db.save_match_for_upload(&data, Some(&a))
        .await
        .expect("replay after login");
    assert!(db.claim_match_upload(&a).await.expect("still unassigned").is_none());
    assert!(
        db.queue_match_upload(data.mtga_match.id(), &a)
            .await
            .expect("explicit queue")
    );
    assert!(db.claim_match_upload(&b).await.expect("other account").is_none());
    assert!(matches!(
        db.queue_match_upload(data.mtga_match.id(), &b).await,
        Err(Error::UploadAccountConflict)
    ));
    let mut changed = data.clone();
    changed.decks.clear();
    assert!(matches!(
        db.save_match_for_upload(&changed, Some(&b)).await,
        Err(Error::UploadAccountConflict)
    ));
    let queued = job(&db, &a).await;
    let snapshot: MatchData = serde_json::from_str(&queued.payload).expect("payload");
    assert_eq!(snapshot.decks[0].mainboard(), &[123]);
    assert_eq!(snapshot.mulligans[0].hand(), "[123]");
    assert_eq!(snapshot.results[0].winning_team_id(), 1);
    assert_eq!(snapshot.event_logs.len(), 1);
}

#[sqlx::test(migrations = "./migrations/postgres")]
async fn failure_schedules_retry_and_permanent_failure_waits_for_manual_retry(pool: PgPool) {
    let db = database(pool);
    let owner = Uuid::new_v4().to_string();
    let data = data();
    db.save_match_for_upload(&data, Some(&owner)).await.expect("save");
    let first = job(&db, &owner).await;
    db.finish_match_upload(&first, Some("offline"), 300, false)
        .await
        .expect("retry");
    assert!(db.claim_match_upload(&owner).await.expect("delayed").is_none());
    sqlx::query("UPDATE match_upload_outbox SET next_attempt_at = now()")
        .execute(&db.pool)
        .await
        .expect("time passes");
    let second = job(&db, &owner).await;
    assert_eq!(second.attempts, 2);
    db.finish_match_upload(&second, Some("permission denied"), 0, true)
        .await
        .expect("blocked");
    assert!(db.claim_match_upload(&owner).await.expect("blocked").is_none());
    assert_eq!(
        db.match_upload_status(data.mtga_match.id())
            .await
            .expect("status")
            .expect("job")
            .last_error
            .as_deref(),
        Some("permission denied")
    );
    db.queue_match_upload(data.mtga_match.id(), &owner)
        .await
        .expect("retry");
    assert_eq!(job(&db, &owner).await.attempts, 1);
}

#[sqlx::test(migrations = "./migrations/postgres")]
async fn server_and_download_writes_do_not_queue_but_legacy_local_matches_can_be_queued(pool: PgPool) {
    let db = database(pool);
    let data = data();
    db.upsert_match_data(
        &data.mtga_match,
        &data.decks,
        &data.mulligans,
        &data.results,
        &data.opponent_deck.cards,
        &data.event_logs,
        None,
    )
    .await
    .expect("ordinary write");
    assert!(
        db.match_upload_status(data.mtga_match.id())
            .await
            .expect("status")
            .is_none()
    );
    let owner = Uuid::new_v4().to_string();
    db.queue_match_upload(data.mtga_match.id(), &owner)
        .await
        .expect("queue legacy");
    let queued = job(&db, &owner).await;
    let snapshot: MatchData = serde_json::from_str(&queued.payload).expect("snapshot");
    assert_eq!(snapshot.decks, data.decks);
    assert_eq!(snapshot.opponent_deck, data.opponent_deck);
    assert_eq!(snapshot.results.len(), data.results.len());
    assert!(
        !db.queue_match_upload(&Uuid::new_v4().to_string(), &owner)
            .await
            .expect("missing")
    );
}

#[sqlx::test(migrations = "./migrations/postgres")]
async fn concurrent_workers_cannot_lease_the_same_job(pool: PgPool) {
    let db = database(pool);
    let owner = Uuid::new_v4().to_string();
    db.save_match_for_upload(&data(), Some(&owner)).await.expect("save");
    let (a, b) = tokio::join!(db.claim_match_upload(&owner), db.claim_match_upload(&owner));
    assert_ne!(a.expect("first claim").is_some(), b.expect("second claim").is_some());
}
