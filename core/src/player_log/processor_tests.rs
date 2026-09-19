use tokio::{
    fs::{self, OpenOptions},
    io::AsyncWriteExt,
};

use super::*;

fn draft(id: &str) -> String {
    serde_json::json!({"draftId":id, "SelfPick":1, "SelfPack":1, "PackCards":"A B β {quoted}"}).to_string()
}

async fn next_draft(processor: &mut PlayerLogProcessor) -> String {
    loop {
        match processor.next_item().await.expect("read") {
            LogItem::Event(ParseOutput::DraftNotify(event)) => {
                assert_eq!(event.pack_cards, "A B β {quoted}");
                return event.draft_id;
            }
            LogItem::Progress => {}
            _ => panic!("expected a draft event"),
        }
    }
}

async fn append(path: &Path, bytes: &[u8]) {
    let mut file = OpenOptions::new().append(true).open(path).await.expect("open append");
    file.write_all(bytes).await.expect("append");
    file.flush().await.expect("flush");
}

#[tokio::test]
async fn appends_do_not_replay_old_events_and_partial_utf8_survives_eof() {
    let dir = tempfile::tempdir().expect("directory");
    let path = dir.path().join("Player.log");
    fs::write(&path, draft("first")).await.expect("write");
    let mut processor = PlayerLogProcessor::with_follow(&path, true).await.expect("processor");
    assert_eq!(next_draft(&mut processor).await, "first");
    assert!(matches!(processor.next_item().await.expect("EOF"), LogItem::Eof));
    let json = draft("second");
    let split = json.find('β').expect("unicode") + 1;
    append(&path, &json.as_bytes()[..split]).await;
    assert!(matches!(
        processor.next_item().await.expect("partial"),
        LogItem::Progress
    ));
    assert!(matches!(processor.next_item().await.expect("EOF"), LogItem::Eof));
    append(&path, &json.as_bytes()[split..]).await;
    assert_eq!(next_draft(&mut processor).await, "second");
    assert!(matches!(processor.next_item().await.expect("EOF"), LogItem::Eof));
}

#[tokio::test]
async fn replacement_drains_old_file_and_handles_a_missing_path() {
    let dir = tempfile::tempdir().expect("directory");
    let path = dir.path().join("Player.log");
    let old = dir.path().join("Player-prev.log");
    fs::write(&path, draft("first")).await.expect("write");
    let mut processor = PlayerLogProcessor::with_follow(&path, true).await.expect("processor");
    assert_eq!(next_draft(&mut processor).await, "first");
    fs::rename(&path, &old).await.expect("rename");
    assert!(matches!(
        processor.next_item().await.expect("missing path"),
        LogItem::Eof
    ));
    append(&old, draft("last-old").as_bytes()).await;
    fs::write(&path, draft("new")).await.expect("replacement");
    assert_eq!(next_draft(&mut processor).await, "last-old");
    assert!(matches!(
        processor.next_item().await.expect("boundary"),
        LogItem::Boundary
    ));
    assert_eq!(next_draft(&mut processor).await, "new");
    assert!(matches!(processor.next_item().await.expect("EOF"), LogItem::Eof));
}

#[tokio::test]
async fn truncation_and_fast_regrowth_reset_before_reading_new_bytes() {
    let dir = tempfile::tempdir().expect("directory");
    let path = dir.path().join("Player.log");
    fs::write(&path, draft("initial-long-id")).await.expect("write");
    let mut processor = PlayerLogProcessor::with_follow(&path, true).await.expect("processor");
    assert_eq!(next_draft(&mut processor).await, "initial-long-id");
    for id in ["short", "a-much-longer-replacement-than-the-old-file"] {
        fs::write(&path, draft(id)).await.expect("truncate and regrow");
        assert!(matches!(
            processor.next_item().await.expect("boundary"),
            LogItem::Boundary
        ));
        assert_eq!(next_draft(&mut processor).await, id);
        assert!(matches!(processor.next_item().await.expect("EOF"), LogItem::Eof));
    }
}

#[tokio::test]
async fn same_length_rewrite_is_detected_by_content() {
    let dir = tempfile::tempdir().expect("directory");
    let path = dir.path().join("Player.log");
    fs::write(&path, draft("one")).await.expect("write");
    let mut processor = PlayerLogProcessor::with_follow(&path, true).await.expect("processor");
    assert_eq!(next_draft(&mut processor).await, "one");
    fs::write(&path, draft("two")).await.expect("rewrite");
    assert!(matches!(
        processor.next_item().await.expect("boundary"),
        LogItem::Boundary
    ));
    assert_eq!(next_draft(&mut processor).await, "two");
}

