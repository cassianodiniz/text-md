//! Structural IEEE contract from the measured synthetic Word008 baseline.
//! No Word-produced fixture is embedded. Grid hints and unmeasured run formatting
//! are deliberately excluded; percentage/auto preferences govern the table.

use std::io::{Cursor, Read};

use regex::Regex;
use serde_json::{Value, json};
use wordcraft_doc::props::Borders;
use wordcraft_doc::{Document, StoryRef};
use wordcraft_engine::Session;

const HEADING: &str = "Synthetic References";
const AFTER: &str = "Unrelated text after the synthetic bibliography.";
const TITLES: [&str; 3] = ["Synthetic Article for WordNeo Audit", "Synthetic Atlas for WordNeo Audit", "Uncited Source Must Survive"];

fn synthetic_sources() -> [Value; 3] {
    let book = json!({
        "tag": "SyntheticBook2030", "kind": "book", "author": "Inventada, Lia; Exemplo, Rui",
        "title": TITLES[1], "year": "2030", "publisher": "Fictional Press", "city": "Imaginary City",
        "journal": "", "volume": "2", "pages": "11-29", "url": "https://example.invalid/synthetic-book",
        "editor": "Ficticio, Nilo", "edition": "3", "issue": "4", "month": "09", "day": "14",
        "accessed": "2030-10-16", "doi": "10.0000/wordneo.synthetic.book"
    });
    let mut article = book.clone();
    for (key, value) in [
        ("tag", "SyntheticArticle2031"),
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
    for (key, value) in [
        ("tag", "SyntheticUncited2032"),
        ("author", "Inventado, Caio"),
        ("year", "2032"),
        ("title", TITLES[2]),
        ("doi", "10.0000/wordneo.synthetic.uncited"),
    ] {
        uncited[key] = json!(value);
    }
    [book, article, uncited]
}

fn native_bibliography() -> Session {
    let mut session = Session::new(Document::new());
    session.run("text.insert", &json!({"text": "Synthetic evidence: "})).unwrap();
    for source in synthetic_sources() {
        session.run("references.sources", &json!({"add": source})).unwrap();
    }
    session.run("references.citationStyle", &json!({"style": "IEEE"})).unwrap();
    session.run("references.citation", &json!({"tag": "SyntheticArticle2031", "pages": "17"})).unwrap();
    session.run("text.insert", &json!({"text": " and "})).unwrap();
    session.run("references.citation", &json!({"tag": "SyntheticBook2030", "pages": "5"})).unwrap();
    session.run("text.newParagraph", &json!({})).unwrap();
    session.run("references.bibliography", &json!({"title": HEADING})).unwrap();
    session.run("caret.docEnd", &json!({})).unwrap();
    session.run("text.insert", &json!({"text": AFTER})).unwrap();
    session
}

fn assert_table(doc: &Document) {
    let tables: Vec<_> = doc.body.iter().filter_map(|block| block.as_table()).collect();
    assert_eq!(tables.len(), 1, "Word008's native IEEE result is one real table, not three plain entry paragraphs");
    let table = tables[0];
    assert_eq!(table.rows.len(), 3);
    assert!(table.rows.iter().all(|row| row.cells.len() == 2), "one number column and one entry column");
    assert_eq!(table.props.width_pct, Some(100.0), "Word tblW=5000 pct");
    assert!(!table.props.fixed, "AutoFit stays enabled; measured grid values are hints, not fixed column widths");
    assert_eq!(table.props.cell_margins, Some([0.75; 4]), "Word tblCellMar=15 dxa on every side");

    let mut borders =
        table.props.style.as_deref().and_then(|id| doc.styles.table_style(id)).and_then(|style| style.parts.borders).unwrap_or_else(Borders::default);
    if let Some(own) = table.props.borders {
        borders.overlay(&own);
    }
    assert!(!borders.any_visible(), "the bibliography has no visible table grid");
    for (index, row) in table.rows.iter().enumerate() {
        let number = &row.cells[0];
        let entry = &row.cells[1];
        assert_eq!(number.props.width_pct, Some(1.0), "Word first-cell tcW=50 pct, not fifty percent");
        assert_eq!(entry.props.width_pct, None);
        assert_eq!(entry.props.width, None, "Word second-cell tcW=0 auto");
        for cell in &row.cells {
            assert!(!cell.props.borders.is_some_and(|b| b.any_visible()));
            assert_eq!(cell.props.margins.or(table.props.cell_margins), Some([0.75; 4]));
        }
        let cell_text = |cell: &wordcraft_doc::table::Cell| {
            cell.blocks.iter().filter_map(|block| block.as_para()).map(|p| p.plain_text()).collect::<Vec<_>>().join("\n")
        };
        let label = format!("[{}]", index + 1);
        assert_eq!(cell_text(number).trim(), label, "the numeral is its own cell");
        let text = cell_text(entry);
        assert_eq!(text.matches(TITLES[index]).count(), 1, "entry keeps its source title in the measured citation order");
        assert!(!text.contains(&label), "the entry cell must not repeat its numeral");
    }
}

fn assert_field_envelope(doc: &Document) {
    assert_table(doc);
    assert_eq!(doc.sources.len(), 3);
    assert!(!doc.has_unbalanced_field_ranges(), "bibliography markers must not become orphans across cells");
    let ranges = doc.field_ranges(StoryRef::Body);
    let bibliography: Vec<_> = ranges.iter().filter(|range| range.instr.split_whitespace().next() == Some("BIBLIOGRAPHY")).collect();
    assert_eq!(bibliography.len(), 1);
    let range = bibliography[0];
    assert_eq!((range.start.path.depth(), range.end.path.depth()), (0, 0), "Word's field markers are outside the cells");
    let table_index = doc.body.iter().position(|block| block.as_table().is_some()).unwrap();
    assert!(
        range.start.path.last() < table_index && table_index < range.end.path.last(),
        "the complete table belongs to the one cached field result"
    );

    let text = doc.plain_text(StoryRef::Body);
    for value in [HEADING, AFTER].into_iter().chain(TITLES) {
        assert_eq!(text.matches(value).count(), 1, "refresh/reopen must not duplicate titles or surrounding content: {value}");
    }
    let (heading_index, heading) =
        doc.body.iter().enumerate().filter_map(|(i, b)| b.as_para().map(|p| (i, p))).find(|(_, p)| p.text.contains(HEADING)).unwrap();
    let heading_end = heading.text.find(HEADING).unwrap() + HEADING.len();
    assert_eq!(heading.props.style.as_deref(), Some("Heading1"));
    assert!(
        heading_index < range.start.path.last() || (heading_index == range.start.path.last() && heading_end <= range.start.off),
        "the heading is outside the result"
    );
    let (after_index, after) =
        doc.body.iter().enumerate().filter_map(|(i, b)| b.as_para().map(|p| (i, p))).find(|(_, p)| p.text.contains(AFTER)).unwrap();
    let after_offset = after.text.find(AFTER).unwrap();
    assert!(
        after_index > range.end.path.last()
            || (after_index == range.end.path.last() && after_offset >= range.end.off + wordcraft_doc::para::OBJ.len_utf8()),
        "unrelated following text is outside the result"
    );
}

fn document_xml(bytes: &[u8]) -> String {
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut xml = String::new();
    zip.by_name("word/document.xml").unwrap().read_to_string(&mut xml).unwrap();
    xml
}

fn opening_tag<'a>(xml: &'a str, name: &str) -> &'a str {
    Regex::new(&format!(r"<w:{}(?:\s[^>]*)?/?>", regex::escape(name)))
        .unwrap()
        .find(xml)
        .unwrap_or_else(|| panic!("missing measured OOXML property w:{name}"))
        .as_str()
}

