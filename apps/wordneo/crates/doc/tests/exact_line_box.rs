//! Model contract through existing public JSON APIs, before a new Rust field exists.

use serde_json::{Value, json};
use wordcraft_doc::props::ParaProps;

#[test]
fn paragraph_text_alignment_five_tokens_survive_public_json() {
    for token in ["auto", "baseline", "top", "center", "bottom"] {
        let input = json!({"textAlignment": token, "spaceAfter": 6.0});
        let mut props: ParaProps = serde_json::from_value(input.clone()).unwrap();
        for _ in 0..2 {
            let output = serde_json::to_value(&props).unwrap();
            assert_eq!(output, input, "alignment token {token} remains explicit");
            props = serde_json::from_value(output).unwrap();
        }
    }
}

#[test]
fn omitted_alignment_is_optional_and_is_not_invented_by_json() {
    let props: ParaProps = serde_json::from_value(json!({})).unwrap();
    assert_eq!(props, ParaProps::default());
    assert_eq!(serde_json::to_value(props).unwrap(), json!({}));
}

#[test]
fn overlay_preserves_inheritance_and_explicit_auto_cancels_it() {
    let mut base: ParaProps = serde_json::from_value(json!({"textAlignment": "top"})).unwrap();
    base.overlay(&ParaProps::default());
    assert_eq!(serde_json::to_value(&base).unwrap()["textAlignment"], Value::String("top".into()));
    let explicit: ParaProps = serde_json::from_value(json!({"textAlignment": "auto"})).unwrap();
    base.overlay(&explicit);
    assert_eq!(serde_json::to_value(base).unwrap()["textAlignment"], Value::String("auto".into()));
}
