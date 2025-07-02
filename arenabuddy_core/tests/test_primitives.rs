use arenabuddy_core::mtga_events::primitives::AnnotationType;
use serde_json::json;

#[test]
fn test_annotation_type() {
    let annotation_type_strings = json! {[
        "AnnotationType_ResolutionStart",
        "AnnotationType_ResolutionComplete",
        "AnnotationType_CardRevealed"
    ]};
    let _: Vec<AnnotationType> =
        serde_json::from_value(annotation_type_strings).expect("valid annotation type");
}
