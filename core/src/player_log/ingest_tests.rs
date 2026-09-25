use super::*;

#[tokio::test]
async fn zero_poll_interval_is_rejected_without_panicking() {
    let dir = tempfile::tempdir().expect("directory");
    let config = IngestionConfig::new(dir.path().join("Player.log")).with_poll_interval(Duration::ZERO);
    assert!(LogIngestionService::new(config).await.is_err());
}

async fn receive(rx: &mut mpsc::UnboundedReceiver<IngestionEvent>) -> IngestionEvent {
    tokio::time::timeout(Duration::from_secs(5), rx.recv())
        .await
        .expect("event timeout")
        .expect("event channel")
}

fn draft(id: &str) -> String {
    serde_json::json!({"draftId":id,"SelfPick":1,"SelfPack":1,"PackCards":"A B"}).to_string()
}

#[tokio::test]
async fn follows_appends_and_replacement_without_replaying_events() {
    use tokio::io::AsyncWriteExt;
    let dir = tempfile::tempdir().expect("directory");
    let path = dir.path().join("Player.log");
    tokio::fs::write(&path, draft("first")).await.expect("write");
    let (events, mut rx) = mpsc::unbounded_channel();
    let (stop, shutdown) = mpsc::unbounded_channel();
    let service =
        LogIngestionService::new(IngestionConfig::new(path.clone()).with_poll_interval(Duration::from_millis(20)))
            .await
            .expect("service")
            .with_shutdown_receiver(shutdown)
            .with_event_callback(Arc::new(move |event| {
                let _ = events.send(event);
                Box::pin(async {})
            }));
    let task = tokio::spawn(service.start());
    assert!(matches!(receive(&mut rx).await, IngestionEvent::DraftNotify(e) if e.draft_id == "first"));
    tokio::fs::write(dir.path().join("unrelated.log"), "sibling")
        .await
        .expect("sibling");
    let mut file = tokio::fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .await
        .expect("append");
    file.write_all(draft("second").as_bytes()).await.expect("write");
    file.flush().await.expect("flush");
    assert!(matches!(receive(&mut rx).await, IngestionEvent::DraftNotify(e) if e.draft_id == "second"));
    tokio::fs::rename(&path, dir.path().join("old.log"))
        .await
        .expect("rename");
    tokio::fs::write(&path, draft("third")).await.expect("replacement");
    assert!(matches!(receive(&mut rx).await, IngestionEvent::LogRotated));
    assert!(matches!(receive(&mut rx).await, IngestionEvent::DraftNotify(e) if e.draft_id == "third"));
    stop.send(()).expect("shutdown");
    tokio::time::timeout(Duration::from_secs(5), task)
        .await
        .expect("shutdown timeout")
        .expect("join")
        .expect("service result");
    assert!(rx.try_recv().is_err());
}

#[tokio::test]
async fn missing_parent_falls_back_to_polling_and_sender_drop_stops_service() {
    let dir = tempfile::tempdir().expect("directory");
    let parent = dir.path().join("not-yet-created");
    let path = parent.join("Player.log");
    let (events, mut rx) = mpsc::unbounded_channel();
    let (stop, shutdown) = mpsc::unbounded_channel();
    let service =
        LogIngestionService::new(IngestionConfig::new(path.clone()).with_poll_interval(Duration::from_millis(20)))
            .await
            .expect("service")
            .with_shutdown_receiver(shutdown)
            .with_event_callback(Arc::new(move |event| {
                let _ = events.send(event);
                Box::pin(async {})
            }));
    let task = tokio::spawn(service.start());
    tokio::task::yield_now().await;
    tokio::fs::create_dir(&parent).await.expect("parent");
    tokio::fs::write(&path, draft("created")).await.expect("create");
    assert!(matches!(receive(&mut rx).await, IngestionEvent::DraftNotify(e) if e.draft_id == "created"));
    drop(stop);
    tokio::time::timeout(Duration::from_secs(5), task)
        .await
        .expect("shutdown timeout")
        .expect("join")
        .expect("service result");
}

#[tokio::test]
async fn finite_service_reports_incomplete_json_and_exits() {
    let dir = tempfile::tempdir().expect("directory");
    let path = dir.path().join("Player.log");
    tokio::fs::write(&path, "{\"unfinished\":").await.expect("write");
    let (events, mut rx) = mpsc::unbounded_channel();
    let service = LogIngestionService::new(IngestionConfig::new(path).with_follow(false))
        .await
        .expect("service")
        .with_event_callback(Arc::new(move |event| {
            let _ = events.send(event);
            Box::pin(async {})
        }));
    tokio::time::timeout(Duration::from_secs(5), service.start())
        .await
        .expect("timeout")
        .expect("result");
    assert!(matches!(receive(&mut rx).await, IngestionEvent::ParseError(_)));
    assert!(rx.recv().await.is_none());
}

struct DraftCollector(mpsc::UnboundedSender<MTGADraft>);

#[async_trait::async_trait]
impl DraftWriter for DraftCollector {
    async fn write(&mut self, draft: &MTGADraft) -> Result<()> {
        self.0.send(draft.clone()).expect("collect draft");
        Ok(())
    }
}

fn pack_event(pack_number: u8, selection: u8) -> BusinessEvent {
    serde_json::from_value(serde_json::json!({
        "DraftId":"c66fc47c-0309-4c6e-b109-47fcbac5a06b",
        "EventId":"PremierDraft_FDN_20250101", "SeatNumber":1,
        "PackNumber":pack_number, "PickNumber":selection, "PickGrpId":123,
        "CardsInPack":[123], "AutoPick":false, "TimeRemainingOnPick":10,
        "EventType":1, "EventTime":"2026-01-01T00:00:00Z"
    }))
    .expect("business event")
}

#[tokio::test]
async fn rotation_discards_old_draft_packs_and_keeps_the_writer() {
    let dir = tempfile::tempdir().expect("directory");
    let path = dir.path().join("Player.log");
    tokio::fs::write(&path, "").await.expect("write");
    let (drafts, mut collected) = mpsc::unbounded_channel();
    let (events, mut rx) = mpsc::unbounded_channel();
    let (stop, shutdown) = mpsc::unbounded_channel();
    let mut service = LogIngestionService::new(
        IngestionConfig::new(path.clone())
            .with_rotation_watch(false)
            .with_poll_interval(Duration::from_millis(20)),
    )
    .await
    .expect("service")
    .add_draft_writer(Box::new(DraftCollector(drafts)))
    .with_shutdown_receiver(shutdown)
    .with_event_callback(Arc::new(move |event| {
        let _ = events.send(event);
        Box::pin(async {})
    }));
    service
        .draft_builder
        .process_event(&pack_event(1, 1))
        .await
        .expect("old pack");
    tokio::fs::rename(&path, dir.path().join("old.log"))
        .await
        .expect("rename");
    let request = serde_json::to_string(&pack_event(3, 13)).expect("serialize");
    tokio::fs::write(&path, serde_json::json!({"id":"event", "request":request}).to_string())
        .await
        .expect("replacement");
    let task = tokio::spawn(service.start());
    assert!(matches!(receive(&mut rx).await, IngestionEvent::LogRotated));
    let result = tokio::time::timeout(Duration::from_secs(5), collected.recv())
        .await
        .expect("draft timeout")
        .expect("draft");
    assert_eq!(result.packs().len(), 1);
    assert_eq!(result.packs()[0].pack_number(), 3);
    stop.send(()).expect("shutdown");
    tokio::time::timeout(Duration::from_secs(5), task)
        .await
        .expect("shutdown timeout")
        .expect("join")
        .expect("service result");
}
