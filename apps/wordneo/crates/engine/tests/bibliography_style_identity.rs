//! Word008 synthetic customStyle-only contrast: omitting customStyle makes the
//! newly created Bibliografia style built-in and prevents Bibliografia1 on open.
//! Evidence: candidate008/style-identity-controls/runs/
//! word-20261016T2208396768178Z-465e90c4/word-style-identity.json.
use std::io::{Cursor, Read};

use regex::Regex;
use serde_json::json;
use wordcraft_doc::Document;
use wordcraft_engine::Session;

#[test]
fn a_new_ieee_bibliography_style_keeps_the_observed_builtin_identity_in_docx() {
    let mut session = Session::new(Document::new());
    assert!(session.doc.styles.get("Bibliografia").is_none(), "this guard covers creation, not replacement of a user's existing style");
    session
        .run(
            "references.sources",
            &json!({"add": {"tag": "SyntheticIdentityBook", "kind": "book", "author": "Example, Alex",
                "title": "Synthetic bibliography style identity", "year": "2031"}}),
        )
        .unwrap();
    session.run("references.citationStyle", &json!({"style": "IEEE"})).unwrap();
    session.run("references.bibliography", &json!({"title": "Synthetic References"})).unwrap();
    let style = session.doc.styles.get("Bibliografia").expect("IEEE creates its referenced entry style");
    assert!(style.builtin, "the observed Word style is built-in; customStyle1 imports as a different Bibliografia1 style");

    let bytes = wordcraft_docx::write(&session.doc).unwrap();
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut xml = String::new();
    archive.by_name("word/styles.xml").unwrap().read_to_string(&mut xml).unwrap();
    let pattern = Regex::new(r#"<w:style\b[^>]*\bw:styleId="Bibliografia"[^>]*>"#).unwrap();
    let definitions: Vec<_> = pattern.find_iter(&xml).collect();
    assert_eq!(definitions.len(), 1, "one referenced built-in style definition");
    assert!(!definitions[0].as_str().contains("w:customStyle="), "Word's matched control omits the custom-style declaration");
}
