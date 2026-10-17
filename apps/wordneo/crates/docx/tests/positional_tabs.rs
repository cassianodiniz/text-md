//! Absolute-position tabs (ECMA-376 §17.3.3.21): handwritten fixtures, no Word assets.

use std::io::{Read, Write};

use wordcraft_doc::para::{PositionalTabAlign, PositionalTabRelativeTo};
use wordcraft_doc::props::{CharProps, TabLeader};
use wordcraft_doc::{InlineObject, Path, StoryRef};

fn docx_with_footer(content: &str) -> Vec<u8> {
    let root = r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#;
    let rels = r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rIdFooter" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/footer" Target="footer1.xml"/></Relationships>"#;
    let document = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><w:body><w:p><w:r><w:t>Body</w:t></w:r></w:p><w:sectPr><w:footerReference w:type="default" r:id="rIdFooter"/></w:sectPr></w:body></w:document>"#;
    let footer = format!(r#"<w:ftr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:p>{content}</w:p></w:ftr>"#);
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for (name, data) in
        [("_rels/.rels", root), ("word/document.xml", document), ("word/_rels/document.xml.rels", rels), ("word/footer1.xml", footer.as_str())]
    {
        zip.start_file(name, zip::write::SimpleFileOptions::default()).unwrap();
        zip.write_all(data.as_bytes()).unwrap();
    }
    zip.finish().unwrap().into_inner()
}

fn footer_xml(bytes: &[u8]) -> String {
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let mut text = String::new();
    zip.by_name("word/footer1.xml").unwrap().read_to_string(&mut text).unwrap();
    text
}

#[test]
fn footer_positional_tab_retains_alignment_base_leader_and_run_format() {
    // Every schema value, including the actual right/margin/none combination that ordinary
    // tabs cannot express. Neighbouring ordinary tabs must remain ordinary tabs.
    for (alignment, align) in [("left", PositionalTabAlign::Left), ("center", PositionalTabAlign::Center), ("right", PositionalTabAlign::Right)] {
        for (base, relative_to) in [("margin", PositionalTabRelativeTo::Margin), ("indent", PositionalTabRelativeTo::Indent)] {
            for (leader_name, leader) in [
                ("none", TabLeader::None),
                ("dot", TabLeader::Dot),
                ("hyphen", TabLeader::Hyphen),
                ("underscore", TabLeader::Underscore),
                ("middleDot", TabLeader::MiddleDot),
            ] {
                let xml = format!(
                    r#"<w:r><w:t>Left</w:t><w:tab/></w:r><w:r><w:rPr><w:b/></w:rPr><w:ptab w:alignment="{alignment}" w:relativeTo="{base}" w:leader="{leader_name}"/></w:r><w:r><w:t>Right</w:t><w:tab/></w:r>"#
                );
                let original = wordcraft_docx::read(&docx_with_footer(&xml)).unwrap();
                let footer = original.sections()[0].1.footers.default.unwrap();
                let p = original.parts[&footer].blocks[0].as_para().unwrap();
                assert_eq!(p.plain_text(), "Left\t\tRight\t");
                assert_eq!(p.objects, vec![InlineObject::PositionalTab { align, relative_to, leader }]);
                // props_at has caret-left affinity at a run boundary; inspect the actual
                // object character, not the unbolded ordinary tab immediately before it.
                assert_eq!(p.props_at(p.object_offsets()[0]).bold, None);
                assert_eq!(p.props_of_char(p.object_offsets()[0]).bold, Some(true));

                let bytes = wordcraft_docx::write(&original).unwrap();
                let xml = footer_xml(&bytes);
                assert_eq!(xml.matches("<w:tab/>").count(), 2, "{xml}");
                assert!(xml.contains(&format!(r#"<w:ptab w:alignment="{alignment}" w:relativeTo="{base}" w:leader="{leader_name}"/>"#)), "{xml}");
                let back = wordcraft_docx::read(&bytes).unwrap();
                let footer = back.sections()[0].1.footers.default.unwrap();
                let again = back.parts[&footer].blocks[0].as_para().unwrap();
                assert_eq!(again.objects, p.objects);
                assert_eq!(again.plain_text(), p.plain_text());
                assert_eq!(again.props_of_char(again.object_offsets()[0]).bold, Some(true));
            }
        }
    }
}

#[test]
fn inserting_unicode_before_positional_tab_does_not_detach_its_semantics() {
    let input =
        docx_with_footer(r#"<w:r><w:t>Contact</w:t><w:ptab w:alignment="right" w:relativeTo="margin" w:leader="none"/><w:t>Footer</w:t></w:r>"#);
    let mut doc = wordcraft_docx::read(&input).unwrap();
    let footer = doc.sections()[0].1.footers.default.unwrap();
    let p = doc.para_mut(StoryRef::Part(footer), &Path(vec![0])).unwrap();
    p.insert_text(0, "ação ", &CharProps::default()).unwrap();
    let bytes = wordcraft_docx::write(&doc).unwrap();
    assert!(footer_xml(&bytes).contains(r#"<w:ptab w:alignment="right" w:relativeTo="margin" w:leader="none"/>"#));
    let back = wordcraft_docx::read(&bytes).unwrap();
    let footer = back.sections()[0].1.footers.default.unwrap();
    let p = back.parts[&footer].blocks[0].as_para().unwrap();
    assert_eq!(p.plain_text(), "ação Contact\tFooter");
    assert_eq!(p.object_offsets(), vec!["ação Contact".len()]);
}

#[test]
fn malformed_positional_tab_uses_bounded_defaults_and_stays_a_positional_tab() {
    let bytes = docx_with_footer(r#"<w:r><w:ptab w:alignment="sideways" w:relativeTo="screen" w:leader="invalid"/></w:r>"#);
    let doc = wordcraft_docx::read(&bytes).unwrap();
    let footer = doc.sections()[0].1.footers.default.unwrap();
    assert_eq!(
        doc.parts[&footer].blocks[0].as_para().unwrap().objects,
        vec![InlineObject::PositionalTab { align: PositionalTabAlign::Left, relative_to: PositionalTabRelativeTo::Margin, leader: TabLeader::None }]
    );
    assert!(footer_xml(&wordcraft_docx::write(&doc).unwrap()).contains("<w:ptab "));
}

#[test]
fn positional_tab_keeps_tracked_insertion_and_deletion_on_save() {
    for tag in ["ins", "del"] {
        let input = docx_with_footer(&format!(
            r#"<w:{tag} w:id="7" w:author="Synthetic Author"><w:r><w:ptab w:alignment="right" w:relativeTo="margin" w:leader="none"/></w:r></w:{tag}>"#
        ));
        let doc = wordcraft_docx::read(&input).unwrap();
        let bytes = wordcraft_docx::write(&doc).unwrap();
        let xml = footer_xml(&bytes);
        assert!(xml.contains(&format!("<w:{tag} ")), "{xml}");
        let back = wordcraft_docx::read(&bytes).unwrap();
        let footer = back.sections()[0].1.footers.default.unwrap();
        let p = back.parts[&footer].blocks[0].as_para().unwrap();
        let props = p.props_at(p.object_offsets()[0]);
        assert!(matches!(p.objects[0], InlineObject::PositionalTab { .. }));
        assert!(if tag == "ins" { props.ins.is_some() } else { props.del.is_some() });
    }
}
