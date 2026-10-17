//! Run properties observed in the fictional Word008 result XML, not Word assets.
use serde_json::json;
use wordcraft_doc::{Document, StoryRef};
use wordcraft_engine::Session;

fn fixture() -> Session {
    let mut s = Session::new(Document::new());
    s.run("references.citationStyle", &json!({"style":"IEEE"})).unwrap();
    let mut source = json!({
        "tag":"SyntheticBook2030", "kind":"book", "author":"Inventada, Lia; Exemplo, Rui",
        "title":"Synthetic Atlas for WordNeo Audit", "year":"2030", "publisher":"Fictional Press",
        "city":"Imaginary City", "volume":"2", "pages":"11-29", "editor":"Ficticio, Nilo",
        "edition":"3", "issue":"4", "month":"09", "day":"14",
        "url":"https://example.invalid/synthetic-book", "accessed":"2030-10-16", "doi":"10.0000/wordneo.synthetic.book"
    });
    s.run("references.sources", &json!({"add":source})).unwrap();
    for (key, value) in [
        ("tag", "SyntheticArticle2031"),
        ("kind", "article"),
        ("author", "Exemplo, Tais"),
        ("title", "Synthetic Article for WordNeo Audit"),
        ("year", "2031"),
        ("publisher", "Synthetic Publisher"),
        ("journal", "Fictional Journal"),
        ("volume", "7"),
        ("issue", "2"),
        ("pages", "101-109"),
        ("url", "https://example.invalid/synthetic-article"),
        ("doi", "10.0000/wordneo.synthetic.article"),
    ] {
        source[key] = json!(value);
    }
    s.run("references.sources", &json!({"add":source})).unwrap();
    s.run("references.bibliography", &json!({"title":"Synthetic References"})).unwrap();
    s
}

#[test]
fn generated_ieee_number_and_entry_runs_keep_words_no_proof_property() {
    let mut s = fixture();
    for _ in 0..2 {
        let table = s.doc.body.iter().find_map(|b| b.as_table()).unwrap();
        assert_eq!(table.rows.len(), 2);
        for row in &table.rows {
            for cell in &row.cells {
                for p in cell.blocks.iter().filter_map(|b| b.as_para()) {
                    assert!(!p.text.is_empty());
                    for run in &p.runs {
                        assert_eq!(run.props.no_proof, Some(true), "Word008 marks both number and source text noProof: {}", p.plain_text());
                    }
                }
            }
        }
        s = Session::new(wordcraft_docx::read(&wordcraft_docx::write(&s.doc).unwrap()).unwrap());
        s.run("references.updateFields", &json!({})).unwrap();
    }
}

#[test]
fn only_the_measured_journal_segment_adds_italic_and_complex_script_italic() {
    let mut s = fixture();
    for _ in 0..2 {
        let table = s.doc.body.iter().find_map(|b| b.as_table()).unwrap();
        let mut journals = 0;
        for p in table.rows.iter().flat_map(|r| &r.cells).flat_map(|c| &c.blocks).filter_map(|b| b.as_para()) {
            let mut off = 0;
            for run in &p.runs {
                let text = p.text.get(off..off + run.len).unwrap();
                off += run.len;
                let expected = text == "Fictional Journal, ";
                assert_eq!(run.props.italic, expected.then_some(true), "{text}");
                assert_eq!(run.props.italic_cs, expected.then_some(true), "{text}");
                journals += usize::from(expected);
            }
        }
        assert_eq!(journals, 1);
        assert_eq!(s.doc.field_ranges(StoryRef::Body).len(), 1);
        s = Session::new(wordcraft_docx::read(&wordcraft_docx::write(&s.doc).unwrap()).unwrap());
        s.run("references.updateFields", &json!({})).unwrap();
    }
}