fn attribute<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    tag.split_once(&format!("w:{name}=\"")).and_then(|(_, value)| value.split('"').next())
}

fn assert_exported_properties(bytes: &[u8]) {
    let xml = document_xml(bytes);
    let start = xml.find(opening_tag(&xml, "tbl")).unwrap();
    let tail = &xml[start..];
    let table = &tail[..tail.find("</w:tbl>").unwrap() + "</w:tbl>".len()];
    let width = opening_tag(table, "tblW");
    assert_eq!((attribute(width, "w"), attribute(width, "type")), (Some("5000"), Some("pct")));
    // TableProps has no spacing member in the RED baseline. Inspect the actual
    // exported XML so this is a compilable behavioral contract before adding one.
    let spacing = opening_tag(table, "tblCellSpacing");
    assert_eq!((attribute(spacing, "w"), attribute(spacing, "type")), (Some("15"), Some("dxa")));
    let start = table.find("<w:tblCellMar>").unwrap();
    let margins = &table[start..table.find("</w:tblCellMar>").unwrap()];
    for side in ["top", "left", "bottom", "right"] {
        let margin = opening_tag(margins, side);
        assert_eq!((attribute(margin, "w"), attribute(margin, "type")), (Some("15"), Some("dxa")));
    }
    let cell_width_pattern = Regex::new(r"<w:tcW(?:\s[^>]*)?/?>").unwrap();
    let cell_widths: Vec<_> = cell_width_pattern.find_iter(table).map(|tag| tag.as_str()).collect();
    assert_eq!(cell_widths.len(), 6);
    for pair in cell_widths.as_chunks::<2>().0 {
        assert_eq!((attribute(pair[0], "w"), attribute(pair[0], "type")), (Some("50"), Some("pct")));
        assert_eq!((attribute(pair[1], "w"), attribute(pair[1], "type")), (Some("0"), Some("auto")));
    }
}

#[test]
fn native_ieee_bibliography_uses_the_word_table_inside_one_field_through_refresh() {
    let session = native_bibliography();
    assert_table(&session.doc);
    let bytes = wordcraft_docx::write(&session.doc).unwrap();
    assert_exported_properties(&bytes);
    let mut session = Session::new(wordcraft_docx::read(&bytes).unwrap());
    assert_field_envelope(&session.doc);
    let block_count = session.doc.body.len();
    for _ in 0..2 {
        session.run("references.updateFields", &json!({})).unwrap();
        let bytes = wordcraft_docx::write(&session.doc).unwrap();
        assert_exported_properties(&bytes);
        session = Session::new(wordcraft_docx::read(&bytes).unwrap());
        assert_field_envelope(&session.doc);
        assert_eq!(session.doc.body.len(), block_count, "replacing the cached table must not accumulate boundary paragraphs");
    }
}
