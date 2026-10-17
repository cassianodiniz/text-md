//! Package contracts isolated by the synthetic Word bibliography 2x2 control.
use std::collections::BTreeMap;
use std::io::{Cursor, Read, Write};

use quick_xml::XmlVersion;
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use wordcraft_doc::{Document, Source};

const CUSTOM_XML: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/customXml";
const BIB_NS: &str = "http://schemas.openxmlformats.org/officeDocument/2006/bibliography";

fn source_document() -> Document {
    let mut doc = Document::new();
    doc.settings.bib_style = Some("IEEE".into());
    doc.sources = [
        ("SyntheticBook2030", "book", "Inventada, Lia; Exemplo, Rui", "Synthetic Atlas for WordNeo Audit", "2030"),
        ("SyntheticArticle2031", "article", "Exemplo, Tais", "Synthetic Article for WordNeo Audit", "2031"),
        ("SyntheticUncited2032", "book", "Inventado, Caio", "Uncited Source Must Survive", "2032"),
    ]
    .into_iter()
    .map(|(tag, kind, author, title, year)| Source {
        tag: tag.into(),
        kind: kind.into(),
        author: author.into(),
        title: title.into(),
        year: year.into(),
        ..Default::default()
    })
    .collect();
    doc
}

fn unzip(bytes: &[u8]) -> BTreeMap<String, Vec<u8>> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut parts = BTreeMap::new();
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        let name = file.name().to_string();
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).unwrap();
        assert!(parts.insert(name, bytes).is_none(), "no duplicate package parts");
    }
    parts
}

fn zip(parts: &BTreeMap<String, Vec<u8>>) -> Vec<u8> {
    let mut archive = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, bytes) in parts {
        archive.start_file(name, zip::write::SimpleFileOptions::default()).unwrap();
        archive.write_all(bytes).unwrap();
    }
    archive.finish().unwrap().into_inner()
}

fn elements(bytes: &[u8]) -> Vec<(String, BTreeMap<String, String>)> {
    let mut reader = Reader::from_reader(bytes);
    let mut buffer = Vec::new();
    let mut result = Vec::new();
    loop {
        buffer.clear();
        match reader.read_event_into(&mut buffer).unwrap() {
            Event::Start(element) | Event::Empty(element) => {
                let name = String::from_utf8_lossy(element.local_name().as_ref()).into_owned();
                let attrs = element
                    .attributes()
                    .map(|attribute| {
                        let attribute = attribute.unwrap();
                        let key = String::from_utf8_lossy(attribute.key.as_ref()).into_owned();
                        let value = attribute.normalized_value(XmlVersion::Explicit1_0).unwrap().into_owned();
                        (key, value)
                    })
                    .collect();
                result.push((name, attrs));
            }
            Event::Eof => return result,
            _ => {}
        }
    }
}

fn bibliography_parts(parts: &BTreeMap<String, Vec<u8>>) -> Vec<&str> {
    parts
        .iter()
        .filter_map(|(name, bytes)| {
            if !name.ends_with(".xml") {
                return None;
            }
            let parsed = elements(bytes);
            let (root, attrs) = parsed.first()?;
            (root == "Sources" && attrs.values().any(|value| value == BIB_NS)).then_some(name.as_str())
        })
        .collect()
}

#[test]
fn bibliography_is_related_from_the_main_document_for_word() {
    let parts = unzip(&wordcraft_docx::write(&source_document()).unwrap());
    let bib = bibliography_parts(&parts);
    assert_eq!(bib.len(), 1, "one native bibliography data store");
    let target = format!("../{}", bib[0]);
    let relationships = elements(&parts["word/_rels/document.xml.rels"]);
    assert!(
        relationships.iter().any(|(name, attrs)| {
            name == "Relationship"
                && attrs.get("Type").is_some_and(|kind| kind == CUSTOM_XML)
                && attrs.get("Target") == Some(&target)
                && attrs.get("TargetMode").is_none_or(|mode| mode != "External")
        }),
        "the native bibliography must be reachable from the main document; a package-root link alone is insufficient: {relationships:?}"
    );
}

#[test]
fn bibliography_data_part_has_application_xml_content_type_for_word() {
    let parts = unzip(&wordcraft_docx::write(&source_document()).unwrap());
    let bib = bibliography_parts(&parts);
    assert_eq!(bib.len(), 1);
    let part_name = format!("/{}", bib[0]);
    let types = elements(&parts["[Content_Types].xml"]);
    let override_type = types
        .iter()
        .find_map(|(name, attrs)| (name == "Override" && attrs.get("PartName") == Some(&part_name)).then(|| attrs.get("ContentType")).flatten());
    let content_type = override_type.or_else(|| {
        types.iter().find_map(|(name, attrs)| {
            (name == "Default" && attrs.get("Extension").is_some_and(|extension| extension == "xml")).then(|| attrs.get("ContentType")).flatten()
        })
    });
    assert_eq!(
        content_type.map(String::as_str),
        Some("application/xml"),
        "Word requires the bibliography data store to have XML content type; customXmlProperties keeps its own separate type"
    );
}

