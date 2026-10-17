//! Staged outside SOURCE; not compiled or executed.
//! Word008 single-variable control: only the empty IEEE table anchor's
//! ListEntry -> Normal changes table Y by 4pt; do not change cell/heading styles.

use std::io::{Cursor, Read};

use regex::Regex;
use serde_json::json;
use wordcraft_doc::{Document, InlineObject, StoryRef};
use wordcraft_engine::Session;

const HEADING: &str = "Synthetic anchor references";
const BOOK: &str = "A fictional book for the IEEE anchor contract";
const AFTER: &str = "Independent text after the IEEE field.";

fn native_book_bibliography() -> Session {
    let mut session = Session::new(Document::new());
    session.run("text.insert", &json!({"text": "Synthetic citation: "})).unwrap();
    session
        .run(
            "references.sources",
            &json!({"add": {
                "tag": "AnchorBook2030", "kind": "book", "author": "Fixture, Ari",
                "title": BOOK, "year": "2030", "publisher": "Fictional Press"
            }}),
        )
        .unwrap();
    session.run("references.citationStyle", &json!({"style": "IEEE"})).unwrap();
    session.run("references.citation", &json!({"tag": "AnchorBook2030", "pages": "5"})).unwrap();
    session.run("text.newParagraph", &json!({})).unwrap();
    session.run("references.bibliography", &json!({"title": HEADING})).unwrap();
    session.run("caret.docEnd", &json!({})).unwrap();
    session.run("text.insert", &json!({"text": AFTER})).unwrap();
    session
}

fn assert_populated_ieee_anchor(doc: &Document) {
    assert!(!doc.has_unbalanced_field_ranges());
    assert_eq!(doc.sources.len(), 1);
    let ranges = doc.field_ranges(StoryRef::Body);
    let bibliography: Vec<_> = ranges.iter().filter(|r| r.instr.split_whitespace().next() == Some("BIBLIOGRAPHY")).collect();
    assert_eq!(bibliography.len(), 1);
    let range = bibliography[0];
    assert_eq!((range.start.path.depth(), range.end.path.depth()), (0, 0));
    let start = range.start.path.last();
    let end = range.end.path.last();
    let anchor = doc.body[start].as_para().unwrap();
    assert_eq!(range.start.off, 0);
    assert!(anchor.plain_text().trim().is_empty(), "the field anchor has no visible entry/title text");
    assert_eq!(anchor.props.style.as_deref(), Some("Normal"), "Word's populated IEEE table anchor uses Normal, not ListEntry.after4");
    assert!(matches!(anchor.object_at(range.start.off), Some(InlineObject::FieldStart { locked: false, .. })));
    assert!(matches!(doc.body[end].as_para().unwrap().object_at(range.end.off), Some(InlineObject::FieldEnd)));

    let tables: Vec<_> = doc.body.iter().enumerate().filter_map(|(i, b)| b.as_table().map(|t| (i, t))).collect();
    assert_eq!(tables.len(), 1);
    let (table_index, table) = tables[0];
    assert!(start < table_index && table_index < end, "the whole table belongs to the field result");
    assert_eq!((table.rows.len(), table.rows[0].cells.len()), (1, 2));
    let number = table.rows[0].cells[0].blocks[0].as_para().unwrap();
    let entry = table.rows[0].cells[1].blocks[0].as_para().unwrap();
    assert_eq!(number.plain_text().trim(), "[1]");
    assert_eq!(entry.plain_text().matches(BOOK).count(), 1);
    for cell in &table.rows[0].cells {
        for block in &cell.blocks {
            if let Some(para) = block.as_para() {
                assert_eq!(para.props.style.as_deref(), Some("Bibliografia"), "only the empty body anchor changes style");
            }
        }
    }
    let text = doc.plain_text(StoryRef::Body);
    for value in [HEADING, BOOK, AFTER] {
        assert_eq!(text.matches(value).count(), 1, "refresh/reopen preserves exactly one occurrence: {value}");
    }
    let (heading_index, heading) =
        doc.body.iter().enumerate().filter_map(|(i, b)| b.as_para().map(|p| (i, p))).find(|(_, p)| p.text.contains(HEADING)).unwrap();
    assert_eq!(heading.props.style.as_deref(), Some("Heading1"));
    assert!(heading_index < start, "the heading remains outside the result");
    let (after_index, after) =
        doc.body.iter().enumerate().filter_map(|(i, b)| b.as_para().map(|p| (i, p))).find(|(_, p)| p.text.contains(AFTER)).unwrap();
    assert!(after_index > end || (after_index == end && after.text.find(AFTER).unwrap() >= range.end.off + wordcraft_doc::para::OBJ.len_utf8()));
}

fn exported_and_reopened(session: Session) -> Session {
    let bytes = wordcraft_docx::write(&session.doc).unwrap();
    let mut archive = zip::ZipArchive::new(Cursor::new(&bytes)).unwrap();
    let mut xml = String::new();
    archive.by_name("word/document.xml").unwrap().read_to_string(&mut xml).unwrap();
    let paragraphs = Regex::new(r"(?s)<w:p(?:\s[^>]*)?>.*?</w:p>").unwrap();
    let anchors: Vec<_> = paragraphs.find_iter(&xml).filter(|p| p.as_str().contains("BIBLIOGRAPHY")).collect();
    assert_eq!(anchors.len(), 1);
    let anchor = anchors[0].as_str();
    assert!(Regex::new(r#"<w:pStyle\b[^>]*\bw:val="Normal""#).unwrap().is_match(anchor));
    assert!(anchor.contains("w:fldCharType=\"begin\"") && anchor.contains("w:fldCharType=\"separate\""));
    assert!(!anchor.contains("w:fldCharType=\"end\""), "the table result follows separate, with end in the later body paragraph");
    Session::new(wordcraft_docx::read(&bytes).unwrap())
}

#[test]
fn native_ieee_populated_table_anchor_uses_normal_through_reopen_and_refresh() {
    let session = native_book_bibliography();
    assert_populated_ieee_anchor(&session.doc);
    let mut session = exported_and_reopened(session);
    assert_populated_ieee_anchor(&session.doc);
    let blocks = session.doc.body.len();
    for _ in 0..2 {
        session.run("references.updateFields", &json!({})).unwrap();
        assert_populated_ieee_anchor(&session.doc);
        session = exported_and_reopened(session);
        assert_populated_ieee_anchor(&session.doc);
        assert_eq!(session.doc.body.len(), blocks, "the style correction must not accumulate boundary paragraphs");
    }
}
