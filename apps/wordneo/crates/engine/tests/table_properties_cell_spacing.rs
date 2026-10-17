//! Staged contract outside SOURCE; not compiled/executed.
//! cellSpacing uses the L1/file-point value (0.75pt -> 15dxa), not Word COM's
//! display-distance conversion. Omitted key preserves; null inherits; zero overrides.

use std::io::{Cursor, Read};
use std::sync::Arc;

use regex::Regex;
use serde_json::json;
use wordcraft_doc::styles::{Style, StyleKind, TableStyleParts};
use wordcraft_doc::{Block, Document, Paragraph, Path, Pos, StoryRef, Table, para_block};
use wordcraft_engine::{Selection, Session};

const STYLE: &str = "SpacingCommandDerived";

fn fixture(direct: Option<f32>) -> Session {
    let mut doc = Document::new();
    doc.styles.styles.push(Style {
        id: "SpacingCommandBase".into(),
        kind: StyleKind::Table,
        table: Some(TableStyleParts { cell_spacing: Some(1.5), ..Default::default() }),
        ..Default::default()
    });
    doc.styles.styles.push(Style {
        id: STYLE.into(),
        kind: StyleKind::Table,
        based_on: Some("SpacingCommandBase".into()),
        table: Some(TableStyleParts::default()),
        ..Default::default()
    });
    let mut table = Table::new(1, 2, 240.0);
    table.props.style = Some(STYLE.into());
    table.props.cell_spacing = direct;
    for (cell, text) in table.rows[0].cells.iter_mut().zip(["Own fixture A", "Own fixture B"]) {
        cell.blocks = vec![para_block(Paragraph::with_text(text, Default::default()))];
    }
    doc.body = vec![Arc::new(Block::Table(table)), para_block(Paragraph::with_text("After the table.", Default::default()))];
    selected(doc)
}

fn selected(doc: Document) -> Session {
    let mut session = Session::new(doc);
    session.sel = Selection::caret(Pos { story: StoryRef::Body, path: Path(vec![0, 0, 0, 0]), off: 0 });
    session
}

fn table(session: &Session) -> &Table {
    session.doc.table(StoryRef::Body, &Path::top(0)).unwrap()
}

fn inherited(session: &Session) -> Option<f32> {
    session.doc.styles.table_style(table(session).props.style.as_deref().unwrap()).unwrap().parts.cell_spacing
}

fn assert_getter(session: &mut Session, expected: Option<f32>) {
    let state = session.run("table.properties", &json!({})).unwrap();
    match expected {
        Some(value) => assert_eq!(state["cellSpacing"], json!(value)),
        None => assert!(state.get("cellSpacing").is_none(), "None uses existing serde omission; zero must not be omitted"),
    }
    assert_eq!(table(session).props.cell_spacing, expected, "getter agrees with the document, not an echoed input");
}

fn save_reopen(session: Session, direct: Option<f32>, twips: Option<&str>) -> Session {
    let bytes = wordcraft_docx::write(&session.doc).unwrap();
    let mut archive = zip::ZipArchive::new(Cursor::new(&bytes)).unwrap();
    let mut xml = String::new();
    archive.by_name("word/document.xml").unwrap().read_to_string(&mut xml).unwrap();
    let tags: Vec<_> = Regex::new(r"<w:tblCellSpacing\b[^>]*/>").unwrap().find_iter(&xml).map(|m| m.as_str()).collect();
    match twips {
        Some(value) => {
            assert_eq!(tags.len(), 1);
            assert!(tags[0].contains(&format!("w:w=\"{value}\"")) && tags[0].contains("w:type=\"dxa\""));
        }
        None => assert!(tags.is_empty(), "inheritance removes only the direct table property"),
    }
    let mut reopened = selected(wordcraft_docx::read(&bytes).unwrap());
    assert_eq!(table(&reopened).props.cell_spacing, direct);
    assert_eq!(inherited(&reopened), Some(1.5), "the base/derived style remains intact through DOCX");
    assert_getter(&mut reopened, direct);
    reopened
}

#[test]
fn table_properties_cell_spacing_sets_file_points_in_one_undo_step_and_reopens() {
    let mut session = fixture(None);
    let before = table(&session).clone();
    session.run("table.properties", &json!({"cellSpacing": 0.75})).unwrap();
    assert_eq!(table(&session).props.cell_spacing, Some(0.75), "the public command must edit the table");
    assert_eq!(inherited(&session), Some(1.5));
    let applied = table(&session).clone();
    // Undo before calling the existing mutating-spec getter, so the test does
    // not depend on unrelated history behavior of a query command.
    session.run("edit.undo", &json!({})).unwrap();
    assert_eq!(table(&session), &before);
    session.run("edit.redo", &json!({})).unwrap();
    assert_eq!(table(&session), &applied);
    assert_getter(&mut session, Some(0.75));
    let mut session = save_reopen(session, Some(0.75), Some("15"));
    session.run("table.properties", &json!({"align": "center"})).unwrap();
    assert_eq!(table(&session).props.cell_spacing, Some(0.75), "an omitted spacing key preserves the direct value");
}

#[test]
fn table_properties_cell_spacing_zero_explicitly_overrides_inherited_spacing() {
    let mut session = fixture(None);
    session.run("table.properties", &json!({"cellSpacing": 0.0})).unwrap();
    assert_eq!(table(&session).props.cell_spacing, Some(0.0), "zero is an explicit direct override, not inheritance");
    assert_eq!(inherited(&session), Some(1.5));
    assert_getter(&mut session, Some(0.0));
    let session = save_reopen(session, Some(0.0), Some("0"));
    assert_eq!(table(&session).props.cell_spacing.or(inherited(&session)), Some(0.0));
}

#[test]
fn table_properties_cell_spacing_null_clears_override_and_restores_style_after_reopen() {
    let mut session = fixture(Some(0.0));
    session.run("table.properties", &json!({"cellSpacing": null})).unwrap();
    assert_eq!(table(&session).props.cell_spacing, None, "null removes the direct zero override");
    assert_eq!(inherited(&session), Some(1.5));
    assert_getter(&mut session, None);
    let session = save_reopen(session, None, None);
    assert_eq!(table(&session).props.cell_spacing.or(inherited(&session)), Some(1.5));
}

#[test]
fn table_properties_cell_spacing_bounds_and_invalid_values_follow_existing_length_contract() {
    for (input, expected) in [(-6.0, 0.0), (2000.0, 1584.0)] {
        let mut session = fixture(Some(0.75));
        session.run("table.properties", &json!({"cellSpacing": input})).unwrap();
        assert_eq!(table(&session).props.cell_spacing, Some(expected), "finite length is clamped to 0..1584 file points");
    }
    // NaN/Infinity are not JSON numbers. A finite f64 that overflows f32 and
    // malformed values must not accidentally behave like an explicit null.
    for bad in [json!(1e300), json!("NaN"), json!(false), json!([1, 2]), json!({})] {
        let mut session = fixture(Some(0.75));
        let before = table(&session).props.clone();
        let _ = session.run("table.properties", &json!({"cellSpacing": bad}));
        assert_eq!(table(&session).props, before, "invalid values may error or be ignored, matching existing optional length parsing");
    }
}
