//! Public synthetic I/O contract for paragraph w:textAlignment.
//! No new model field is named: this RED compiles before model support exists.

use std::collections::BTreeMap;
use std::io::{Cursor, Read, Write};
use std::sync::Arc;

use quick_xml::events::Event;
use quick_xml::reader::Reader;
use wordcraft_doc::props::{Align, LineSpacing, ParaProps};
use wordcraft_doc::{Block, Document};

const MAIN: &str = "word/document.xml";
const STYLES: &str = "word/styles.xml";

fn unzip(bytes: &[u8]) -> BTreeMap<String, Vec<u8>> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut parts = BTreeMap::new();
    for index in 0..archive.len() {
        let mut part = archive.by_index(index).unwrap();
        let mut payload = Vec::new();
        part.read_to_end(&mut payload).unwrap();
        assert!(parts.insert(part.name().into(), payload).is_none());
    }
    parts
}

fn zip(parts: &BTreeMap<String, Vec<u8>>) -> Vec<u8> {
    let mut archive = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, payload) in parts {
        archive.start_file(name, zip::write::SimpleFileOptions::default()).unwrap();
        archive.write_all(payload).unwrap();
    }
    archive.finish().unwrap().into_inner()
}

fn alignments(xml: &[u8]) -> Vec<String> {
    let mut reader = Reader::from_reader(xml);
    let mut values = Vec::new();
    let mut path: Vec<Vec<u8>> = Vec::new();
    let mut buffer = Vec::new();
    loop {
        buffer.clear();
        let event = reader.read_event_into(&mut buffer).unwrap();
        if let Event::Start(element) | Event::Empty(element) = &event
            && element.name().as_ref() == b"w:textAlignment"
        {
            assert_eq!(path.last().map(Vec::as_slice), Some(b"w:pPr".as_slice()));
            let value = element.attributes().map(Result::unwrap).find(|a| a.key.as_ref() == b"w:val").unwrap();
            values.push(String::from_utf8(value.value.into_owned()).unwrap());
        }
        match event {
            Event::Start(element) => path.push(element.name().as_ref().to_vec()),
            Event::End(element) => assert_eq!(path.pop().as_deref(), Some(element.name().as_ref())),
            Event::Eof => return values,
            _ => {}
        }
    }
}

