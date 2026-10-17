//! Behavioral coverage of a bibliography result becoming empty and populated again.
//! Synthetic in-memory documents only; no Word-produced fixture or runtime is used.

use serde_json::{Value, json};
use wordcraft_doc::{Document, StoryRef};
use wordcraft_engine::Session;

fn source() -> Value {
    json!({
        "tag": "RangeEdgeSource", "kind": "book", "author": "Fixture, Ari",
        "title": "A synthetic range edge source", "year": "2030", "publisher": "Fictional Press"
    })
}

fn reopened(session: Session) -> Session {
    Session::new(wordcraft_docx::read(&wordcraft_docx::write(&session.doc).unwrap()).unwrap())
}

#[test]
fn bibliography_populated_empty_reopen_and_populated_again_has_no_extra_entry_paragraph() {
    let mut session = Session::new(Document::new());
    session.run("references.sources", &json!({"add": source()})).unwrap();
    session.run("references.bibliography", &json!({"title": "Range edge bibliography"})).unwrap();
    let mut session = reopened(session);
    let initial_blocks = session.doc.body.len();
    for cycle in 0..2 {
        session.run("references.sources", &json!({"remove": "RangeEdgeSource"})).unwrap();
        session.run("references.updateFields", &json!({})).unwrap();
        assert!(!session.doc.plain_text(StoryRef::Body).contains("A synthetic range edge source"));
        session = reopened(session);
        session.run("references.sources", &json!({"add": source()})).unwrap();
        session.run("references.updateFields", &json!({})).unwrap();
        session = reopened(session);
        let text = session.doc.plain_text(StoryRef::Body);
        assert_eq!(text.matches("Range edge bibliography").count(), 1, "cycle{cycle}: {text}");
        assert_eq!(text.matches("A synthetic range edge source").count(), 1, "cycle{cycle}: {text}");
        let empty_entries = session
            .doc
            .body
            .iter()
            .filter_map(|block| block.as_para())
            .filter(|p| p.props.style.as_deref() == Some("ListEntry") && p.plain_text().trim().is_empty())
            .count();
        assert_eq!(
            (session.doc.body.len(), empty_entries),
            (initial_blocks, 0),
            "cycle{cycle}: re-populating the same field must replace its empty anchor, not retain it as an extra title/entry"
        );
    }
}
