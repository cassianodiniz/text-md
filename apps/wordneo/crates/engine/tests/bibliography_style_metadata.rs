//! Public JSON/model and command contracts for imported bibliography identity.
//! No dependency on a newly introduced Rust field is needed for this RED.

use std::io::{Cursor, Read};

use serde_json::{Value, json};
use wordcraft_doc::Document;
use wordcraft_engine::Session;

const APA6: &str = "\\APASixthEditionOfficeOnline.xsl";

fn imported_session() -> Session {
    let mut value = serde_json::to_value(Document::new()).unwrap();
    value["settings"]["bibStyle"] = json!("APA");
    value["settings"]["bibStyleMetadata"] = json!({"styleName": "APA", "selectedStyle": APA6, "version": "6"});
    Session::new(serde_json::from_value(value).unwrap())
}

fn metadata(session: &Session) -> Value {
    serde_json::to_value(&session.doc).unwrap()["settings"]["bibStyleMetadata"].clone()
}

fn saved_bibliography(session: &Session) -> String {
    let bytes = wordcraft_docx::write(&session.doc).unwrap();
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut stores = Vec::new();
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        if !file.name().starts_with("customXml/") || !file.name().ends_with(".xml") {
            continue;
        }
        let mut xml = String::new();
        file.read_to_string(&mut xml).unwrap();
        if xml.contains("<b:Sources") {
            stores.push(xml);
        }
    }
    assert_eq!(stores.len(), 1);
    stores.remove(0)
}

#[test]
fn selecting_the_same_apa_style_preserves_imported_identity_through_edits_and_reopen() {
    let mut session = imported_session();
    session.run("references.citationStyle", &json!({"style": "APA"})).unwrap();
    session
        .run(
            "references.sources",
            &json!({"add": {"tag": "MetadataBook", "kind": "book", "author": "Fixture, Ari", "title": "Synthetic style metadata source"}}),
        )
        .unwrap();
    session.run("references.bibliography", &json!({"title": "Synthetic metadata bibliography"})).unwrap();
    session.run("references.updateFields", &json!({})).unwrap();
    assert_eq!(metadata(&session), json!({"styleName": "APA", "selectedStyle": APA6, "version": "6"}));
    let xml = saved_bibliography(&session);
    assert!(xml.contains(&format!("SelectedStyle=\"{APA6}\"")) && xml.contains("Version=\"6\""));
    let reopened = Session::new(wordcraft_docx::read(&wordcraft_docx::write(&session.doc).unwrap()).unwrap());
    assert_eq!(metadata(&reopened), metadata(&session));
    assert_eq!(reopened.doc.sources, session.doc.sources);
}

#[test]
fn selecting_a_different_style_invalidates_imported_identity_without_reviving_it() {
    let mut session = imported_session();
    assert_eq!(metadata(&session)["selectedStyle"], APA6, "the public model must first retain the imported metadata");
    session.run("references.citationStyle", &json!({"style": "IEEE"})).unwrap();
    assert!(metadata(&session).is_null(), "changing styles removes the imported identity");
    let xml = saved_bibliography(&session);
    assert!(xml.contains("StyleName=\"IEEE\"") && xml.contains("SelectedStyle=\"\\IEEE.XSL\""));
    assert!(!xml.contains("Version=") && !xml.contains(APA6));
    let mut session = Session::new(wordcraft_docx::read(&wordcraft_docx::write(&session.doc).unwrap()).unwrap());
    session.run("references.citationStyle", &json!({"style": "APA"})).unwrap();
    let xml = saved_bibliography(&session);
    assert!(xml.contains("SelectedStyle=\"\\APA.XSL\""));
    assert!(!xml.contains("Version=") && !xml.contains(APA6), "switching back uses the native default, not discarded metadata");
}