fn fixture(token: Option<&str>, style: Option<&str>, extra_styles: &str) -> Vec<u8> {
    let original = unzip(&wordcraft_docx::write(&Document::from_text("Synthetic base.")).unwrap());
    let mut parts = original.clone();
    let alignment = token.map(|t| format!(r#"<w:textAlignment w:val="{t}"/>"#)).unwrap_or_default();
    let style = style.map(|s| format!(r#"<w:pStyle w:val="{s}"/>"#)).unwrap_or_default();
    let main = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:pPr>{style}<w:keepNext w:val="0"/><w:snapToGrid w:val="0"/><w:spacing w:after="120" w:line="480" w:lineRule="exact"/><w:jc w:val="right"/>{alignment}<w:outlineLvl w:val="2"/><w:rPr><w:b/></w:rPr></w:pPr><w:r><w:rPr><w:i/></w:rPr><w:t>Synthetic exact alignment.</w:t></w:r></w:p><w:p><w:r><w:t>Trailing synthetic sentinel.</w:t></w:r></w:p></w:body></w:document>"#
    );
    parts.insert(MAIN.into(), main.into_bytes());
    if !extra_styles.is_empty() {
        let styles = String::from_utf8(parts[STYLES].clone()).unwrap();
        assert_eq!(styles.matches("</w:styles>").count(), 1);
        parts.insert(STYLES.into(), styles.replacen("</w:styles>", &format!("{extra_styles}</w:styles>"), 1).into_bytes());
    }
    let actual = unzip(&zip(&parts));
    assert!(actual.keys().eq(original.keys()), "no package parts added");
    for (name, payload) in original {
        if name != MAIN && !(name == STYLES && !extra_styles.is_empty()) {
            assert_eq!(actual[&name], payload, "unrelated part {name} unchanged");
        }
    }
    zip(&parts)
}

fn assert_body_preserved(actual: &Document, control: &Document) {
    assert_eq!(actual.body.len(), control.body.len());
    for (actual, control) in actual.body.iter().zip(&control.body) {
        let mut actual = actual.as_para().unwrap().clone();
        let control = control.as_para().unwrap();
        assert_eq!(actual.props.line_spacing, actual.text.starts_with("Synthetic").then_some(LineSpacing::Exactly(24.0)));
        if actual.text.starts_with("Synthetic") {
            assert_eq!(actual.props.space_after, Some(6.0));
            assert_eq!(actual.props.keep_next, Some(false));
            assert_eq!(actual.props.snap_to_grid, Some(false));
            assert_eq!(actual.props.align, Some(Align::Right));
            assert_eq!(actual.props.outline_level, Some(2));
        }
        actual.props = control.props.clone();
        assert_eq!(&actual, control, "text, runs, mark, objects and section stay unchanged");
    }
    assert_eq!(actual.settings, control.settings);
}

fn round_trip(token: Option<&str>, expected: Option<&str>) {
    let control = wordcraft_docx::read(&fixture(None, None, "")).unwrap();
    let mut bytes = fixture(token, None, "");
    let expected: Vec<String> = expected.map(str::to_string).into_iter().collect();
    for round in 1..=2 {
        let document = wordcraft_docx::read(&bytes).unwrap();
        assert_body_preserved(&document, &control);
        bytes = wordcraft_docx::write(&document).unwrap();
        let parts = unzip(&bytes);
        assert_eq!(alignments(&parts[MAIN]), expected, "textAlignment survives round {round}: {token:?}");
        if let Some(token) = token.filter(|_| !expected.is_empty()) {
            let xml = String::from_utf8(parts[MAIN].clone()).unwrap();
            let alignment = xml.find(&format!(r#"<w:textAlignment w:val="{token}""#)).unwrap();
            assert!(xml.find("<w:jc").unwrap() < alignment);
            assert!(alignment < xml.find("<w:outlineLvl").unwrap());
            assert!(alignment < xml.find("<w:rPr").unwrap(), "alignment remains before the paragraph mark properties");
        }
    }
}

#[test]
fn text_alignment_auto_survives_two_round_trips() {
    round_trip(Some("auto"), Some("auto"));
}

#[test]
fn text_alignment_baseline_survives_two_round_trips() {
    round_trip(Some("baseline"), Some("baseline"));
}

#[test]
fn text_alignment_top_survives_two_round_trips() {
    round_trip(Some("top"), Some("top"));
}

#[test]
fn text_alignment_center_survives_two_round_trips() {
    round_trip(Some("center"), Some("center"));
}

#[test]
fn text_alignment_bottom_survives_two_round_trips() {
    round_trip(Some("bottom"), Some("bottom"));
}

#[test]
fn absent_text_alignment_stays_omitted() {
    round_trip(None, None);
}

#[test]
fn unknown_or_missing_ooxml_alignment_is_ignored() {
    round_trip(Some("unexpected-alignment"), None);
    let mut parts = unzip(&fixture(Some("auto"), None, ""));
    let main = String::from_utf8(parts[MAIN].clone()).unwrap();
    parts.insert(MAIN.into(), main.replace(r#"<w:textAlignment w:val="auto"/>"#, "<w:textAlignment/>").into_bytes());
    let document = wordcraft_docx::read(&zip(&parts)).unwrap();
    assert!(alignments(&unzip(&wordcraft_docx::write(&document).unwrap())[MAIN]).is_empty());
}

fn exported_props(props: ParaProps) -> Vec<String> {
    let mut document = Document::from_text("Effective synthetic style.");
    let Block::Para(paragraph) = Arc::make_mut(&mut document.body[0]) else { panic!("paragraph fixture") };
    paragraph.props = props;
    alignments(&unzip(&wordcraft_docx::write(&document).unwrap())[MAIN])
}

#[test]
fn paragraph_styles_inherit_alignment_and_direct_auto_overrides_without_flattening() {
    let styles = r#"<w:style w:type="paragraph" w:styleId="ExactBase"><w:name w:val="ExactBase"/><w:pPr><w:textAlignment w:val="center"/></w:pPr></w:style><w:style w:type="paragraph" w:styleId="ExactChild"><w:name w:val="ExactChild"/><w:basedOn w:val="ExactBase"/></w:style>"#;
    let mut bytes = fixture(None, Some("ExactChild"), styles);
    let direct = wordcraft_docx::read(&fixture(Some("auto"), None, "")).unwrap().body[0].as_para().unwrap().props.clone();
    for _ in 0..2 {
        let document = wordcraft_docx::read(&bytes).unwrap();
        assert!(exported_props(document.styles.get("ExactChild").unwrap().para.clone()).is_empty(), "do not flatten inheritance");
        let (mut effective, _) = document.styles.para_style_props("ExactChild");
        assert_eq!(exported_props(effective.clone()), ["center"]);
        effective.overlay(&ParaProps::default());
        assert_eq!(exported_props(effective.clone()), ["center"], "absent direct value keeps inherited alignment");
        effective.overlay(&direct);
        assert_eq!(exported_props(effective), ["auto"], "explicit auto cancels inherited center");
        bytes = wordcraft_docx::write(&document).unwrap();
        let parts = unzip(&bytes);
        assert!(alignments(&parts[MAIN]).is_empty(), "paragraph-local absence survives");
        assert!(alignments(&parts[STYLES]).iter().any(|v| v == "center"));
    }
}
