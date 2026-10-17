//! Behavioral guards for ownership of the rich p/table/p bibliography cache.
//! Synthetic sources and public Session commands only; no Word runtime/fixture.

use std::io::{Cursor, Read};

use serde_json::{Value, json};
use wordcraft_doc::props::{Align, CharProps};
use wordcraft_doc::section::SectionProps;
use wordcraft_doc::{Document, StoryRef};
use wordcraft_engine::Session;

const HEADING: &str = "Edge bibliography.";
const AFTER: &str = "Unrelated text after edge bibliography.";
const TITLES: [&str; 3] = ["Synthetic Article for WordNeo Audit", "Synthetic Atlas for WordNeo Audit", "Uncited Source Must Survive"];
const TAGS: [&str; 3] = ["SyntheticBook2030", "SyntheticArticle2031", "SyntheticUncited2032"];

fn sources() -> [Value; 3] {
    let book = json!({
        "tag": TAGS[0], "kind": "book", "author": "Inventada, Lia; Exemplo, Rui",
        "title": TITLES[1], "year": "2030", "publisher": "Fictional Press", "city": "Imaginary City",
        "journal": "", "volume": "2", "pages": "11-29", "url": "https://example.invalid/synthetic-book",
        "editor": "Ficticio, Nilo", "edition": "3", "issue": "4", "month": "09", "day": "14",
        "accessed": "2030-10-16", "doi": "10.0000/wordneo.synthetic.book"
    });
    let mut article = book.clone();
    for (key, value) in [
        ("tag", TAGS[1]),
        ("kind", "article"),
        ("author", "Exemplo, Tais"),
        ("title", TITLES[0]),
        ("year", "2031"),
        ("publisher", "Synthetic Publisher"),
        ("journal", "Fictional Journal"),
        ("volume", "7"),
        ("issue", "2"),
        ("pages", "101-109"),
        ("url", "https://example.invalid/synthetic-article"),
        ("doi", "10.0000/wordneo.synthetic.article"),
    ] {
        article[key] = json!(value);
    }
    let mut uncited = book.clone();
    for (key, value) in
        [("tag", TAGS[2]), ("author", "Inventado, Caio"), ("year", "2032"), ("title", TITLES[2]), ("doi", "10.0000/wordneo.synthetic.uncited")]
    {
        uncited[key] = json!(value);
    }
    [book, article, uncited]
}

fn fixture(style: &str) -> Session {
    let mut s = Session::new(Document::new());
    s.run("text.insert", &json!({"text": "Synthetic evidence: "})).unwrap();
    for source in sources() {
        s.run("references.sources", &json!({"add": source})).unwrap();
    }
    s.run("references.citationStyle", &json!({"style": style})).unwrap();
    s.run("references.citation", &json!({"tag": TAGS[1], "pages": "17"})).unwrap();
    s.run("text.insert", &json!({"text": " and "})).unwrap();
    s.run("references.citation", &json!({"tag": TAGS[0], "pages": "5"})).unwrap();
    s.run("text.newParagraph", &json!({})).unwrap();
    s.run("references.bibliography", &json!({"title": HEADING})).unwrap();
    s.run("caret.docEnd", &json!({})).unwrap();
    s.run("text.insert", &json!({"text": AFTER})).unwrap();
    s
}

fn reopened(s: &Session) -> Session {
    Session::new(wordcraft_docx::read(&wordcraft_docx::write(&s.doc).unwrap()).unwrap())
}

fn bibliography_ranges(s: &Session) -> Vec<wordcraft_doc::fields::FieldRange> {
    s.doc.field_ranges(StoryRef::Body).into_iter().filter(|r| r.instr.split_whitespace().next() == Some("BIBLIOGRAPHY")).collect()
}

fn table_count(s: &Session) -> usize {
    s.doc.body.iter().filter(|block| block.as_table().is_some()).count()
}

fn assert_one_cache(s: &Session, populated: bool) {
    assert!(!s.doc.has_unbalanced_field_ranges());
    assert_eq!(bibliography_ranges(s).len(), 1);
    let text = s.doc.plain_text(StoryRef::Body);
    for marker in [HEADING, AFTER] {
        assert_eq!(text.matches(marker).count(), 1, "surrounding text stays outside replacement: {marker}");
    }
    for title in TITLES {
        assert_eq!(text.matches(title).count(), usize::from(populated), "cache must not accumulate or retain stale entries: {title}");
    }
}

