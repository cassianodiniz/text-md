//! IEEE entry text observed in the existing synthetic Word007 update.
//! Whitespace/cell separators are normalized; punctuation and components are not.
//!
//! Additional one-property controls: Word 16.0.20430, Sources3/IEEE/Update0.
//! Evidence under wordneo/bibliography-investigation/formatting008/runs:
//! - word-20261016T2034256706947Z-e2a0da2c/word-formatting.json (16 components)
//!   SHA256 36867ae37c333d803af6b53cd41287cc61f994b1deefae147a0bdd73b62df2da.
//! - word-20261016T2038442720825Z-030d46e2/word-formatting.json (4 author lists)
//!   SHA256 35c8dcd9e52b4403f28c8a9ad5ca97c233f6cd2b4ac59f59ea96239efb1a50e0.
//!
//! The target entry is copied from result_text, not reconstructed by this formatter.
//! presentation-expectations.json in formatting008 links each case to its literal.
//! SourceType-only contrasts additionally use word-20261016T2042013162615Z-3f02546c
//! (SHA256 9c42542263317e4afe98434e8e68f04d3562a337861a321e8bad9e8b50eb8450).

use serde_json::json;
use wordcraft_doc::Source;
use wordcraft_engine::cmd::citations::entry;

fn book() -> Source {
    serde_json::from_value(json!({
        "tag": "SyntheticBook2030", "kind": "book", "author": "Inventada, Lia; Exemplo, Rui",
        "title": "Synthetic Atlas for WordNeo Audit", "year": "2030", "publisher": "Fictional Press",
        "city": "Imaginary City", "journal": "", "volume": "2", "pages": "11-29",
        "url": "https://example.invalid/synthetic-book", "editor": "Ficticio, Nilo", "edition": "3",
        "issue": "4", "month": "09", "day": "14", "accessed": "2030-10-16",
        "doi": "10.0000/wordneo.synthetic.book"
    }))
    .unwrap()
}

fn compact(text: &str) -> String {
    text.chars().filter(|c| !c.is_whitespace() && *c != '\u{7}').collect()
}

#[test]
fn ieee_full_book_entry_keeps_the_components_and_separators_observed_in_word() {
    let actual = entry(&book(), "IEEE", 2);
    let observed = "[2] L. Inventada and R. Exemplo, Synthetic Atlas for WordNeo Audit, 3 ed., vol. 2, N. Ficticio, Ed., Imaginary City: Fictional Press, 2030, pp. 11-29.";
    assert_eq!(
        compact(&actual),
        compact(observed),
        "same fictional book as Word007; do not discard edition/editor/pages or add its unrendered URL: {actual}"
    );
}

#[test]
fn ieee_full_article_entry_keeps_issue_and_date_observed_in_word() {
    let mut article = book();
    article.tag = "SyntheticArticle2031".into();
    article.kind = "article".into();
    article.author = "Exemplo, Tais".into();
    article.title = "Synthetic Article for WordNeo Audit".into();
    article.year = "2031".into();
    article.publisher = "Synthetic Publisher".into();
    article.journal = "Fictional Journal".into();
    article.volume = "7".into();
    article.issue = "2".into();
    article.pages = "101-109".into();
    article.url = "https://example.invalid/synthetic-article".into();
    article.doi = "10.0000/wordneo.synthetic.article".into();
    let actual = entry(&article, "IEEE", 1);
    let observed = "[1] T. Exemplo, \"Synthetic Article for WordNeo Audit,\" Fictional Journal, vol. 7, no. 2, pp. 101-109, 14 09 2031.";
    assert_eq!(
        compact(&actual),
        compact(observed),
        "same fictional journal article as Word007; source kind remains article when optional fields are absent: {actual}"
    );
}

fn article() -> Source {
    let mut article = book();
    article.tag = "SyntheticArticle2031".into();
    article.kind = "article".into();
    article.author = "Exemplo, Tais".into();
    article.title = "Synthetic Article for WordNeo Audit".into();
    article.year = "2031".into();
    article.publisher = "Synthetic Publisher".into();
    article.journal = "Fictional Journal".into();
    article.volume = "7".into();
    article.issue = "2".into();
    article.pages = "101-109".into();
    article.url = "https://example.invalid/synthetic-article".into();
    article.doi = "10.0000/wordneo.synthetic.article".into();
    article
}

