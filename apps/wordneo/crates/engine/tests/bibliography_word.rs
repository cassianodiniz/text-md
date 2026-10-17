//! Native-command regressions from the existing synthetic Word D observation.
//! Only page locators and the measured three-source selection are asserted here.

use serde_json::{Value, json};
use wordcraft_doc::Document;
use wordcraft_engine::Session;

const BOOK_TITLE: &str = "Synthetic Atlas for WordNeo Audit";
const ARTICLE_TITLE: &str = "Synthetic Article for WordNeo Audit";
const UNCITED_TITLE: &str = "Uncited Source Must Survive";

fn synthetic_sources() -> [Value; 3] {
    let book = json!({
        "tag": "SyntheticBook2030", "kind": "book", "author": "Inventada, Lia; Exemplo, Rui",
        "title": BOOK_TITLE, "year": "2030", "publisher": "Fictional Press", "city": "Imaginary City",
        "journal": "", "volume": "2", "pages": "11-29", "url": "https://example.invalid/synthetic-book",
        "editor": "Ficticio, Nilo", "edition": "3", "issue": "4", "month": "09", "day": "14",
        "accessed": "2030-10-16", "doi": "10.0000/wordneo.synthetic.book"
    });
    let mut article = book.clone();
    for (key, value) in [
        ("tag", "SyntheticArticle2031"),
        ("kind", "article"),
        ("author", "Exemplo, Tais"),
        ("title", ARTICLE_TITLE),
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
        ("title", UNCITED_TITLE),
        ("doi", "10.0000/wordneo.synthetic.uncited"),
    ] {
        uncited[key] = json!(value);
    }
    [book, article, uncited]
}

fn cited_session() -> Session {
    let mut session = Session::new(Document::new());
    session.run("text.insert", &json!({"text": "Synthetic evidence: "})).unwrap();
    for source in synthetic_sources() {
        session.run("references.sources", &json!({"add": source})).unwrap();
    }
    session.run("references.citationStyle", &json!({"style": "IEEE"})).unwrap();
    // The measured citation order is Article first, then Book; it differs from
    // the source-manager insertion order and must not be inferred from that list.
    session.run("references.citation", &json!({"tag": "SyntheticArticle2031", "pages": "17"})).unwrap();
    session.run("text.insert", &json!({"text": " and "})).unwrap();
    session.run("references.citation", &json!({"tag": "SyntheticBook2030", "pages": "5"})).unwrap();
    session
}

fn document_text(session: &mut Session) -> String {
    session.run("document.text", &json!({})).unwrap()["text"].as_str().unwrap().to_string()
}

#[test]
fn native_ieee_citations_keep_the_explicit_page_locators_seen_in_word() {
    let text = document_text(&mut cited_session());
    assert!(
        text.contains("[1, p. 17]") && text.contains("[2, p. 5]"),
        "Word D updates the Article citation to [1, p. 17] and Book to [2, p. 5]; native command results must keep both locators: {text}"
    );
}

#[test]
fn native_ieee_bibliography_includes_the_uncited_document_source_seen_in_word() {
    let mut session = cited_session();
    session.run("text.newParagraph", &json!({})).unwrap();
    session.run("references.bibliography", &json!({"title": "Synthetic References"})).unwrap();
    let text = document_text(&mut session);
    // This is the exact three-source selection and order observed in D, not a
    // policy inferred for other styles, source-manager orders or Word versions.
    for title in [ARTICLE_TITLE, BOOK_TITLE, UNCITED_TITLE] {
        assert_eq!(text.matches(title).count(), 1, "Word D includes each of the three document sources once: missing/duplicated {title}: {text}");
    }
    let article = text.find(ARTICLE_TITLE).unwrap();
    let book = text.find(BOOK_TITLE).unwrap();
    let uncited = text.find(UNCITED_TITLE).unwrap();
    assert!(article < book && book < uncited, "preserve D's measured Article, Book, Uncited order: {text}");
}