fn main_xml(s: &Session) -> String {
    let bytes = wordcraft_docx::write(&s.doc).unwrap();
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut xml = String::new();
    archive.by_name("word/document.xml").unwrap().read_to_string(&mut xml).unwrap();
    xml
}

#[test]
fn ieee_apa_ieee_switches_replace_one_cache_without_accumulating_anchors() {
    let mut s = reopened(&fixture("IEEE"));
    assert_one_cache(&s, true);
    let (blocks, tables) = (s.doc.body.len(), table_count(&s));
    let original_sources = s.doc.sources.clone();
    for _ in 0..2 {
        s.run("references.citationStyle", &json!({"style": "APA"})).unwrap();
        s = reopened(&s);
        assert_eq!(table_count(&s), 0, "the IEEE table must not remain outside the APA field");
        assert_eq!(bibliography_ranges(&s).len(), 1);
        let text = s.doc.plain_text(StoryRef::Body);
        for marker in [HEADING, AFTER, TITLES[0], TITLES[1]] {
            assert_eq!(text.matches(marker).count(), 1);
        }
        s.run("references.citationStyle", &json!({"style": "IEEE"})).unwrap();
        s = reopened(&s);
        assert_one_cache(&s, true);
        assert_eq!((s.doc.body.len(), table_count(&s)), (blocks, tables));
        assert_eq!(s.doc.sources, original_sources);
    }
}

#[test]
fn ieee_empty_reopen_and_repopulate_keeps_one_cache_and_stable_anchors() {
    let mut s = reopened(&fixture("IEEE"));
    let populated_shape = (s.doc.body.len(), table_count(&s));
    let mut empty_blocks = None;
    for _ in 0..2 {
        for tag in TAGS {
            s.run("references.sources", &json!({"remove": tag})).unwrap();
        }
        s.run("references.updateFields", &json!({})).unwrap();
        s = reopened(&s);
        assert_one_cache(&s, false);
        assert_eq!(table_count(&s), 0);
        if let Some(expected) = empty_blocks {
            assert_eq!(s.doc.body.len(), expected);
        }
        empty_blocks = Some(s.doc.body.len());
        for source in sources() {
            s.run("references.sources", &json!({"add": source})).unwrap();
        }
        s.run("references.updateFields", &json!({})).unwrap();
        s = reopened(&s);
        assert_one_cache(&s, true);
        assert_eq!((s.doc.body.len(), table_count(&s)), populated_shape);
    }
}

#[test]
fn bibliography_refresh_preserves_external_suffix_properties_and_empty_suffix_sections() {
    const SUFFIX: &str = "External suffix keeps its own formatting.";
    for style in ["IEEE", "APA"] {
        for with_text in [false, true] {
            let mut s = reopened(&fixture("IEEE"));
            let end = bibliography_ranges(&s).pop().unwrap().end;
            let para = s.doc.para_mut(StoryRef::Body, &end.path).unwrap();
            // Synthetic user formatting outside the field, not a Word layout measure.
            para.section = Some(Box::new(SectionProps { margin_bottom: 83.0, ..Default::default() }));
            if with_text {
                para.insert_text(end.off + wordcraft_doc::para::OBJ.len_utf8(), SUFFIX, &CharProps { italic: Some(true), ..Default::default() })
                    .unwrap();
                para.props.style = Some("Quote".into());
                para.props.align = Some(Align::Right);
                para.props.indent_left = Some(17.0);
            }
            let mut s = reopened(&s);
            let sections: Vec<_> = s.doc.body.iter().filter_map(|b| b.as_para()).filter_map(|p| p.section.clone()).collect();
            let suffix_props =
                with_text.then(|| s.doc.body.iter().filter_map(|b| b.as_para()).find(|p| p.text.contains(SUFFIX)).unwrap().props.clone());
            s.run("references.citationStyle", &json!({"style": style})).unwrap();
            for _ in 0..2 {
                s.run("references.updateFields", &json!({})).unwrap();
                s = reopened(&s);
                assert!(!s.doc.has_unbalanced_field_ranges());
                let got: Vec<_> = s.doc.body.iter().filter_map(|b| b.as_para()).filter_map(|p| p.section.clone()).collect();
                assert_eq!(got, sections, "{style}: a section on an empty or nonempty external suffix must survive");
                if let Some(expected) = &suffix_props {
                    let text = s.doc.plain_text(StoryRef::Body);
                    assert_eq!(text.matches(SUFFIX).count(), 1);
                    let para = s.doc.body.iter().filter_map(|b| b.as_para()).find(|p| p.text.contains(SUFFIX)).unwrap();
                    assert_eq!(&para.props, expected, "{style}: an external suffix keeps its paragraph formatting");
                    assert_eq!(para.props_of_char(para.text.find(SUFFIX).unwrap()).italic, Some(true));
                }
            }
        }
    }
}