// Each public-formatter case changes one Source property from the same baseline.
// Empty optional fields are deliberate inputs, with punctuation measured in Word.
macro_rules! ieee_word_control {
    ($name:ident, $fixture:ident, $number:literal, $field:ident, $value:literal, $case:literal, $observed:literal) => {
        #[test]
        fn $name() {
            let mut source = $fixture();
            source.$field = $value.into();
            let actual = entry(&source, "IEEE", $number);
            assert_eq!(
                compact(&actual),
                compact($observed),
                "Word008 control {}: punctuation/components must match the observed entry; actual={actual}",
                $case
            );
        }
    };
}

ieee_word_control!(
    ieee_word_book_no_edition,
    book,
    2,
    edition,
    "",
    "book-no-edition",
    "[2] L. Inventada and R. Exemplo, Synthetic Atlas for WordNeo Audit, vol. 2, N. Ficticio, Ed., Imaginary City: Fictional Press, 2030, pp. 11-29."
);
ieee_word_control!(
    ieee_word_book_no_volume,
    book,
    2,
    volume,
    "",
    "book-no-volume",
    "[2] L. Inventada and R. Exemplo, Synthetic Atlas for WordNeo Audit, 3 ed., N. Ficticio, Ed., Imaginary City: Fictional Press, 2030, pp. 11-29."
);
ieee_word_control!(
    ieee_word_book_no_editor,
    book,
    2,
    editor,
    "",
    "book-no-editor",
    "[2] L. Inventada and R. Exemplo, Synthetic Atlas for WordNeo Audit, 3 ed., vol. 2, Imaginary City: Fictional Press, 2030, pp. 11-29."
);
ieee_word_control!(
    ieee_word_book_no_pages,
    book,
    2,
    pages,
    "",
    "book-no-pages",
    "[2] L. Inventada and R. Exemplo, Synthetic Atlas for WordNeo Audit, 3 ed., vol. 2, N. Ficticio, Ed., Imaginary City: Fictional Press, 2030."
);
ieee_word_control!(
    ieee_word_book_no_url,
    book,
    2,
    url,
    "",
    "book-no-url",
    "[2] L. Inventada and R. Exemplo, Synthetic Atlas for WordNeo Audit, 3 ed., vol. 2, N. Ficticio, Ed., Imaginary City: Fictional Press, 2030, pp. 11-29."
);
ieee_word_control!(
    ieee_word_book_no_city,
    book,
    2,
    city,
    "",
    "book-no-city",
    "[2] L. Inventada and R. Exemplo, Synthetic Atlas for WordNeo Audit, 3 ed., vol. 2, N. Ficticio, Ed., Fictional Press, 2030, pp. 11-29."
);
ieee_word_control!(
    ieee_word_book_no_publisher,
    book,
    2,
    publisher,
    "",
    "book-no-publisher",
    "[2] L. Inventada and R. Exemplo, Synthetic Atlas for WordNeo Audit, 3 ed., vol. 2, N. Ficticio, Ed., Imaginary City, 2030, pp. 11-29."
);
ieee_word_control!(
    ieee_word_book_no_year,
    book,
    2,
    year,
    "",
    "book-no-year",
    "[2] L. Inventada and R. Exemplo, Synthetic Atlas for WordNeo Audit, 3 ed., vol. 2, N. Ficticio, Ed., Imaginary City: Fictional Press, pp. 11-29."
);
ieee_word_control!(
    ieee_word_book_no_author_uses_editor_initially,
    book,
    2,
    author,
    "",
    "book-no-author",
    "[2] N. Ficticio, Ed., Synthetic Atlas for WordNeo Audit, 3 ed., vol. 2, Imaginary City: Fictional Press, 2030, pp. 11-29."
);
ieee_word_control!(
    ieee_word_article_no_issue,
    article,
    1,
    issue,
    "",
    "article-no-issue",
    r#"[1] T. Exemplo, "Synthetic Article for WordNeo Audit," Fictional Journal, vol. 7, pp. 101-109, 14 09 2031."#
);
ieee_word_control!(
    ieee_word_article_no_volume,
    article,
    1,
    volume,
    "",
    "article-no-volume",
    r#"[1] T. Exemplo, "Synthetic Article for WordNeo Audit," Fictional Journal, no. 2, pp. 101-109, 14 09 2031."#
);
ieee_word_control!(
    ieee_word_article_no_pages,
    article,
    1,
    pages,
    "",
    "article-no-pages",
    r#"[1] T. Exemplo, "Synthetic Article for WordNeo Audit," Fictional Journal, vol. 7, no. 2, 14 09 2031."#
);
ieee_word_control!(
    ieee_word_article_no_day,
    article,
    1,
    day,
    "",
    "article-no-day",
    r#"[1] T. Exemplo, "Synthetic Article for WordNeo Audit," Fictional Journal, vol. 7, no. 2, pp. 101-109, 09 2031."#
);
ieee_word_control!(
    ieee_word_article_no_month_also_omits_day,
    article,
    1,
    month,
    "",
    "article-no-month",
    r#"[1] T. Exemplo, "Synthetic Article for WordNeo Audit," Fictional Journal, vol. 7, no. 2, pp. 101-109, 2031."#
);
ieee_word_control!(
    ieee_word_article_no_journal_remains_an_article,
    article,
    1,
    journal,
    "",
    "article-no-journalname",
    r#"[1] T. Exemplo, "Synthetic Article for WordNeo Audit," vol. 7, no. 2, pp. 101-109, 14 09 2031."#
);
ieee_word_control!(
    ieee_word_article_no_url,
    article,
    1,
    url,
    "",
    "article-no-url",
    r#"[1] T. Exemplo, "Synthetic Article for WordNeo Audit," Fictional Journal, vol. 7, no. 2, pp. 101-109, 14 09 2031."#
);
ieee_word_control!(
    ieee_word_book_one_author,
    book,
    2,
    author,
    "Inventada, Lia",
    "book-author1",
    "[2] L. Inventada, Synthetic Atlas for WordNeo Audit, 3 ed., vol. 2, N. Ficticio, Ed., Imaginary City: Fictional Press, 2030, pp. 11-29."
);
ieee_word_control!(
    ieee_word_book_three_authors,
    book,
    2,
    author,
    "Inventada, Lia; Exemplo, Rui; Amostra, Mara",
    "book-author3",
    "[2] L. Inventada, R. Exemplo and M. Amostra, Synthetic Atlas for WordNeo Audit, 3 ed., vol. 2, N. Ficticio, Ed., Imaginary City: Fictional Press, 2030, pp. 11-29."
);
ieee_word_control!(
    ieee_word_article_two_authors,
    article,
    1,
    author,
    "Exemplo, Tais; Ilustracao, Bento",
    "article-author2",
    r#"[1] T. Exemplo and B. Ilustracao, "Synthetic Article for WordNeo Audit," Fictional Journal, vol. 7, no. 2, pp. 101-109, 14 09 2031."#
);
ieee_word_control!(
    ieee_word_article_three_authors,
    article,
    1,
    author,
    "Exemplo, Tais; Ilustracao, Bento; Amostra, Mara",
    "article-author3",
    r#"[1] T. Exemplo, B. Ilustracao and M. Amostra, "Synthetic Article for WordNeo Audit," Fictional Journal, vol. 7, no. 2, pp. 101-109, 14 09 2031."#
);
ieee_word_control!(
    ieee_word_article_payload_as_book_ignores_filled_journal,
    article,
    1,
    kind,
    "book",
    "article-as-book",
    "[1] T. Exemplo, Synthetic Article for WordNeo Audit, 3 ed., vol. 7, N. Ficticio, Ed., Imaginary City: Synthetic Publisher, 2031, pp. 101-109."
);
ieee_word_control!(
    ieee_word_book_payload_as_article_keeps_article_punctuation_without_journal,
    book,
    2,
    kind,
    "article",
    "book-as-article",
    r#"[2] L. Inventada and R. Exemplo, "Synthetic Atlas for WordNeo Audit," vol. 2, no. 4, pp. 11-29, 14 09 2030."#
);
