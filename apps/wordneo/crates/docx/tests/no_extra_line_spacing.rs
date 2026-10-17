//! A synthetic DOCX I/O contract for w:compat/w:noExtraLineSpacing.
//! These tests use existing read/write APIs, without requiring a new Settings field.

use std::collections::BTreeMap;
use std::io::{Cursor, Read, Write};

use quick_xml::XmlVersion;
use quick_xml::events::{BytesStart, Event};
use quick_xml::reader::Reader;
use wordcraft_doc::Document;

const SETTINGS: &str = "word/settings.xml";

fn unzip(bytes: &[u8]) -> BTreeMap<String, Vec<u8>> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut parts = BTreeMap::new();
    for index in 0..archive.len() {
        let mut part = archive.by_index(index).unwrap();
        let name = part.name().to_string();
        let mut payload = Vec::new();
        part.read_to_end(&mut payload).unwrap();
        assert!(parts.insert(name, payload).is_none(), "no duplicate ZIP parts");
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

fn on_off(element: &BytesStart<'_>) -> bool {
    let value = element.attributes().find_map(|attribute| {
        let attribute = attribute.unwrap();
        (attribute.key.as_ref() == b"w:val").then(|| attribute.normalized_value(XmlVersion::Explicit1_0).unwrap().into_owned())
    });
    match value.as_deref() {
        None | Some("1" | "true" | "on") => true,
        Some("0" | "false" | "off") => false,
        value => panic!("unexpected ST_OnOff spelling: {value:?}"),
    }
}

fn no_extra_line_spacing(xml: &[u8]) -> Vec<bool> {
    let mut reader = Reader::from_reader(xml);
    let mut buffer = Vec::new();
    let mut path: Vec<Vec<u8>> = Vec::new();
    let mut values = Vec::new();
    loop {
        buffer.clear();
        let event = reader.read_event_into(&mut buffer).unwrap();
        if let Event::Start(element) | Event::Empty(element) = &event
            && element.name().as_ref() == b"w:noExtraLineSpacing"
        {
            assert_eq!(path, [b"w:settings".to_vec(), b"w:compat".to_vec()], "the flag is a direct compatibility child");
            values.push(on_off(element));
        }
        match event {
            Event::Start(element) => path.push(element.name().as_ref().to_vec()),
            Event::End(element) => assert_eq!(path.pop().as_deref(), Some(element.name().as_ref())),
            Event::Eof => {
                assert!(path.is_empty());
                return values;
            }
            _ => {}
        }
    }
}

fn fixture(element: Option<&str>) -> Vec<u8> {
    let mut document = Document::from_text("Synthetic compatibility fixture.\nTrailing synthetic sentinel.");
    document.settings.compat_mode = 14;
    document.settings.track_changes = true;
    document.settings.default_tab = 42.0;
    let original = unzip(&wordcraft_docx::write(&document).unwrap());
    let original_xml = String::from_utf8(original[SETTINGS].clone()).unwrap();
    assert!(no_extra_line_spacing(original_xml.as_bytes()).is_empty());
    assert_eq!(original_xml.matches("<w:compat>").count(), 1);

    let modified_xml = match element {
        Some(element) => original_xml.replacen("<w:compat>", &format!("<w:compat>{element}"), 1),
        None => original_xml.clone(),
    };
    let mut parts = original.clone();
    parts.insert(SETTINGS.into(), modified_xml.into_bytes());
    let bytes = zip(&parts);
    let actual = unzip(&bytes);
    assert!(actual.keys().eq(original.keys()), "the fixture adds no package parts");
    let changed: Vec<_> = original.iter().filter(|(name, payload)| actual.get(*name) != Some(*payload)).map(|(name, _)| name.as_str()).collect();
    let expected_changed = if element.is_some() { vec![SETTINGS] } else { vec![] };
    assert_eq!(changed, expected_changed, "every other uncompressed package payload stays byte-identical");
    let actual_xml = String::from_utf8(actual[SETTINGS].clone()).unwrap();
    match element {
        Some(element) => {
            assert_eq!(actual_xml.matches(element).count(), 1);
            assert_eq!(actual_xml.replacen(element, "", 1), original_xml, "the sole intervention is the compatibility element");
        }
        None => assert_eq!(actual_xml, original_xml),
    }
    bytes
}

fn assert_two_round_trips(element: Option<&str>, expected: Option<bool>) {
    let control = wordcraft_docx::read(&fixture(None)).unwrap();
    let mut bytes = fixture(element);
    let expected_values: Vec<_> = expected.into_iter().collect();
    assert_eq!(no_extra_line_spacing(&unzip(&bytes)[SETTINGS]), expected_values, "the synthetic input has the requested state");
    for round in 1..=2 {
        let document = wordcraft_docx::read(&bytes).unwrap();
        assert_eq!(document.body, control.body, "the synthetic body survives round {round}");
        assert_eq!(document.settings.compat_mode, 14, "compatibility mode is independent of this flag");
        assert!(document.settings.track_changes);
        assert_eq!(document.settings.default_tab, 42.0);
        bytes = wordcraft_docx::write(&document).unwrap();
        assert_eq!(
            no_extra_line_spacing(&unzip(&bytes)[SETTINGS]),
            expected_values,
            "noExtraLineSpacing state {expected:?} must survive read/write round {round}; input {element:?}"
        );
    }
}

#[test]
fn no_extra_line_spacing_true_and_implicit_true_survive_docx_round_trips() {
    for element in [
        "<w:noExtraLineSpacing/>",
        r#"<w:noExtraLineSpacing w:val="1"/>"#,
        r#"<w:noExtraLineSpacing w:val="true"/>"#,
        r#"<w:noExtraLineSpacing w:val="on"></w:noExtraLineSpacing>"#,
    ] {
        assert_two_round_trips(Some(element), Some(true));
    }
}

#[test]
fn no_extra_line_spacing_explicit_false_survives_docx_round_trips() {
    for element in [
        r#"<w:noExtraLineSpacing w:val="0"/>"#,
        r#"<w:noExtraLineSpacing w:val="false"/>"#,
        r#"<w:noExtraLineSpacing w:val="off"></w:noExtraLineSpacing>"#,
    ] {
        assert_two_round_trips(Some(element), Some(false));
    }
}

#[test]
fn absent_no_extra_line_spacing_stays_absent_after_docx_round_trips() {
    assert_two_round_trips(None, None);
}

#[test]
fn new_document_does_not_invent_no_extra_line_spacing() {
    let mut bytes = wordcraft_docx::write(&Document::new()).unwrap();
    for _ in 0..2 {
        assert!(no_extra_line_spacing(&unzip(&bytes)[SETTINGS]).is_empty());
        let document = wordcraft_docx::read(&bytes).unwrap();
        assert_eq!(document.settings.compat_mode, wordcraft_doc::COMPAT_MODE_CURRENT);
        bytes = wordcraft_docx::write(&document).unwrap();
    }
    assert!(no_extra_line_spacing(&unzip(&bytes)[SETTINGS]).is_empty());
}