fn ensure_relationship(parts: &mut BTreeMap<String, Vec<u8>>, part: &str, target: &str) {
    if elements(&parts[part]).iter().any(|(name, attrs)| {
        name == "Relationship" && attrs.get("Type").is_some_and(|kind| kind == CUSTOM_XML) && attrs.get("Target").is_some_and(|value| value == target)
    }) {
        return;
    }
    let before = String::from_utf8(parts[part].clone()).unwrap();
    let mut id = "bibliographyDualOwnerGuard".to_string();
    while before.contains(&format!("Id=\"{id}\"")) {
        id.push('x');
    }
    let relationship = format!(r#"<Relationship Id="{id}" Type="{CUSTOM_XML}" Target="{target}"/></Relationships>"#);
    assert_eq!(before.matches("</Relationships>").count(), 1);
    parts.insert(part.into(), before.replace("</Relationships>", &relationship).into_bytes());
}

#[test]
fn bibliography_dual_owner_links_preserve_three_sources_without_duplicate_stores() {
    let expected = source_document();
    let mut parts = unzip(&wordcraft_docx::write(&expected).unwrap());
    let bib = bibliography_parts(&parts)[0].to_string();
    // Exercise the reader with both normalized relationship paths to the same
    // part, plus its conventional-name fallback; the sources must be read once.
    ensure_relationship(&mut parts, "_rels/.rels", &bib);
    ensure_relationship(&mut parts, "word/_rels/document.xml.rels", &format!("../{bib}"));
    let mut bytes = zip(&parts);
    for _ in 0..2 {
        let reopened = wordcraft_docx::read(&bytes).unwrap();
        assert_eq!(reopened.sources, expected.sources, "both owners must retain exactly the three original source records");
        assert_eq!(reopened.settings.bib_style.as_deref(), Some("IEEE"));
        bytes = wordcraft_docx::write(&reopened).unwrap();
        let saved = unzip(&bytes);
        let stores = bibliography_parts(&saved);
        assert_eq!(stores.len(), 1, "modeled bibliography must not also return as an opaque custom XML store");
        let source_count = elements(&saved[stores[0]]).iter().filter(|(name, _)| name == "Source").count();
        assert_eq!(source_count, 3, "one data store with one record per original source");
    }
}

#[test]
fn empty_bibliography_keeps_its_style_in_one_store_through_reexport() {
    let mut expected = Document::new();
    expected.settings.bib_style = Some("IEEE".into());
    assert!(expected.sources.is_empty());
    let mut bytes = wordcraft_docx::write(&expected).unwrap();
    for _ in 0..3 {
        let parts = unzip(&bytes);
        let stores = bibliography_parts(&parts);
        assert_eq!(stores.len(), 1, "the explicit style needs one metadata store even with no sources");
        let elements = elements(&parts[stores[0]]);
        assert_eq!(elements.iter().filter(|(name, _)| name == "Source").count(), 0);
        let (_, attrs) = elements.iter().find(|(name, _)| name == "Sources").unwrap();
        assert_eq!(attrs.get("StyleName").map(String::as_str), Some("IEEE"));
        let target = format!("../{}", stores[0]);
        assert!(has_document_bibliography_relationship(&parts, &target), "the main document owns the empty bibliography metadata");
        let reopened = wordcraft_docx::read(&bytes).unwrap();
        assert!(reopened.sources.is_empty());
        assert_eq!(reopened.settings.bib_style.as_deref(), Some("IEEE"));
        bytes = wordcraft_docx::write(&reopened).unwrap();
    }
}

fn has_document_bibliography_relationship(parts: &BTreeMap<String, Vec<u8>>, target: &str) -> bool {
    elements(&parts["word/_rels/document.xml.rels"]).iter().any(|(name, attrs)| {
        name == "Relationship" && attrs.get("Type").is_some_and(|kind| kind == CUSTOM_XML) && attrs.get("Target").is_some_and(|value| value == target)
    })
}

#[test]
fn a_default_document_without_sources_or_style_has_no_bibliography_metadata() {
    let document = Document::new();
    assert!(document.sources.is_empty());
    assert_eq!(document.settings.bib_style, None);
    let parts = unzip(&wordcraft_docx::write(&document).unwrap());
    assert!(bibliography_parts(&parts).is_empty());
    assert!(!parts.keys().any(|name| name.starts_with("customXml/")), "no metadata parts are added to the default document");
    for path in ["_rels/.rels", "word/_rels/document.xml.rels"] {
        assert!(
            !elements(&parts[path]).iter().any(|(name, attrs)| name == "Relationship" && attrs.get("Type").is_some_and(|kind| kind == CUSTOM_XML))
        );
    }
}
