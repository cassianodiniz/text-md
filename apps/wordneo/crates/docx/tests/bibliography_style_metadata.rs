//! Synthetic bibliography-store identity round trips; no Word-produced fixture.

use std::collections::BTreeMap;
use std::io::{Cursor, Read, Write};

use quick_xml::XmlVersion;
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use wordcraft_doc::{Document, Source};

const BIB_NS: &str = "http://schemas.openxmlformats.org/officeDocument/2006/bibliography";
const APA6: &str = "\\APASixthEditionOfficeOnline.xsl";

fn unzip(bytes: &[u8]) -> BTreeMap<String, Vec<u8>> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut parts = BTreeMap::new();
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        let name = file.name().to_string();
        let mut payload = Vec::new();
        file.read_to_end(&mut payload).unwrap();
        assert!(parts.insert(name, payload).is_none());
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

fn root_attributes(xml: &[u8]) -> Option<BTreeMap<String, String>> {
    let mut reader = Reader::from_reader(xml);
    let mut buffer = Vec::new();
    loop {
        match reader.read_event_into(&mut buffer).unwrap() {
            Event::Start(root) | Event::Empty(root) => {
                if root.local_name().as_ref() != b"Sources" {
                    return None;
                }
                let attrs: BTreeMap<_, _> = root
                    .attributes()
                    .map(|attribute| {
                        let attribute = attribute.unwrap();
                        (
                            String::from_utf8_lossy(attribute.key.as_ref()).into_owned(),
                            attribute.normalized_value(XmlVersion::Explicit1_0).unwrap().into_owned(),
                        )
                    })
                    .collect();
                return attrs.values().any(|value| value == BIB_NS).then_some(attrs);
            }
            Event::Eof => return None,
            _ => buffer.clear(),
        }
    }
}

fn bibliography_roots(parts: &BTreeMap<String, Vec<u8>>) -> Vec<(&str, BTreeMap<String, String>)> {
    parts
        .iter()
        .filter(|(name, _)| name.ends_with(".xml"))
        .filter_map(|(name, xml)| root_attributes(xml).map(|attrs| (name.as_str(), attrs)))
        .collect()
}

fn imported_fixture(sources: Vec<Source>) -> Vec<u8> {
    let mut document = Document::new();
    document.settings.bib_style = Some("APA".into());
    document.sources = sources;
    let mut parts = unzip(&wordcraft_docx::write(&document).unwrap());
    let before = parts.clone();
    let roots = bibliography_roots(&parts);
    assert_eq!(roots.len(), 1);
    let path = roots[0].0.to_string();
    let xml = String::from_utf8(parts[&path].clone()).unwrap();
    assert_eq!(xml.matches("SelectedStyle=\"\\APA.XSL\"").count(), 1);
    let changed = xml.replace("SelectedStyle=\"\\APA.XSL\"", &format!("SelectedStyle=\"{APA6}\" Version=\"6\""));
    parts.insert(path.clone(), changed.into_bytes());
    assert_eq!(parts.len(), before.len());
    for (name, payload) in &before {
        if name != &path {
            assert_eq!(&parts[name], payload, "the fixture changes only bibliography root metadata: {name}");
        }
    }
    zip(&parts)
}

fn assert_imported_identity(bytes: &[u8]) {
    let parts = unzip(bytes);
    let roots = bibliography_roots(&parts);
    assert_eq!(roots.len(), 1, "the modeled store must not also survive as opaque custom XML");
    let attrs = &roots[0].1;
    assert_eq!(attrs.get("StyleName").map(String::as_str), Some("APA"));
    assert_eq!(attrs.get("SelectedStyle").map(String::as_str), Some(APA6), "retain the imported style identity rather than synthesizing APA.XSL");
    assert_eq!(attrs.get("Version").map(String::as_str), Some("6"));
}

#[test]
fn imported_apa6_empty_store_keeps_selected_style_and_version_for_two_round_trips() {
    let mut bytes = imported_fixture(Vec::new());
    assert_imported_identity(&bytes);
    for _ in 0..2 {
        let document = wordcraft_docx::read(&bytes).unwrap();
        assert_eq!(document.settings.bib_style.as_deref(), Some("APA"));
        assert!(document.sources.is_empty());
        bytes = wordcraft_docx::write(&document).unwrap();
        assert_imported_identity(&bytes);
    }
}

#[test]
fn imported_apa6_identity_survives_populated_source_edits_and_reopen() {
    let original = Source {
        tag: "MetadataOriginal".into(),
        kind: "book".into(),
        author: "Fixture, Ari".into(),
        title: "Original synthetic source".into(),
        ..Default::default()
    };
    let added = Source {
        tag: "MetadataAdded".into(),
        kind: "article".into(),
        author: "Example, Bea".into(),
        title: "Added synthetic source".into(),
        ..Default::default()
    };
    let mut document = wordcraft_docx::read(&imported_fixture(vec![original.clone()])).unwrap();
    assert_eq!(document.sources, vec![original.clone()]);
    document.sources[0].title = "Edited synthetic source".into();
    document.sources.push(added);
    let expected = document.sources.clone();
    for _ in 0..2 {
        let bytes = wordcraft_docx::write(&document).unwrap();
        assert_imported_identity(&bytes);
        document = wordcraft_docx::read(&bytes).unwrap();
        assert_eq!(document.sources, expected, "source edits are independent of imported style metadata");
    }
}

#[test]
fn new_documents_keep_the_existing_default_bibliography_metadata_contract() {
    let mut document = Document::new();
    let parts = unzip(&wordcraft_docx::write(&document).unwrap());
    assert!(bibliography_roots(&parts).is_empty());
    assert!(!parts.keys().any(|name| name.starts_with("customXml/")));
    document.settings.bib_style = Some("APA".into());
    let parts = unzip(&wordcraft_docx::write(&document).unwrap());
    let roots = bibliography_roots(&parts);
    assert_eq!(roots.len(), 1);
    assert_eq!(roots[0].1.get("SelectedStyle").map(String::as_str), Some("\\APA.XSL"));
    assert!(!roots[0].1.contains_key("Version"));
}