#[test]
fn bibliography_growth_keeps_the_caret_after_unrelated_following_text() {
    let mut s = reopened(&fixture("APA"));
    s.run("caret.docEnd", &json!({})).unwrap();
    s.run("references.citationStyle", &json!({"style": "IEEE"})).unwrap();
    const TYPED: &str = " Typing stays after this text.";
    s.run("text.insert", &json!({"text": TYPED})).unwrap();
    let text = s.doc.plain_text(StoryRef::Body);
    assert!(text.contains(&format!("{AFTER}{TYPED}")), "replacing earlier bibliography blocks must retain the caret at the existing suffix: {text}");
    s.run("references.updateFields", &json!({})).unwrap();
    assert!(
        s.doc.plain_text(StoryRef::Body).contains(&format!("{AFTER}{TYPED}")),
        "typing outside the field must not be consumed by its next refresh"
    );
}

#[test]
fn locked_ieee_cache_survives_updates_and_code_view_then_unlocks() {
    let mut s = reopened(&fixture("IEEE"));
    s.run("select.all", &json!({})).unwrap();
    s.run("fields.lock", &json!({})).unwrap();
    assert!(bibliography_ranges(&s)[0].locked);
    let before = main_xml(&s);
    let mut updated = sources()[0].clone();
    updated["title"] = json!("Updated book in bibliography edge controls");
    s.run("references.sources", &json!({"add": updated})).unwrap();
    s.run("view.fieldCodes", &json!({"value": true})).unwrap();
    s.run("references.updateFields", &json!({})).unwrap();
    assert_eq!(main_xml(&s), before, "a locked result remains unchanged; code view is not document content");
    s = reopened(&s);
    assert!(bibliography_ranges(&s)[0].locked);
    s.run("select.all", &json!({})).unwrap();
    s.run("fields.unlock", &json!({})).unwrap();
    s.run("references.updateFields", &json!({})).unwrap();
    s = reopened(&s);
    assert!(!bibliography_ranges(&s)[0].locked);
    let text = s.doc.plain_text(StoryRef::Body);
    assert_eq!(text.matches("Updated book in bibliography edge controls").count(), 1);
    assert!(!text.contains(TITLES[1]));
}

#[test]
fn two_bibliographies_refresh_separate_caches_without_duplicating_surroundings() {
    let mut s = fixture("IEEE");
    const SECOND: &str = "Second edge bibliography.";
    const LAST: &str = "Text after the second bibliography.";
    s.run("caret.docEnd", &json!({})).unwrap();
    s.run("text.newParagraph", &json!({})).unwrap();
    s.run("references.bibliography", &json!({"title": SECOND})).unwrap();
    s.run("caret.docEnd", &json!({})).unwrap();
    s.run("text.insert", &json!({"text": LAST})).unwrap();
    let mut s = reopened(&s);
    let shape = (s.doc.body.len(), table_count(&s));
    for _ in 0..2 {
        s.run("references.updateFields", &json!({})).unwrap();
        s = reopened(&s);
        let ranges = bibliography_ranges(&s);
        assert_eq!(ranges.len(), 2);
        assert!(ranges[0].end < ranges[1].start);
        assert!(!s.doc.has_unbalanced_field_ranges());
        assert_eq!((s.doc.body.len(), table_count(&s)), shape);
        let text = s.doc.plain_text(StoryRef::Body);
        for marker in [HEADING, AFTER, SECOND, LAST] {
            assert_eq!(text.matches(marker).count(), 1);
        }
        for title in TITLES {
            assert_eq!(text.matches(title).count(), 2, "each independently owned result has one entry");
        }
    }
}