#[tokio::test]
async fn incomplete_json_is_discarded_at_a_boundary() {
    let dir = tempfile::tempdir().expect("directory");
    let path = dir.path().join("Player.log");
    fs::write(&path, b"{\"unfinished\":\"").await.expect("write");
    let mut processor = PlayerLogProcessor::with_follow(&path, true).await.expect("processor");
    assert!(matches!(
        processor.next_item().await.expect("partial"),
        LogItem::Progress
    ));
    fs::rename(&path, dir.path().join("old.log")).await.expect("rename");
    fs::write(&path, draft("new")).await.expect("replacement");
    assert!(matches!(
        processor.next_item().await,
        Err(crate::Error::Parse(ParseError::Error(_)))
    ));
    assert!(matches!(
        processor.next_item().await.expect("boundary"),
        LogItem::Boundary
    ));
    assert_eq!(next_draft(&mut processor).await, "new");
}

#[tokio::test]
async fn following_waits_for_initial_creation_but_snapshot_requires_a_file() {
    let dir = tempfile::tempdir().expect("directory");
    let path = dir.path().join("Player.log");
    assert!(PlayerLogProcessor::try_new(&path).await.is_err());
    let mut processor = PlayerLogProcessor::with_follow(&path, true).await.expect("follow");
    assert!(matches!(processor.next_item().await.expect("missing"), LogItem::Eof));
    fs::write(&path, draft("created")).await.expect("create");
    assert_eq!(next_draft(&mut processor).await, "created");
    assert!(PlayerLogProcessor::with_follow(dir.path(), true).await.is_err());
}

#[tokio::test]
async fn snapshot_stops_at_its_initial_length_and_reports_incomplete_final_json_once() {
    let dir = tempfile::tempdir().expect("directory");
    let path = dir.path().join("Player.log");
    fs::write(&path, draft("initial")).await.expect("write");
    let mut processor = PlayerLogProcessor::try_new(&path).await.expect("snapshot");
    append(&path, draft("later").as_bytes()).await;
    assert!(matches!(
        processor.get_next_event().await.expect("initial"),
        ParseOutput::DraftNotify(_)
    ));
    assert!(matches!(
        processor.get_next_event().await,
        Err(crate::Error::Parse(ParseError::NoEvent))
    ));

    fs::write(&path, b"{\"incomplete\":").await.expect("write");
    let mut processor = PlayerLogProcessor::try_new(&path).await.expect("snapshot");
    assert!(matches!(
        processor.get_next_event().await,
        Err(crate::Error::Parse(ParseError::Error(_)))
    ));
    assert!(matches!(
        processor.get_next_event().await,
        Err(crate::Error::Parse(ParseError::NoEvent))
    ));
}

#[tokio::test]
async fn malformed_and_invalid_utf8_frames_do_not_hide_the_next_valid_event() {
    let dir = tempfile::tempdir().expect("directory");
    let path = dir.path().join("Player.log");
    let mut bytes = b"{invalid}{\"text\":\"\xff\"}".to_vec();
    bytes.extend_from_slice(draft("valid").as_bytes());
    fs::write(&path, bytes).await.expect("write");
    let mut processor = PlayerLogProcessor::try_new(&path).await.expect("snapshot");
    for _ in 0..2 {
        assert!(processor.get_next_event().await.is_err());
    }
    assert_eq!(next_draft(&mut processor).await, "valid");
}

#[test]
fn dispatch_uses_top_level_fields_not_text_inside_strings_or_nested_objects() {
    for input in [
        r#"{"text":"clientToMatchServiceMessage greToClientEvent matchGameRoomStateChangedEvent"}"#,
        r#"{"nested":{"greToClientEvent":{}}}"#,
    ] {
        assert!(matches!(parse(input).expect("unknown event"), ParseOutput::NoEvent));
    }
    assert!(parse(r#"{"greToClientEvent":null}"#).is_err());
    assert!(parse("not json").is_err());
}

#[tokio::test]
async fn rewrite_with_identical_header_is_detected_at_the_read_checkpoint() {
    let dir = tempfile::tempdir().expect("directory");
    let path = dir.path().join("Player.log");
    let header = "header without JSON\n".repeat(100);
    fs::write(&path, format!("{header}{}", draft("one")))
        .await
        .expect("write");
    let mut processor = PlayerLogProcessor::with_follow(&path, true).await.expect("processor");
    assert_eq!(next_draft(&mut processor).await, "one");
    fs::write(&path, format!("{header}{}", draft("two")))
        .await
        .expect("rewrite");
    assert!(matches!(
        processor.next_item().await.expect("boundary"),
        LogItem::Boundary
    ));
    assert_eq!(next_draft(&mut processor).await, "two");
}
