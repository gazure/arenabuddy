use super::*;

fn objects(frames: Vec<Frame>) -> Vec<String> {
    frames
        .into_iter()
        .map(|frame| match frame {
            Frame::Object(bytes) => String::from_utf8(bytes).expect("UTF-8"),
            Frame::Rejected(reason) => panic!("unexpected rejection: {reason}"),
        })
        .collect()
}

#[test]
fn preserves_spaces_braces_escapes_and_unicode_at_every_split() {
    let value = serde_json::json!({
        "player": "A Player β🧙",
        "request": "{\"text\":\"a } and { and \\\" quote\"}",
        "path": "C:\\Arena\\",
        "array": [{"name": "A B"}, {"name": "} {"}],
    });
    let json = serde_json::to_string_pretty(&value).expect("JSON");
    for split in 0..=json.len() {
        let mut decoder = JsonFramer::default();
        let mut frames = decoder.push(&json.as_bytes()[..split]);
        frames.extend(decoder.push(&json.as_bytes()[split..]));
        assert_eq!(objects(frames), std::slice::from_ref(&json), "split at byte {split}");
        assert!(decoder.finish().is_none());
    }
}

#[test]
fn frames_multiple_objects_and_multiline_data_without_modification() {
    let mut decoder = JsonFramer::default();
    let frames = decoder.push(b"log prefix { \"key\" : \"A B\" } noise\n{\n\"outer\":{\"inner\":1}\n} tail");
    assert_eq!(
        objects(frames),
        ["{ \"key\" : \"A B\" }", "{\n\"outer\":{\"inner\":1}\n}"]
    );
}

#[test]
fn ignores_plain_text_and_stray_closing_braces() {
    let mut decoder = JsonFramer::default();
    assert!(decoder.push(b"plain log line } ]\n").is_empty());
    assert_eq!(objects(decoder.push(b"{}")), ["{}"]);
}

#[test]
fn an_incomplete_object_is_reported_once_and_does_not_join_the_next_file() {
    let mut decoder = JsonFramer::default();
    assert!(decoder.push(b"{\"player\":\"unfinished\\").is_empty());
    assert!(matches!(decoder.finish(), Some(Frame::Rejected(_))));
    assert!(decoder.finish().is_none());
    assert_eq!(objects(decoder.push(b"{\"new\":true}")), ["{\"new\":true}"]);
}

#[test]
fn oversized_frames_are_bounded_and_skipped_without_emitting_nested_objects() {
    let mut decoder = JsonFramer::default();
    decoder.push(b"{\"large\":\"");
    let frames = decoder.push(&vec![b'a'; MAX_FRAME_BYTES]);
    assert!(matches!(frames.as_slice(), [Frame::Rejected(_)]));
    assert!(decoder.bytes.is_empty());
    let frames = decoder.push(b"\",\"nested\":{\"not_an_event\":true}} {\"next\":true}");
    assert_eq!(objects(frames), ["{\"next\":true}"]);
}
