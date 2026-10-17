//! Structural field-scope regression isolated from the existing synthetic Word observation.
//! No Word output or binary fixture is embedded here.

use std::io::{Cursor, Read};

use quick_xml::XmlVersion;
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use wordcraft_doc::{CharProps, Document, InlineObject, Paragraph, Style, para_block};

const HEADING: &str = "Synthetic references heading";
const FIRST: &str = "[1] First synthetic bibliography entry.";
const SECOND: &str = "[2] Second synthetic bibliography entry.";
const AFTER: &str = "Unrelated text after the bibliography.";

fn generated_bibliography() -> Document {
    let mut doc = Document::new();
    doc.styles.upsert(Style {
        id: "ListEntry".into(),
        name: "List Entry".into(),
        kind: wordcraft_doc::StyleKind::Paragraph,
        based_on: Some("Normal".into()),
        ..Default::default()
    });
    let mut heading = Paragraph::with_text(HEADING, CharProps::default()).styled("Heading1");
    heading
        .insert_object(
            heading.len(),
            InlineObject::Field { instr: "BIBLIOGRAPHY".into(), result: String::new(), locked: false, code: false },
            &CharProps::default(),
        )
        .unwrap();
    doc.body = vec![
        para_block(heading),
        para_block(Paragraph::with_text(FIRST, CharProps::default()).styled("ListEntry")),
        para_block(Paragraph::with_text(SECOND, CharProps::default()).styled("ListEntry")),
        para_block(Paragraph::with_text(AFTER, CharProps::default())),
    ];
    doc
}

/// (paragraph number, semantic token kind, token value), in XML document order.
fn tokens(xml: &[u8]) -> Vec<(usize, &'static str, String)> {
    let mut reader = Reader::from_reader(xml);
    let mut buffer = Vec::new();
    let mut paragraph = 0;
    let mut text_kind = None;
    let mut result = Vec::new();
    loop {
        buffer.clear();
        match reader.read_event_into(&mut buffer).unwrap() {
            Event::Start(element) | Event::Empty(element) => match element.local_name().as_ref() {
                b"p" => paragraph += 1,
                b"t" => text_kind = Some("text"),
                b"instrText" => text_kind = Some("instruction"),
                b"fldChar" => {
                    let kind = element
                        .attributes()
                        .map(Result::unwrap)
                        .find(|attribute| attribute.key.as_ref() == b"w:fldCharType")
                        .unwrap()
                        .normalized_value(XmlVersion::Explicit1_0)
                        .unwrap()
                        .into_owned();
                    result.push((paragraph, "field", kind));
                }
                _ => {}
            },
            Event::Text(text) => {
                if let Some(kind) = text_kind {
                    result.push((paragraph, kind, text.decode().unwrap().into_owned()));
                }
            }
            Event::End(element) if matches!(element.local_name().as_ref(), b"t" | b"instrText") => text_kind = None,
            Event::Eof => return result,
            _ => {}
        }
    }
}

#[test]
fn native_bibliography_entries_are_the_cached_result_of_one_field() {
    let bytes = wordcraft_docx::write(&generated_bibliography()).unwrap();
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut xml = Vec::new();
    archive.by_name("word/document.xml").unwrap().read_to_end(&mut xml).unwrap();
    let events = tokens(&xml);
    let position = |kind: &str, value: &str| {
        let matches: Vec<_> = events.iter().enumerate().filter(|(_, (_, k, text))| *k == kind && text.trim() == value).collect();
        assert_eq!(matches.len(), 1, "one {kind} token for {value}: {events:?}");
        matches[0].0
    };
    let heading = position("text", HEADING);
    let begin = position("field", "begin");
    let instruction = position("instruction", "BIBLIOGRAPHY");
    let separate = position("field", "separate");
    let first = position("text", FIRST);
    let second = position("text", SECOND);
    let end = position("field", "end");
    let after = position("text", AFTER);
    assert_ne!(events[first].0, events[second].0, "the cached result retains two real paragraphs");
    assert!(heading < begin && begin < instruction && instruction < separate, "preserve the title outside the field: {events:?}");
    assert!(
        separate < first && first < second && second < end && end < after,
        "both entry paragraphs must belong to the single BIBLIOGRAPHY result, with following text outside: {events:?}"
    );
}

#[test]
fn bibliography_cached_paragraphs_keep_their_range_and_formatting_through_reexport() {
    let mut original = generated_bibliography();
    let first = original.para_mut(wordcraft_doc::StoryRef::Body, &wordcraft_doc::Path::top(1)).unwrap();
    first.format(0, first.len(), &|props| props.italic = Some(true)).unwrap();
    first.props.indent_left = Some(36.0);
    first.props.indent_first = Some(-36.0);
    let text = original.plain_text(wordcraft_doc::StoryRef::Body);
    let mut doc = wordcraft_docx::read(&wordcraft_docx::write(&original).unwrap()).unwrap();
    for _ in 0..3 {
        let ranges = doc.field_ranges(wordcraft_doc::StoryRef::Body);
        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges[0].instr, "BIBLIOGRAPHY");
        assert_eq!(ranges[0].start.path, wordcraft_doc::Path::top(1));
        assert_eq!(ranges[0].end.path, wordcraft_doc::Path::top(2));
        assert_eq!(doc.plain_text(wordcraft_doc::StoryRef::Body), text);
        assert_eq!(doc.body.len(), original.body.len(), "do not invent empty boundary paragraphs");
        let first = doc.para(wordcraft_doc::StoryRef::Body, &wordcraft_doc::Path::top(1)).unwrap();
        assert_eq!(first.props.indent_left, Some(36.0));
        assert_eq!(first.props.indent_first, Some(-36.0));
        assert_eq!(first.props_of_char(first.text.find(FIRST).unwrap()).italic, Some(true));
        doc = wordcraft_docx::read(&wordcraft_docx::write(&doc).unwrap()).unwrap();
    }
    assert!(original.field_ranges(wordcraft_doc::StoryRef::Body).is_empty(), "writing must not mutate the native document");
}

#[test]
fn bibliography_field_code_view_does_not_change_exported_result_scope() {
    for show_code in [false, true] {
        let mut doc = generated_bibliography();
        let heading = doc.para_mut(wordcraft_doc::StoryRef::Body, &wordcraft_doc::Path::top(0)).unwrap();
        let InlineObject::Field { code, .. } = &mut heading.objects[0] else { panic!("native field fixture") };
        *code = show_code;
        let reopened = wordcraft_docx::read(&wordcraft_docx::write(&doc).unwrap()).unwrap();
        let ranges = reopened.field_ranges(wordcraft_doc::StoryRef::Body);
        assert_eq!(ranges.len(), 1, "view-only field code state {show_code} must not leave cached entries outside the field");
        assert_eq!(ranges[0].start.path, wordcraft_doc::Path::top(1));
        assert_eq!(ranges[0].end.path, wordcraft_doc::Path::top(2));
        assert_eq!(reopened.plain_text(wordcraft_doc::StoryRef::Body), doc.plain_text(wordcraft_doc::StoryRef::Body));
        let heading = doc.para(wordcraft_doc::StoryRef::Body, &wordcraft_doc::Path::top(0)).unwrap();
        assert!(matches!(heading.objects[0], InlineObject::Field { code, .. } if code == show_code), "export must not mutate the view state");
    }
}
