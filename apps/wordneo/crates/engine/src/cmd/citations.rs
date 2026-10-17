//! References: citations and bibliography (APA, MLA, Chicago, IEEE, Harvard, ISO 690, Turabian,
//! GB/T 7714, GOST, SIST02), source manager, index, table of figures, cross-references, table
//! of authorities, footnote options.

use serde_json::{Value, json};
use wordcraft_doc::collate::{SortKey, heading_letter, sort_key};
use wordcraft_doc::para::InlineObject;
use wordcraft_doc::props::{CharProps, TabAlign, TabLeader, TabStop};
use wordcraft_doc::table::{Cell, Row, Table};
use wordcraft_doc::{Block, Document, Paragraph, Path, Pos, Source, StoryRef};

use super::{delete_selection, sel_result};
use crate::{CmdError, CmdResult, CommandSpec, Selection, Session, p};

/// How a style orders its bibliography (and, for numbered styles, numbers its citations).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Order {
    /// Alphabetical by the first author's surname (the title when there is no author).
    Author,
    /// Alphabetical by title.
    Title,
    /// In order of first citation in the document.
    Cited,
}

/// How one author's name is written in a bibliography entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Name {
    /// Rivera, Alex
    LastFirst,
    /// Alex Rivera
    FirstLast,
    /// Rivera, A.
    LastInitials,
    /// A. Rivera
    InitialsLast,
    /// Rivera A.
    LastSpaceInitials,
    /// RIVERA, Alex
    UpperLastFirst,
    /// RIVERA A
    UpperLastBareInitials,
}

/// The author list of a bibliography entry.
#[derive(Clone, Copy, Debug)]
pub struct Names {
    /// The first author's form, and every later author's.
    pub first: Name,
    pub rest: Name,
    /// Between names; between exactly two; before the last of three or more.
    pub sep: &'static str,
    pub two: &'static str,
    pub last: &'static str,
    /// List at most this many authors, then `et_al` (0: list them all).
    pub max: usize,
    pub et_al: &'static str,
}

/// The in-text citation.
#[derive(Clone, Copy, Debug)]
pub enum Cite {
    /// `(Who<year_sep>Year<pages_sep>Pages)`.
    AuthorDate { year_sep: &'static str, pages_sep: &'static str },
    /// `(Who Pages)`.
    AuthorPage,
    /// `<open>n<close>`; pages follow `pages_sep` inside the brackets (left out when `None`).
    Number { open: &'static str, close: &'static str, pages_sep: Option<&'static str> },
}

/// A citation and bibliography style, as data. Entry templates fill `{n}`, `{names}`, `{year}`,
/// `{title}`, `{city}`, `{publisher}`, `{pub}` (city: publisher), `{journal}`, `{volume}`,
/// `{pages}`, `{url}` and `{type}` (GB/T 7714 document type code); text between `⟦` and `⟧` is
/// left out when a field in it is empty, and a full stop never doubles up.
#[derive(Debug)]
pub struct CiteStyle {
    /// The name shown in the style picker and stored in the session.
    pub name: &'static str,
    /// Lowercase letter/digit prefixes of Word's style names and style-sheet file names that mean
    /// this style (`gostname` matches "GOST - Name Sort"); the longest match wins.
    pub aliases: &'static [&'static str],
    pub cite: Cite,
    /// Joins two authors in a citation ("&" or "and").
    pub and: &'static str,
    /// Cite this many authors or more as "First et al.".
    pub et_al_from: usize,
    pub order: Order,
    pub names: Names,
    /// Entry template for a book (or anything without a journal), and for a journal article.
    pub book: &'static str,
    pub article: &'static str,
    /// Default title of the bibliography.
    pub bib_title: &'static str,
}

impl CiteStyle {
    /// Numbered styles cite `[1]`/`(1)` and list entries without a hanging indent.
    pub fn numbered(&self) -> bool {
        matches!(self.cite, Cite::Number { .. })
    }
}

const fn names(first: Name, rest: Name, sep: &'static str, two: &'static str, last: &'static str) -> Names {
    Names { first, rest, sep, two, last, max: 0, et_al: "" }
}

const AUTHOR_DATE: Cite = Cite::AuthorDate { year_sep: " ", pages_sep: ", p. " };

/// Every style, in picker order (alphabetical). Formats follow the published style guides'
/// rules for the source types WordCraft records, written in our own words.
pub static STYLES: [CiteStyle; 12] = [
    CiteStyle {
        name: "APA",
        aliases: &["apa"],
        cite: Cite::AuthorDate { year_sep: ", ", pages_sep: ", p. " },
        and: "&",
        et_al_from: 3,
        order: Order::Author,
        names: names(Name::LastInitials, Name::LastInitials, ", ", ", & ", ", & "),
        book: "⟦{names} ⟧({year}). {title}.⟦ {publisher}.⟧⟦ {url}⟧",
        article: "⟦{names} ⟧({year}). {title}. {journal}⟦, {volume}⟧⟦, {pages}⟧.⟦ {url}⟧",
        bib_title: "References",
    },
    CiteStyle {
        name: "Chicago",
        aliases: &["chicago"],
        cite: Cite::AuthorDate { year_sep: " ", pages_sep: ", " },
        and: "and",
        et_al_from: 3,
        order: Order::Author,
        names: names(Name::LastFirst, Name::FirstLast, ", ", ", and ", ", and "),
        book: "⟦{names}. ⟧{year}. {title}.⟦ {pub}.⟧⟦ {url}⟧",
        article: "⟦{names}. ⟧{year}. {title}. {journal}⟦, {volume}⟧⟦, {pages}⟧.⟦ {url}⟧",
        bib_title: "References",
    },
    // GB/T 7714 sequence-coding: numbered by first citation, surnames in capitals with bare
    // initials, at most three authors, a document type code after the title.
    CiteStyle {
        name: "GB/T 7714",
        aliases: &["gb"],
        cite: Cite::Number { open: "[", close: "]", pages_sep: Some(", ") },
        and: "and",
        et_al_from: 3,
        order: Order::Cited,
        names: Names { max: 3, et_al: ", et al.", ..names(Name::UpperLastBareInitials, Name::UpperLastBareInitials, ", ", ", ", ", ") },
        book: "[{n}] ⟦{names}. ⟧{title}[{type}].⟦ {pub},⟧ {year}.⟦ {url}.⟧",
        article: "[{n}] ⟦{names}. ⟧{title}[{type}]. {journal}, {year}⟦, {volume}⟧⟦: {pages}⟧.⟦ {url}.⟧",
        bib_title: "References",
    },
    // GOST 7.1: a numbered list in alphabetical order (by author or by title); areas separated
    // by " – ", the container after "//".
    CiteStyle {
        name: "GOST (name sort)",
        aliases: &["gost", "gostname"],
        cite: Cite::Number { open: "[", close: "]", pages_sep: Some(", p. ") },
        and: "and",
        et_al_from: 3,
        order: Order::Author,
        names: names(Name::LastSpaceInitials, Name::LastSpaceInitials, ", ", ", ", ", "),
        book: GOST_BOOK,
        article: GOST_ARTICLE,
        bib_title: "References",
    },
    CiteStyle {
        name: "GOST (title sort)",
        aliases: &["gosttitle"],
        cite: Cite::Number { open: "[", close: "]", pages_sep: Some(", p. ") },
        and: "and",
        et_al_from: 3,
        order: Order::Title,
        names: names(Name::LastSpaceInitials, Name::LastSpaceInitials, ", ", ", ", ", "),
        book: GOST_BOOK,
        article: GOST_ARTICLE,
        bib_title: "References",
    },
    // Harvard as taught by Anglia Ruskin: "Surname, I., Year. Title. Place: Publisher."
    CiteStyle {
        name: "Harvard (Anglia)",
        aliases: &["harvard"],
        cite: Cite::AuthorDate { year_sep: ", ", pages_sep: ", p. " },
        and: "and",
        et_al_from: 4,
        order: Order::Author,
        names: names(Name::LastInitials, Name::LastInitials, ", ", " and ", " and "),
        book: "⟦{names}, ⟧{year}. {title}.⟦ {pub}.⟧⟦ Available at: <{url}>.⟧",
        article: "⟦{names}, ⟧{year}. {title}. {journal}⟦, {volume}⟧⟦, pp. {pages}⟧.⟦ Available at: <{url}>.⟧",
        bib_title: "References",
    },
    CiteStyle {
        name: "IEEE",
        aliases: &["ieee"],
        cite: Cite::Number { open: "[", close: "]", pages_sep: Some(", p. ") },
        and: "and",
        et_al_from: 3,
        order: Order::Cited,
        names: names(Name::InitialsLast, Name::InitialsLast, ", ", ", ", ", "),
        book: "[{n}] ⟦{names}, ⟧“{title},”⟦ {pub},⟧ {year}.⟦ {url}⟧",
        article: "[{n}] ⟦{names}, ⟧“{title},” {journal}⟦, {volume}⟧⟦, {pages}⟧, {year}.⟦ {url}⟧",
        bib_title: "References",
    },
    // ISO 690 first element and date: surnames in capitals, the year after the names.
    CiteStyle {
        name: "ISO 690 (author-date)",
        aliases: &["iso690"],
        cite: AUTHOR_DATE,
        and: "and",
        et_al_from: 4,
        order: Order::Author,
        names: names(Name::UpperLastFirst, Name::UpperLastFirst, ", ", " and ", " and "),
        book: "⟦{names}, ⟧{year}. {title}.⟦ {pub}.⟧⟦ Available from: {url}⟧",
        article: "⟦{names}, ⟧{year}. {title}. {journal}⟦, {volume}⟧⟦, {pages}⟧.⟦ Available from: {url}⟧",
        bib_title: "References",
    },
    // ISO 690 numeric reference: numbered by first citation, the year with the imprint.
    CiteStyle {
        name: "ISO 690 (numerical)",
        aliases: &["iso690num", "iso690nmerical"],
        cite: Cite::Number { open: "(", close: ")", pages_sep: Some(", p. ") },
        and: "and",
        et_al_from: 4,
        order: Order::Cited,
        names: names(Name::UpperLastFirst, Name::UpperLastFirst, ", ", " and ", " and "),
        book: "{n}. ⟦{names}. ⟧{title}.⟦ {pub},⟧ {year}.⟦ Available from: {url}⟧",
        article: "{n}. ⟦{names}. ⟧{title}. {journal}. {year}⟦, {volume}⟧⟦, {pages}⟧.⟦ Available from: {url}⟧",
        bib_title: "References",
    },
    CiteStyle {
        name: "MLA",
        aliases: &["mla"],
        cite: Cite::AuthorPage,
        and: "and",
        et_al_from: 3,
        order: Order::Author,
        names: Names { max: 1, et_al: ", et al.", ..names(Name::LastFirst, Name::LastFirst, ", ", ", ", ", ") },
        book: "⟦{names}. ⟧“{title}.”⟦ {publisher},⟧ {year}.⟦ {url}⟧",
        article: "⟦{names}. ⟧“{title}.” {journal}⟦, {volume}⟧⟦, {pages}⟧, {year}.⟦ {url}⟧",
        bib_title: "Works Cited",
    },
    // SIST 02: numbered by first citation, full names separated by semicolons.
    CiteStyle {
        name: "SIST02",
        aliases: &["sist"],
        cite: Cite::Number { open: "(", close: ")", pages_sep: Some(", p. ") },
        and: "and",
        et_al_from: 3,
        order: Order::Cited,
        names: names(Name::LastFirst, Name::LastFirst, "; ", "; ", "; "),
        book: "{n}) ⟦{names}. ⟧{title}.⟦ {city},⟧⟦ {publisher},⟧ {year}.⟦ {url}⟧",
        article: "{n}) ⟦{names}. ⟧{title}. {journal}. {year}⟦, vol. {volume}⟧⟦, p. {pages}⟧.⟦ {url}⟧",
        bib_title: "References",
    },
    // Turabian: author-date citations, a bibliography with the year in the imprint.
    CiteStyle {
        name: "Turabian",
        aliases: &["turabian"],
        cite: Cite::AuthorDate { year_sep: " ", pages_sep: ", " },
        and: "and",
        et_al_from: 4,
        order: Order::Author,
        names: names(Name::LastFirst, Name::FirstLast, ", ", ", and ", ", and "),
        book: "⟦{names}. ⟧{title}.⟦ {pub},⟧ {year}.⟦ {url}.⟧",
        article: "⟦{names}. ⟧“{title}.” {journal}⟦ {volume}⟧ ({year})⟦: {pages}⟧.⟦ {url}.⟧",
        bib_title: "Bibliography",
    },
];

const GOST_BOOK: &str = "{n}. ⟦{names} ⟧{title}. –⟦ {city} :⟧⟦ {publisher},⟧ {year}.⟦ – URL: {url}.⟧";
const GOST_ARTICLE: &str = "{n}. ⟦{names} ⟧{title} // {journal}. – {year}.⟦ – Vol. {volume}.⟧⟦ – P. {pages}.⟧⟦ – URL: {url}.⟧";

/// The style called `name`: one of ours (any case), or Word's style name or style-sheet file
/// name (`Harvard - Anglia`, `\GostTitle.XSL`).
pub fn find_style(name: &str) -> Option<&'static CiteStyle> {
    if let Some(st) = STYLES.iter().find(|x| x.name.eq_ignore_ascii_case(name.trim())) {
        return Some(st);
    }
    let key: String = name.chars().filter(char::is_ascii_alphanumeric).map(|c| c.to_ascii_lowercase()).collect();
    let key = key.strip_suffix("xsl").unwrap_or(&key);
    STYLES
        .iter()
        .flat_map(|st| st.aliases.iter().map(move |a| (st, *a)))
        .filter(|(_, a)| key.starts_with(a))
        .max_by_key(|(_, a)| a.len())
        .map(|(st, _)| st)
}

/// The session's style; APA when the name is unknown.
pub fn style(name: &str) -> &'static CiteStyle {
    find_style(name).unwrap_or(&STYLES[0])
}

pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("references.sources", "Manage Sources", "References › Citations & Bibliography", sources)
            .params(r#"{"add"?: Source, "remove"?: tag} → list"#),
        CommandSpec::new("references.citation", "Insert Citation", "References › Citations & Bibliography", citation).params(r#"{"tag"?: string, "source"?: Source (added if new), "pages"?: string}"#),
        CommandSpec::new("references.citationStyle", "Style", "References › Citations & Bibliography", |s, v| {
            let st = p::str(v, "style").unwrap_or("APA");
            let st = find_style(st).ok_or_else(|| {
                CmdError::Params(format!("style must be one of {:?}", STYLES.iter().map(|x| x.name).collect::<Vec<_>>()))
            })?;
            if s.doc.settings.bib_style_metadata.as_ref().is_some_and(|metadata| metadata.style_name != st.name) {
                s.doc.settings.bib_style_metadata = None;
            }
            s.bib_style = st.name.to_string();
            // Recorded in the document too, so a save keeps it.
            s.doc.settings.bib_style = Some(st.name.to_string());
            update_citations(s)?;
            Ok(json!({"style": s.bib_style}))
        })
        .params(
            r#"{"style": "APA|Chicago|GB/T 7714|GOST (name sort)|GOST (title sort)|Harvard (Anglia)|IEEE|ISO 690 (author-date)|ISO 690 (numerical)|MLA|SIST02|Turabian" (Word's style names work too)}"#,
        ),
        CommandSpec::new("references.bibliography", "Bibliography", "References › Citations & Bibliography", |s, v| {
            let title = p::str(v, "title").unwrap_or(style(&s.bib_style).bib_title).to_string();
            generated_list(s, "BIBLIOGRAPHY", &title)?;
            sel_result(s)
        })
        .params(r#"{"title"?: string}"#),
        CommandSpec::new("references.tableOfFigures", "Insert Table of Figures", "References › Captions", |s, v| {
            let label = p::str(v, "label").unwrap_or("Figure").to_string();
            generated_list(s, &format!("TOC \\c \"{label}\""), "")?;
            sel_result(s)
        })
        .params(r#"{"label"?: "Figure|Table|Equation"}"#),
        CommandSpec::new("references.updateFigures", "Update Table", "References › Captions", |s, _| {
            update_generated(s)?;
            sel_result(s)
        }),
        CommandSpec::new("references.markCitation", "Mark Citation", "References › Table of Authorities", |s, v| {
            let entry = p::str(v, "entry").map(str::to_string).unwrap_or_else(|| s.selected_text().trim().to_string());
            if entry.is_empty() {
                return Err(CmdError::Params("select a citation or give `entry`".into()));
            }
            let (_, b) = s.sel.ordered();
            s.doc.insert_object(&b, InlineObject::Field { instr: format!("TA \\l \"{}\"", entry.replace('"', "")), result: String::new(), locked: false, code: false }, &CharProps { hidden: Some(true), ..Default::default() })?;
            Ok(json!({"entry": entry}))
        })
        .params(r#"{"entry"?: string}"#)
        .key(super::non_mac("Alt+Shift+I")),
        CommandSpec::new("references.tableOfAuthorities", "Insert Table of Authorities", "References › Table of Authorities", |s, _| {
            generated_list(s, "TOA", "Table of Authorities")?;
            sel_result(s)
        }),
        CommandSpec::new("references.updateAuthorities", "Update Table", "References › Table of Authorities", |s, _| {
            update_generated(s)?;
            sel_result(s)
        }),
        CommandSpec::new("insert.crossReference", "Cross-reference", "Insert › Links", cross_ref)
            .params(r#"{"to": "heading|bookmark|figure|table", "target": string (text / name / number), "show"?: "text|page|number|aboveBelow"} or {} to list targets"#),
        CommandSpec::new("references.noteOptions", "Footnote and Endnote", "References › Footnotes", super::references::note_options).params(
            r#"{"kind"?: "footnote|endnote", "pos"?: "pageBottom|beneathText" (footnotes) | "sectEnd|docEnd" (endnotes), "numFmt"?: "decimal|lowerRoman|upperRoman|lowerLetter|upperLetter|…", "numStart"?: 1-32767, "numRestart"?: "continuous|eachSect|eachPage" (each page: footnotes only), "scope"?: "document|section", "footnoteFormat"?: …, "endnoteFormat"?: …} — no settings: the options in effect at the caret"#,
        ),
        CommandSpec::new("references.researcher", "Researcher", "References › Research", |s, _| {
            s.status = "Researcher needs an online service; WordCraft keeps your documents offline.".into();
            Ok(json!({"available": false}))
        })
        .pure(),
    ]
}

fn authors(a: &str) -> Vec<(String, String)> {
    a.split(';')
        .map(str::trim)
        .filter(|x| !x.is_empty())
        .map(|x| match x.split_once(',') {
            Some((last, first)) => (last.trim().to_string(), first.trim().to_string()),
            None => match x.rsplit_once(' ') {
                Some((first, last)) => (last.trim().to_string(), first.trim().to_string()),
                None => (x.to_string(), String::new()),
            },
        })
        .collect()
}

/// "Alex Mei" → "A. M." (`dots`), or "A M" without them.
fn initials(first: &str, dots: bool) -> String {
    let sep = if dots { ". " } else { " " };
    let mut out = first.split_whitespace().filter_map(|w| w.chars().next()).map(String::from).collect::<Vec<_>>().join(sep);
    if dots && !out.is_empty() {
        out.push('.');
    }
    out
}

fn name(form: Name, last: &str, first: &str) -> String {
    let join = |a: String, sep: &str, b: String| if a.is_empty() || b.is_empty() { format!("{a}{b}") } else { format!("{a}{sep}{b}") };
    match form {
        Name::LastFirst => join(last.to_string(), ", ", first.to_string()),
        Name::FirstLast => join(first.to_string(), " ", last.to_string()),
        Name::LastInitials => join(last.to_string(), ", ", initials(first, true)),
        Name::InitialsLast => join(initials(first, true), " ", last.to_string()),
        Name::LastSpaceInitials => join(last.to_string(), " ", initials(first, true)),
        Name::UpperLastFirst => join(last.to_uppercase(), ", ", first.to_string()),
        Name::UpperLastBareInitials => join(last.to_uppercase(), " ", initials(first, false)),
    }
}

fn name_list(au: &[(String, String)], form: &Names) -> String {
    let shown = if form.max > 0 { au.get(..form.max).unwrap_or(au) } else { au };
    let parts: Vec<String> = shown.iter().enumerate().map(|(i, (l, f))| name(if i == 0 { form.first } else { form.rest }, l, f)).collect();
    let mut out = match parts.as_slice() {
        [] => String::new(),
        [one] => one.clone(),
        [a, b] => format!("{a}{}{b}", form.two),
        [init @ .., z] => format!("{}{}{z}", init.join(form.sep), form.last),
    };
    if shown.len() < au.len() {
        out.push_str(form.et_al);
    }
    out
}

/// Append `s`, dropping its leading full stop when `out` already ends with one ("M." + ". ").
fn push_text(out: &mut String, s: &str) {
    match s.strip_prefix('.') {
        Some(rest) if out.ends_with('.') => out.push_str(rest),
        _ => out.push_str(s),
    }
}

/// Fill one template segment; `None` when `optional` and a field in it is empty.
fn fill_segment(seg: &str, field: &dyn Fn(&str) -> String, optional: bool) -> Option<String> {
    let mut out = String::new();
    let mut rest = seg;
    while let Some((before, after)) = rest.split_once('{') {
        push_text(&mut out, before);
        let Some((key, after)) = after.split_once('}') else {
            push_text(&mut out, after);
            return Some(out);
        };
        let value = field(key);
        if value.is_empty() && optional {
            return None;
        }
        push_text(&mut out, &value);
        rest = after;
    }
    push_text(&mut out, rest);
    Some(out)
}

/// Fill an entry template (see [`CiteStyle`]).
fn render(template: &str, field: &dyn Fn(&str) -> String) -> String {
    let mut out = String::new();
    let mut rest = template;
    while !rest.is_empty() {
        let (seg, optional, after) = match rest.strip_prefix('⟦') {
            Some(group) => match group.split_once('⟧') {
                Some((g, after)) => (g, true, after),
                None => (group, true, ""),
            },
            None => match rest.split_once('⟦') {
                Some((plain, _)) => (plain, false, rest.get(plain.len()..).unwrap_or("")),
                None => (rest, false, ""),
            },
        };
        if let Some(text) = fill_segment(seg, field, optional) {
            push_text(&mut out, &text);
        }
        rest = after;
    }
    out
}

/// In-text citation for a style; `n` is the source's number in numbered styles.
pub fn cite(src: &Source, style_name: &str, n: usize, pages: &str) -> String {
    let st = style(style_name);
    let au = authors(&src.author);
    let lasts: Vec<&str> = au.iter().map(|a| a.0.as_str()).collect();
    let who = match lasts.as_slice() {
        [] => src.title.clone(),
        [one] => one.to_string(),
        [first, ..] if lasts.len() >= st.et_al_from.max(2) => format!("{first} et al."),
        [a, b] => format!("{a} {} {b}", st.and),
        [init @ .., z] => format!("{} {} {z}", init.join(", "), st.and),
    };
    match st.cite {
        Cite::AuthorPage if pages.is_empty() => format!("({who})"),
        Cite::AuthorPage => format!("({who} {pages})"),
        Cite::AuthorDate { year_sep, .. } if pages.is_empty() => format!("({who}{year_sep}{})", src.year),
        Cite::AuthorDate { year_sep, pages_sep } => format!("({who}{year_sep}{}{pages_sep}{pages})", src.year),
        Cite::Number { open, close, pages_sep: Some(sep) } if !pages.is_empty() => format!("{open}{n}{sep}{pages}{close}"),
        Cite::Number { open, close, .. } => format!("{open}{n}{close}"),
    }
}

/// GB/T 7714 document type code: M book, J journal article, R report, EB/OL web page, Z other;
/// "/OL" marks a source read online.
fn gb_type(src: &Source) -> String {
    let code = match src.kind.to_ascii_lowercase().as_str() {
        "website" | "web" => return "EB/OL".into(),
        _ if !src.journal.is_empty() => "J",
        "book" | "" => "M",
        "article" => "J",
        "report" => "R",
        _ => "Z",
    };
    if src.url.is_empty() { code.into() } else { format!("{code}/OL") }
}

/// Bibliography entry for a style; `n` is the source's number in numbered styles.
pub fn entry(src: &Source, style_name: &str, n: usize) -> String {
    let st = style(style_name);
    if st.name == "IEEE"
        && let Some(parts) = ieee_parts(src)
    {
        let text: String = parts.into_iter().map(|(text, _)| text).collect();
        return format!("[{n}] {text}");
    }
    let au = authors(&src.author);
    let field = |key: &str| -> String {
        match key {
            "n" => n.to_string(),
            "names" => name_list(&au, &st.names),
            "year" if src.year.is_empty() => "n.d.".into(),
            "year" => src.year.clone(),
            "title" => src.title.clone(),
            "city" => src.city.clone(),
            "publisher" => src.publisher.clone(),
            "pub" => [src.city.as_str(), src.publisher.as_str()].into_iter().filter(|x| !x.is_empty()).collect::<Vec<_>>().join(": "),
            "journal" => src.journal.clone(),
            "volume" => src.volume.clone(),
            "pages" => src.pages.clone(),
            "url" => src.url.clone(),
            "type" => gb_type(src),
            _ => String::new(),
        }
    };
    render(if src.journal.is_empty() { st.book } else { st.article }, &field)
}

/// Book/JournalArticle presentation measured with the fictional Word008 controls.
/// The bool marks only the journal and its following comma/space as italic.
/// Other source types keep their existing formatter until measured separately.
fn ieee_parts(src: &Source) -> Option<Vec<(String, bool)>> {
    let article = match src.kind.to_ascii_lowercase().as_str() {
        "book" | "" => false, // Empty kind is exported as SourceType Book.
        "article" | "journal" | "journalarticle" => true,
        _ => return None,
    };
    let forms = names(Name::InitialsLast, Name::InitialsLast, ", ", " and ", " and ");
    let author = name_list(&authors(&src.author), &forms);
    let editor = name_list(&authors(&src.editor), &forms);
    if !article {
        let mut components = Vec::new();
        if !author.is_empty() {
            components.push(author);
        } else if !editor.is_empty() {
            components.push(format!("{editor}, Ed."));
        }
        if !src.title.is_empty() {
            components.push(src.title.clone());
        }
        if !src.edition.is_empty() {
            components.push(format!("{} ed.", src.edition));
        }
        if !src.volume.is_empty() {
            components.push(format!("vol. {}", src.volume));
        }
        if !src.author.is_empty() && !editor.is_empty() {
            components.push(format!("{editor}, Ed."));
        }
        let publisher = [src.city.as_str(), src.publisher.as_str()].into_iter().filter(|x| !x.is_empty()).collect::<Vec<_>>().join(": ");
        if !publisher.is_empty() {
            components.push(publisher);
        }
        if !src.year.is_empty() {
            components.push(src.year.clone());
        }
        if !src.pages.is_empty() {
            components.push(format!("pp. {}", src.pages));
        }
        return Some(vec![(format!("{}.", components.join(", ")), false)]);
    }
    let mut parts = Vec::new();
    let mut prefix = String::new();
    if !author.is_empty() {
        prefix.push_str(&format!("{author}, "));
    }
    if !src.title.is_empty() {
        prefix.push_str(&format!("\"{},\" ", src.title));
    }
    if !prefix.is_empty() {
        parts.push((prefix, false));
    }
    let mut components = Vec::new();
    if !src.volume.is_empty() {
        components.push(format!("vol. {}", src.volume));
    }
    if !src.issue.is_empty() {
        components.push(format!("no. {}", src.issue));
    }
    if !src.pages.is_empty() {
        components.push(format!("pp. {}", src.pages));
    }
    let mut date = Vec::new();
    if !src.month.is_empty() {
        if !src.day.is_empty() {
            date.push(src.day.as_str());
        }
        date.push(src.month.as_str());
    }
    if !src.year.is_empty() {
        date.push(src.year.as_str());
    }
    if !date.is_empty() {
        components.push(date.join(" "));
    }
    if !src.journal.is_empty() {
        parts.push((format!("{}{}", src.journal, if components.is_empty() { "" } else { ", " }), true));
    }
    parts.push((format!("{}.", components.join(", ")), false));
    Some(parts)
}

fn sources(s: &mut Session, v: &Value) -> CmdResult {
    if let Some(a) = v.get("add") {
        let mut src: Source = serde_json::from_value(a.clone()).map_err(|e| CmdError::Params(e.to_string()))?;
        if src.tag.is_empty() {
            let base: String = authors(&src.author)
                .first()
                .map(|x| x.0.clone())
                .unwrap_or_else(|| "Source".into())
                .chars()
                .filter(|c| c.is_alphanumeric())
                .collect();
            src.tag = format!("{base}{}", src.year);
        }
        // Edit Source keeps the source's place in the list and refreshes its citations.
        match s.doc.sources.iter().position(|x| x.tag == src.tag) {
            Some(i) => {
                if let Some(x) = s.doc.sources.get_mut(i) {
                    *x = src;
                }
                update_citations(s)?;
            }
            None => s.doc.sources.push(src),
        }
        s.touch();
    }
    if let Some(t) = p::str(v, "remove") {
        s.doc.sources.retain(|x| x.tag != t);
    }
    Ok(serde_json::to_value(&s.doc.sources).unwrap_or(Value::Null))
}

fn citation(s: &mut Session, v: &Value) -> CmdResult {
    if v.get("source").is_some() {
        sources(s, &json!({"add": v.get("source")}))?;
    }
    let tag = match p::str(v, "tag") {
        Some(t) => t.to_string(),
        None => match v.get("source") {
            Some(_) => s.doc.sources.last().map(|x| x.tag.clone()).unwrap_or_default(),
            None => return Err(CmdError::Params("give `tag` or `source`".into())),
        },
    };
    let src = s.doc.sources.iter().find(|x| x.tag == tag).cloned().ok_or_else(|| CmdError::Params(format!("no source `{tag}`")))?;
    let pages = p::str(v, "pages").unwrap_or("").to_string();
    let st = style(&s.bib_style);
    let n = number_of(&s.doc, &bib_order(&s.doc, st, super::default_lang(s)), &tag);
    let props = s.typing_props();
    let at = delete_selection(s)?;
    let instr = if pages.is_empty() { format!("CITATION {tag}") } else { format!("CITATION {tag} \\p {pages}") };
    let end = s.doc.insert_object(&at, InlineObject::Field { instr, result: cite(&src, st.name, n, &pages), locked: false, code: false }, &props)?;
    s.sel = Selection::caret(end);
    // A new citation can renumber the others (numbered styles count in order of first citation).
    if st.numbered() {
        refresh_citations(s)?;
    }
    Ok(json!({"tag": tag}))
}

/// The tag and `\p` pages of a CITATION field instruction.
fn citation_field(instr: &str) -> Option<(&str, &str)> {
    let mut words = instr.split_whitespace();
    if words.next()? != "CITATION" {
        return None;
    }
    let tag = words.next()?;
    let pages = instr.split("\\p").nth(1).map(str::trim).unwrap_or("");
    Some((tag, pages))
}

/// Tags cited in the body, in order of first citation.
fn cited_tags(doc: &Document) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for path in doc.para_paths(StoryRef::Body) {
        let Some(p) = doc.para(StoryRef::Body, &path) else { continue };
        for o in &p.objects {
            if let InlineObject::Field { instr, .. } = o
                && let Some((tag, _)) = citation_field(instr)
                && seen.insert(tag.to_string())
            {
                out.push(tag.to_string());
            }
        }
    }
    out
}

fn author_key(src: &Source) -> String {
    authors(&src.author).first().map(|a| a.0.to_lowercase()).unwrap_or_else(|| src.title.to_lowercase())
}

/// Indices into `doc.sources` in bibliography order: the cited sources (every source while
/// nothing is cited), sorted or in order of first citation as the style asks. IEEE also
/// includes uncited document sources after the cited ones.
fn bib_order(doc: &Document, st: &CiteStyle, lang: Option<String>) -> Vec<usize> {
    let cited = cited_tags(doc);
    let mut idx: Vec<usize> = if cited.is_empty() {
        (0..doc.sources.len()).collect()
    } else if st.order == Order::Cited {
        cited.iter().filter_map(|t| doc.sources.iter().position(|x| &x.tag == t)).collect()
    } else {
        let cited: std::collections::HashSet<&str> = cited.iter().map(String::as_str).collect();
        doc.sources.iter().enumerate().filter(|(_, x)| cited.contains(x.tag.as_str())).map(|(i, _)| i).collect()
    };
    if st.name == "IEEE" {
        let included: std::collections::HashSet<usize> = idx.iter().copied().collect();
        idx.extend((0..doc.sources.len()).filter(|i| !included.contains(i)));
    }
    // Sorted in the document's language (Polish authors `Łukasiewicz` before `Mazur`).
    let key = |i: &usize, by_title: bool| {
        sort_key(&doc.sources.get(*i).map(|x| if by_title { x.title.clone() } else { author_key(x) }).unwrap_or_default(), lang.as_deref())
    };
    match st.order {
        Order::Author => idx.sort_by_cached_key(|i| key(i, false)),
        Order::Title => idx.sort_by_cached_key(|i| key(i, true)),
        Order::Cited => {}
    }
    idx
}

/// A source's number in a numbered style: its place in the bibliography order.
fn number_of(doc: &Document, order: &[usize], tag: &str) -> usize {
    order.iter().position(|&i| doc.sources.get(i).is_some_and(|x| x.tag == tag)).unwrap_or(order.len()).saturating_add(1)
}

/// Refresh CITATION results and generated lists.
pub fn update_citations(s: &mut Session) -> Result<(), CmdError> {
    refresh_citations(s)?;
    update_generated(s)
}

/// Refresh the results of the CITATION fields in the body.
fn refresh_citations(s: &mut Session) -> Result<(), CmdError> {
    let st = style(&s.bib_style);
    let order = bib_order(&s.doc, st, super::default_lang(s));
    for path in s.doc.para_paths(StoryRef::Body) {
        let Some(p) = s.doc.para(StoryRef::Body, &path) else { continue };
        let updates: Vec<(usize, String)> = p
            .objects
            .iter()
            .enumerate()
            .filter_map(|(k, o)| match o {
                InlineObject::Field { instr, .. } => {
                    let (tag, pages) = citation_field(instr)?;
                    let src = s.doc.sources.iter().find(|x| x.tag == tag)?;
                    Some((k, cite(src, st.name, number_of(&s.doc, &order, tag), pages)))
                }
                _ => None,
            })
            .collect();
        if updates.is_empty() {
            continue;
        }
        let para = s.doc.para_mut(StoryRef::Body, &path)?;
        for (k, r) in updates {
            if let Some(InlineObject::Field { result, .. }) = para.objects.get_mut(k) {
                *result = r;
            }
        }
        para.touch();
    }
    Ok(())
}

/// Generated lists (BIBLIOGRAPHY, INDEX, table of figures, TOA): a heading paragraph holding
/// the field, then entry paragraphs marked with the "GeneratedEntry" style family.
pub(crate) fn generated_list(s: &mut Session, instr: &str, title: &str) -> Result<(), CmdError> {
    let at = delete_selection(s)?;
    if at.story != StoryRef::Body || at.path.depth() > 0 {
        return Err(CmdError::Failed("lists go in the main text".into()));
    }
    let mut head = if title.is_empty() { Paragraph::new() } else { Paragraph::with_text(title, CharProps::default()).styled("Heading1") };
    let end = head.len();
    head.insert_object(
        end,
        InlineObject::Field { instr: instr.to_string(), result: String::new(), locked: false, code: false },
        &CharProps::default(),
    )?;
    let i = if s.doc.para_at(&at).is_some_and(|p| p.is_empty()) { at.path.last() } else { s.doc.split_paragraph(&at)?.path.last() };
    s.doc.insert_block(StoryRef::Body, &Path::top(i), Block::Para(head))?;
    update_generated(s)
}

fn list_kind(p: &Paragraph) -> Option<String> {
    p.objects.iter().find_map(|o| match o {
        InlineObject::Field { instr, .. } => {
            let t = instr.trim_start();
            (t.starts_with("BIBLIOGRAPHY") || t.starts_with("INDEX") || t.starts_with("TOA") || t.starts_with("TOC \\c")).then(|| t.to_string())
        }
        _ => None,
    })
}

const ENTRY_STYLE: &str = "ListEntry";
const BIBLIOGRAPHY_STYLE: &str = "Bibliografia";

fn generated_entry(text: String, hanging: bool, width: f32) -> Paragraph {
    let mut para = Paragraph::with_text(&text, CharProps::default()).styled(ENTRY_STYLE);
    if hanging {
        para.props.indent_left = Some(36.0);
        para.props.indent_first = Some(-36.0);
    }
    if text.contains('\t') {
        para.props.tabs = Some(vec![TabStop { pos: width - 0.5, align: TabAlign::Right, leader: TabLeader::Dot }]);
    }
    para
}

fn bibliography_instr(instr: &str) -> bool {
    instr.split_whitespace().next().is_some_and(|name| name.eq_ignore_ascii_case("BIBLIOGRAPHY"))
}

/// Project the native atomic heading field to a body range before generating a
/// table. Existing explicit ranges, nested fields and locked fields stay intact.
fn expand_native_ieee_bibliographies(s: &mut Session) -> Result<(), CmdError> {
    let ranges = s.doc.field_ranges(StoryRef::Body);
    let mut updates = Vec::new();
    for (i, block) in s.doc.body.iter().enumerate() {
        let Some(para) = block.as_para() else { continue };
        let fields: Vec<_> = para
            .object_offsets()
            .into_iter()
            .filter_map(|off| {
                let InlineObject::Field { instr, locked: false, .. } = para.object_at(off)? else { return None };
                let pos = Pos::new(StoryRef::Body, Path::top(i), off);
                (bibliography_instr(instr) && !ranges.iter().any(|range| range.start < pos && pos < range.end)).then(|| (off, instr.clone()))
            })
            .collect();
        if !fields.is_empty() {
            updates.push((i, fields));
        }
    }
    for (i, fields) in updates.into_iter().rev() {
        // Only the legacy native representation owns adjacent ListEntry blocks.
        let mut removed = 0usize;
        while s.doc.body.get(i + 1).and_then(|b| b.as_para()).is_some_and(|p| {
            p.props.style.as_deref() == Some(ENTRY_STYLE)
                && !p.objects.iter().any(|object| matches!(object, InlineObject::FieldStart { .. } | InlineObject::FieldEnd))
        }) {
            s.doc.remove_block(StoryRef::Body, &Path::top(i + 1))?;
            removed = removed.saturating_add(1);
        }
        for pos in [&mut s.sel.anchor, &mut s.sel.focus] {
            if pos.story != StoryRef::Body {
                continue;
            }
            let Some(top) = pos.path.0.first().copied().map(|index| index as usize) else { continue };
            if top > i.saturating_add(removed) {
                if let Some(index) = pos.path.0.first_mut() {
                    *index = u32::try_from(top.saturating_sub(removed)).unwrap_or(u32::MAX);
                }
            } else if top > i {
                // A selection in replaced legacy entries follows their owner.
                *pos = Pos::body(i, fields.first().map(|(off, _)| *off).unwrap_or(0));
            }
        }
        let para = s.doc.para_mut(StoryRef::Body, &Path::top(i))?;
        for (off, instr) in fields.into_iter().rev() {
            let props = para.props_of_char(off).clone();
            let end = off
                .checked_add(wordcraft_doc::para::OBJ.len_utf8())
                .ok_or_else(|| CmdError::Failed("bibliography field boundary is too large".into()))?;
            para.delete(off, end)?;
            para.insert_object(off, InlineObject::FieldEnd, &props)?;
            para.insert_object(off, InlineObject::FieldStart { instr, locked: false }, &props)?;
            // One atomic object becomes two markers in this paragraph.
            for pos in [&mut s.sel.anchor, &mut s.sel.focus] {
                if pos.story == StoryRef::Body && pos.path == Path::top(i) && pos.off >= end {
                    pos.off = pos.off.saturating_add(wordcraft_doc::para::OBJ.len_utf8());
                }
            }
        }
    }
    Ok(())
}

/// One generator for creation, refresh and style changes. Markers always stay
/// in body paragraphs, outside the table cells' independent block lists.
fn bibliography_blocks(s: &mut Session, instr: String, locked: bool, props: &CharProps) -> Result<Vec<Block>, CmdError> {
    let width = super::page::sect(s).text_width();
    let mut blocks = if style(&s.bib_style).name == "IEEE" {
        let order = bib_order(&s.doc, style(&s.bib_style), super::default_lang(s));
        if order.len() > wordcraft_doc::table::MAX_ROWS {
            return Err(CmdError::Failed("bibliography exceeds the table row limit".into()));
        }
        let mut rows = Vec::new();
        for (index, source_index) in order.into_iter().enumerate() {
            let Some(source) = s.doc.sources.get(source_index) else { continue };
            let number = index.saturating_add(1);
            let parts = match ieee_parts(source) {
                Some(parts) => parts,
                None => {
                    let full = entry(source, "IEEE", number);
                    let text = full.strip_prefix(&format!("[{number}] ")).unwrap_or(&full).to_string();
                    vec![(text, false)]
                }
            };
            let label =
                Paragraph::with_text(&format!("[{number}] "), CharProps { no_proof: Some(true), ..Default::default() }).styled(BIBLIOGRAPHY_STYLE);
            let mut content = Paragraph::new().styled(BIBLIOGRAPHY_STYLE);
            for (text, italic) in parts {
                content.insert_text(
                    content.len(),
                    &text,
                    &CharProps { no_proof: Some(true), italic: italic.then_some(true), italic_cs: italic.then_some(true), ..Default::default() },
                )?;
            }
            let make_cell = |para, width_pct| Cell {
                props: wordcraft_doc::props::CellProps { span: 1, width_pct, hide_mark: true, ..Default::default() },
                blocks: vec![wordcraft_doc::para_block(para)],
                ..Default::default()
            };
            rows.push(Row { cells: vec![make_cell(label, Some(1.0)), make_cell(content, None)], ..Default::default() });
        }
        let anchor_style = if rows.is_empty() { ENTRY_STYLE } else { "Normal" };
        let mut blocks = vec![Block::Para(Paragraph::new().styled(anchor_style))];
        if !rows.is_empty() {
            // Grid widths are layout hints. The measured 1%/auto cell preferences
            // and AutoFit determine the columns; no measured fixed width is copied.
            let width = if width.is_finite() { width.clamp(12.0, 1584.0) } else { 468.0 };
            let table = Table {
                props: wordcraft_doc::props::TableProps {
                    width_pct: Some(100.0),
                    cell_margins: Some([0.75; 4]),
                    cell_spacing: Some(0.75),
                    ..Default::default()
                },
                grid: vec![width * 0.01, width * 0.99],
                rows,
                ..Default::default()
            };
            blocks.push(Block::Table(table));
            blocks.push(Block::Para(Paragraph::new()));
        }
        blocks
    } else {
        entries_for(s, "BIBLIOGRAPHY").into_iter().map(|(text, hanging)| Block::Para(generated_entry(text, hanging, width))).collect()
    };
    if blocks.is_empty() {
        blocks.push(Block::Para(Paragraph::new().styled(ENTRY_STYLE)));
    }
    if let Some(Block::Para(first)) = blocks.first_mut() {
        first.insert_object(0, InlineObject::FieldStart { instr, locked }, props)?;
    }
    if let Some(Block::Para(last)) = blocks.last_mut() {
        last.insert_object(last.len(), InlineObject::FieldEnd, props)?;
    }
    Ok(blocks)
}

/// A reopened bibliography owns its result by explicit boundaries, not by a
/// nearby heading or paragraph style. Replace only that result; never add a title.
fn update_bibliography_ranges(s: &mut Session) -> Result<(), CmdError> {
    let ranges: Vec<_> = s
        .doc
        .field_ranges(StoryRef::Body)
        .into_iter()
        .filter(|range| {
            range.depth == 0
                && !range.locked
                && range.start.path.depth() == 0
                && range.end.path.depth() == 0
                && range.instr.split_whitespace().next().is_some_and(|name| name.eq_ignore_ascii_case("BIBLIOGRAPHY"))
        })
        .collect();
    for range in ranges.into_iter().rev() {
        let Some(mut before) = s.doc.para_at(&range.start).cloned() else { continue };
        let Some(mut after) = s.doc.para_at(&range.end).cloned() else { continue };
        let props = before.props_of_char(range.start.off).clone();
        let Some(end_off) = range.end.off.checked_add(wordcraft_doc::para::OBJ.len_utf8()) else {
            return Err(CmdError::Failed("bibliography field boundary is too large".into()));
        };
        let _old_result = before.split_off(range.start.off)?;
        after.delete(0, end_off)?;
        let ieee = style(&s.bib_style).name == "IEEE";
        let mut entries = bibliography_blocks(s, range.instr, range.locked, &props)?;
        let prefix_len = before.len();
        let separate_prefix = ieee && !before.is_empty();
        if ieee {
            if !before.is_empty() {
                entries.insert(0, Block::Para(before));
            }
        } else {
            if !before.is_empty()
                && let Some(Block::Para(first)) = entries.first_mut()
            {
                before.append(first.clone());
                *first = before;
            }
        }
        let (start, end) = (range.start.path.last(), range.end.path.last());
        if s.doc.body.get(start..=end).is_none() {
            return Err(CmdError::Failed("bibliography field has invalid block boundaries".into()));
        }
        // Remember where the regenerated boundaries and outside suffix land
        // before moving the blocks into the document.
        let new_start = Pos::body(start.saturating_add(usize::from(separate_prefix)), if ieee { 0 } else { prefix_len });
        let new_end_off =
            entries.last().and_then(Block::as_para).map(|para| para.len().saturating_sub(wordcraft_doc::para::OBJ.len_utf8())).unwrap_or(0);
        let new_end = Pos::body(start.saturating_add(entries.len().saturating_sub(1)), new_end_off);
        let suffix = if after.is_empty() {
            // An empty suffix can still own a section break. Appending it keeps
            // that section without adding an otherwise empty paragraph.
            if let Some(Block::Para(last)) = entries.last_mut() {
                last.append(after);
            }
            Pos::new(StoryRef::Body, new_end.path.clone(), new_end.off.saturating_add(wordcraft_doc::para::OBJ.len_utf8()))
        } else {
            // This content is outside the field in every citation style. Its
            // own paragraph keeps ParaProps, mark, runs, objects and section.
            let suffix = Pos::body(start.saturating_add(entries.len()), 0);
            entries.push(Block::Para(after));
            suffix
        };
        let removed = end.saturating_sub(start).saturating_add(1);
        let inserted = entries.len();
        let remap = |pos: &mut Pos| {
            if pos.story != StoryRef::Body {
                return;
            }
            let Some(top) = pos.path.0.first().copied().map(|index| index as usize) else { return };
            if top < start {
                return;
            }
            if top > end {
                // Preserve paths inside later tables as well as ordinary text.
                if let Some(index) = pos.path.0.first_mut() {
                    *index = u32::try_from(top.saturating_sub(removed).saturating_add(inserted)).unwrap_or(u32::MAX);
                }
            } else if pos.path == range.start.path && pos.off < range.start.off {
                *pos = Pos::body(start, pos.off);
            } else if pos.path == range.end.path && pos.off >= end_off {
                *pos = Pos::new(StoryRef::Body, suffix.path.clone(), suffix.off.saturating_add(pos.off.saturating_sub(end_off)));
            } else if *pos >= range.end {
                *pos = new_end.clone();
            } else if *pos <= range.start {
                *pos = new_start.clone();
            } else {
                // The old cached content was replaced. Keep an interior caret
                // at the new result boundary, never at an unrelated paragraph.
                *pos = Pos::new(StoryRef::Body, new_start.path.clone(), new_start.off.saturating_add(wordcraft_doc::para::OBJ.len_utf8()));
            }
        };
        s.doc.body.splice(start..=end, entries.into_iter().map(std::sync::Arc::new));
        remap(&mut s.sel.anchor);
        remap(&mut s.sel.focus);
    }
    Ok(())
}

/// Rebuild every generated list.
pub fn update_generated(s: &mut Session) -> Result<(), CmdError> {
    if s.doc.styles.get(ENTRY_STYLE).is_none() {
        s.doc.styles.upsert(wordcraft_doc::Style {
            id: ENTRY_STYLE.into(),
            name: "List Entry".into(),
            kind: wordcraft_doc::StyleKind::Paragraph,
            based_on: Some("Normal".into()),
            para: wordcraft_doc::ParaProps { space_after: Some(4.0), ..Default::default() },
            hidden: true,
            ..Default::default()
        });
    }
    if style(&s.bib_style).name == "IEEE" {
        if s.doc.styles.get(BIBLIOGRAPHY_STYLE).is_none() {
            s.doc.styles.upsert(wordcraft_doc::Style {
                id: BIBLIOGRAPHY_STYLE.into(),
                name: "Bibliografia".into(),
                based_on: Some("Normal".into()),
                builtin: true,
                ..Default::default()
            });
        }
        expand_native_ieee_bibliographies(s)?;
    }
    update_bibliography_ranges(s)?;
    let mut i = 0;
    while i < s.doc.body.len() {
        let kind = s.doc.body.get(i).and_then(|b| b.as_para()).and_then(list_kind);
        let Some(kind) = kind else {
            i += 1;
            continue;
        };
        if style(&s.bib_style).name == "IEEE" && bibliography_instr(&kind) {
            // Atomic bibliography fields left here are locked or nested inside
            // another range, which the range updater deliberately does not own.
            i += 1;
            continue;
        }
        if kind.starts_with("INDEX") {
            super::index::rebuild(s, i, None)?;
            i += 1;
            continue;
        }
        // Remove old entries.
        while s.doc.body.get(i + 1).and_then(|b| b.as_para()).is_some_and(|p| p.props.style.as_deref() == Some(ENTRY_STYLE)) {
            s.doc.remove_block(StoryRef::Body, &Path::top(i + 1))?;
        }
        let lines = entries_for(s, &kind);
        let width = super::page::sect(s).text_width();
        for (k, (text, hanging)) in lines.into_iter().enumerate() {
            let para = generated_entry(text, hanging, width);
            s.doc.insert_block(StoryRef::Body, &Path::top(i + 1 + k), Block::Para(para))?;
        }
        i += 1;
    }
    s.touch();
    Ok(())
}

fn page_of(s: &mut Session, pos: &Pos) -> u32 {
    let l = s.layout();
    l.caret(pos).and_then(|c| l.pages.get(c.page)).map(|p| p.number).unwrap_or(1)
}

/// Entry lines (text, hanging indent) for a generated list.
fn entries_for(s: &mut Session, kind: &str) -> Vec<(String, bool)> {
    if kind.starts_with("BIBLIOGRAPHY") {
        let st = style(&s.bib_style);
        return bib_order(&s.doc, st, super::default_lang(s))
            .into_iter()
            .enumerate()
            .filter_map(|(k, i)| s.doc.sources.get(i).map(|x| (entry(x, st.name, k.saturating_add(1)), !st.numbered())))
            .collect();
    }
    // Collect marks with positions.
    let mut marks: Vec<(String, Pos)> = Vec::new();
    let caption_label = kind.split('"').nth(1).unwrap_or("Figure").to_string();
    for path in s.doc.para_paths(StoryRef::Body) {
        let Some(p) = s.doc.para(StoryRef::Body, &path) else { continue };
        if kind.starts_with("TOC \\c") {
            if p.props.style.as_deref() == Some("Caption") && p.plain_text().starts_with(&caption_label) {
                marks.push((p.plain_text(), Pos { story: StoryRef::Body, path: path.clone(), off: 0 }));
            }
            continue;
        }
        for off in p.object_offsets() {
            if let Some(InlineObject::Field { instr, .. }) = p.object_at(off) {
                let t = instr.trim_start();
                let want = if kind.starts_with("INDEX") { "XE" } else { "TA" };
                if t.starts_with(want) {
                    let e = t.split('"').nth(1).unwrap_or("").to_string();
                    marks.push((e, Pos { story: StoryRef::Body, path: path.clone(), off }));
                }
            }
        }
    }
    let mut with_pages: Vec<(String, u32)> = marks.into_iter().map(|(e, _)| (e, 0)).collect();
    // Pages (layout once).
    let positions: Vec<Pos> = {
        let mut v = Vec::new();
        for path in s.doc.para_paths(StoryRef::Body) {
            let Some(p) = s.doc.para(StoryRef::Body, &path) else { continue };
            if kind.starts_with("TOC \\c") {
                if p.props.style.as_deref() == Some("Caption") && p.plain_text().starts_with(&caption_label) {
                    v.push(Pos { story: StoryRef::Body, path: path.clone(), off: 0 });
                }
                continue;
            }
            for off in p.object_offsets() {
                if let Some(InlineObject::Field { instr, .. }) = p.object_at(off) {
                    let want = if kind.starts_with("INDEX") { "XE" } else { "TA" };
                    if instr.trim_start().starts_with(want) {
                        v.push(Pos { story: StoryRef::Body, path: path.clone(), off });
                    }
                }
            }
        }
        v
    };
    for (i, pos) in positions.iter().enumerate() {
        let pg = page_of(s, pos);
        if let Some(x) = with_pages.get_mut(i) {
            x.1 = pg;
        }
    }
    if kind.starts_with("TOC \\c") {
        return with_pages.into_iter().map(|(t, p)| (format!("{t}\t{p}"), false)).collect();
    }
    // Index / TOA: group by entry, sort, list pages.
    let lang = super::default_lang(s);
    let mut grouped: std::collections::BTreeMap<(SortKey, String), Vec<u32>> = Default::default();
    for (e, p) in with_pages {
        let pages = grouped.entry((sort_key(&e, lang.as_deref()), e)).or_default();
        if !pages.contains(&p) {
            pages.push(p);
        }
    }
    let mut out = Vec::new();
    let mut letter = None;
    for ((_, e), pages) in grouped {
        let first = heading_letter(&e, lang.as_deref());
        if kind.starts_with("INDEX") && first != letter {
            letter = first.clone();
            if let Some(l) = first {
                out.push((l, false));
            }
        }
        let list = pages.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(", ");
        out.push(if kind.starts_with("INDEX") { (format!("{e}, {list}"), true) } else { (format!("{e}\t{list}"), false) });
    }
    out
}

fn cross_ref(s: &mut Session, v: &Value) -> CmdResult {
    let to = p::str(v, "to").unwrap_or("heading");
    let targets = ref_targets(s, to);
    let Some(target) = p::str(v, "target") else {
        return Ok(Value::Array(targets.iter().map(|(t, _)| json!(t)).collect()));
    };
    let find = |ts: Vec<(String, Pos)>| ts.into_iter().find(|(t, _)| t == target || t.starts_with(target));
    let (text, pos) = find(targets).ok_or_else(|| CmdError::Params(format!("no {to} `{target}`")))?;
    let show = p::str(v, "show").unwrap_or("text");
    let page = page_of(s, &pos);
    let result = match show {
        "page" => page.to_string(),
        "number" => text.split(':').next().unwrap_or(&text).trim().to_string(),
        "aboveBelow" => {
            if pos < s.sel.focus {
                "above".into()
            } else {
                "below".into()
            }
        }
        _ if to == "bookmark" => super::references::bookmark_text(&s.doc, &text).unwrap_or_default(),
        _ => text.clone(),
    };
    let props = s.typing_props();
    let at = delete_selection(s)?;
    // The field points at a bookmark: the named one, or a hidden `_Ref` bookmark on the target
    // paragraph (as Word does), so Update Field and Word can resolve it.
    let (name, at) = if to == "bookmark" {
        (text, at)
    } else {
        // Deleting the selection can move paragraphs: look the target up again.
        let (_, pos) = find(ref_targets(s, to)).ok_or_else(|| CmdError::Params(format!("no {to} `{target}`")))?;
        ref_bookmark(s, &pos, show == "number", at)?
    };
    let instr = format!("{} {name} \\h", if show == "page" { "PAGEREF" } else { "REF" });
    let end = s.doc.insert_object(&at, InlineObject::Field { instr, result: result.clone(), locked: false, code: false }, &props)?;
    s.sel = Selection::caret(end);
    Ok(json!({"inserted": result}))
}

/// Cross-reference targets of a kind: (shown text or bookmark name, position).
fn ref_targets(s: &Session, to: &str) -> Vec<(String, Pos)> {
    let mut targets: Vec<(String, Pos)> = Vec::new();
    match to {
        "bookmark" => targets = s.doc.bookmarks(),
        "figure" | "table" | "equation" => {
            let label = match to {
                "table" => "Table",
                "equation" => "Equation",
                _ => "Figure",
            };
            for path in s.doc.para_paths(StoryRef::Body) {
                if let Some(p) = s.doc.para(StoryRef::Body, &path)
                    && p.props.style.as_deref() == Some("Caption")
                    && p.plain_text().starts_with(label)
                {
                    targets.push((p.plain_text(), Pos { story: StoryRef::Body, path: path.clone(), off: 0 }));
                }
            }
        }
        _ => {
            for path in s.doc.para_paths(StoryRef::Body) {
                if let Some(p) = s.doc.para(StoryRef::Body, &path)
                    && s.doc.styles.resolve_para(&p.props).outline_level.is_some()
                    && !p.plain_text().trim().is_empty()
                {
                    targets.push((p.plain_text().trim().to_string(), Pos { story: StoryRef::Body, path: path.clone(), off: 0 }));
                }
            }
        }
    }
    targets
}

/// The hidden `_Ref` bookmark around the target paragraph at `pos` (its label and number only
/// when `number_only`), reusing one already there. Returns its name and `at` moved past the
/// bookmark marks inserted before it.
fn ref_bookmark(s: &mut Session, pos: &Pos, number_only: bool, mut at: Pos) -> Result<(String, Pos), CmdError> {
    const MARK: usize = wordcraft_doc::para::OBJ.len_utf8();
    let para = s.doc.para(pos.story, &pos.path).ok_or_else(|| CmdError::Params("cross-reference target is gone".into()))?;
    let end = if number_only { para.text.find(':').unwrap_or(para.len()) } else { para.len() };
    if let Some(InlineObject::BookmarkStart { name }) = para.object_at(0)
        && name.starts_with("_Ref")
    {
        // Reuse it when it covers the same extent: up to the colon, or the whole paragraph.
        let same = para.object_offsets().into_iter().any(|o| {
            matches!(para.object_at(o), Some(InlineObject::BookmarkEnd { name: n }) if n == name)
                && para.text.get(o.saturating_add(MARK)..).is_some_and(|rest| if number_only { rest.starts_with(':') } else { rest.is_empty() })
        });
        if same {
            return Ok((name.clone(), at));
        }
    }
    let next =
        s.doc.bookmarks().iter().filter_map(|(n, _)| n.strip_prefix("_Ref").and_then(|d| d.parse::<u64>().ok())).max().unwrap_or(0).saturating_add(1);
    let name = format!("_Ref{next:09}");
    let props = CharProps::default();
    s.doc.insert_object(&Pos { off: end, ..pos.clone() }, InlineObject::BookmarkEnd { name: name.clone() }, &props)?;
    s.doc.insert_object(&Pos { off: 0, ..pos.clone() }, InlineObject::BookmarkStart { name: name.clone() }, &props)?;
    if at.story == pos.story && at.path == pos.path {
        if at.off >= end {
            at.off = at.off.saturating_add(2 * MARK);
        } else if at.off > 0 {
            at.off = at.off.saturating_add(MARK);
        }
    }
    Ok((name, at))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Session;

    #[test]
    fn citations_and_bibliography() {
        let mut s = Session::new(wordcraft_doc::Document::new());
        s.run("text.insert", &json!({"text": "Studios matter "})).unwrap();
        s.run("references.citation", &json!({"source": {"author": "Rivera, Alex; Chen, Mei", "title": "Shared Spaces", "year": "2021", "publisher": "Harbor Books", "city": "Portland"}})).unwrap();
        assert!(s.doc.plain_text(StoryRef::Body).contains("(Rivera & Chen, 2021)"), "{}", s.doc.plain_text(StoryRef::Body));
        s.run("text.newParagraph", &json!({})).unwrap();
        s.run("references.bibliography", &json!({})).unwrap();
        let t = s.doc.plain_text(StoryRef::Body);
        assert!(t.contains("Rivera, A., & Chen, M. (2021). Shared Spaces. Harbor Books."), "{t}");
        s.run("references.citationStyle", &json!({"style": "MLA"})).unwrap();
        let t = s.doc.plain_text(StoryRef::Body);
        assert!(t.contains("(Rivera and Chen)"), "{t}");
        s.run("references.citationStyle", &json!({"style": "IEEE"})).unwrap();
        assert!(s.doc.plain_text(StoryRef::Body).contains("[1]"));
    }

    #[test]
    fn bibliography_round_trip_docx() {
        let mut s = Session::new(wordcraft_doc::Document::new());
        s.run("text.insert", &json!({"text": "Studios matter "})).unwrap();
        s.run(
            "references.citation",
            &json!({
                "source": {
                    "author": "Rivera, Alex; Chen, Mei",
                    "title": "Shared Spaces",
                    "year": "2021",
                    "publisher": "Harbor Books",
                    "city": "Portland",
                    "editor": "Johnson, Robert",
                    "edition": "2nd",
                    "issue": "4",
                    "month": "10",
                    "day": "15",
                    "accessed": "2024-05-15",
                    "doi": "10.1000/182"
                }
            }),
        )
        .unwrap();
        s.run("text.newParagraph", &json!({})).unwrap();
        s.run("references.bibliography", &json!({})).unwrap();

        // Serialize to DOCX and back — sources must survive.
        let bytes = wordcraft_docx::write(&s.doc).expect("docx write");
        let doc = wordcraft_docx::read(&bytes).expect("docx read");
        assert_eq!(doc.sources.len(), 1, "round-tripped sources: {:?}", doc.sources);
        assert_eq!(doc.sources[0].author, "Rivera, Alex; Chen, Mei");
        assert_eq!(doc.sources[0].title, "Shared Spaces");
        assert_eq!(doc.sources[0].editor, "Johnson, Robert");
        assert_eq!(doc.sources[0].edition, "2nd");
        assert_eq!(doc.sources[0].issue, "4");
        assert_eq!(doc.sources[0].month, "10");
        assert_eq!(doc.sources[0].day, "15");
        assert_eq!(doc.sources[0].accessed, "2024-05-15");
        assert_eq!(doc.sources[0].doi, "10.1000/182");

        // Round-tripped document in a new session works with citation styles.
        let mut s2 = Session::new(doc);
        s2.run("references.citationStyle", &json!({"style": "Harvard (Anglia)"})).unwrap();
        let t = s2.doc.plain_text(StoryRef::Body);
        assert!(t.contains("(Rivera and Chen, 2021)"), "{t}");
        assert!(t.contains("Rivera, A. and Chen, M., 2021. Shared Spaces. Portland: Harbor Books."), "{t}");
    }

    #[test]
    fn bibliography_reopen_update_and_reexport_keep_one_title_and_result() {
        let mut s = Session::new(Document::new());
        two_sources(&mut s, "Shared Spaces");
        s.run("references.bibliography", &json!({"title": "Unique bibliography heading"})).unwrap();
        s.doc.body.push(wordcraft_doc::para_block(Paragraph::with_text("After the bibliography.", CharProps::default())));
        let bytes = wordcraft_docx::write(&s.doc).unwrap();
        let mut s = Session::new(wordcraft_docx::read(&bytes).unwrap());
        let blocks = s.doc.body.len();
        s.doc.sources.iter_mut().find(|source| source.tag == "A").unwrap().title = "Updated shared spaces".into();
        for _ in 0..3 {
            s.run("references.updateFields", &json!({})).unwrap();
            let text = s.doc.plain_text(StoryRef::Body);
            assert_eq!(text.matches("Unique bibliography heading").count(), 1, "{text}");
            assert_eq!(text.matches("Updated shared spaces").count(), 1, "{text}");
            assert_eq!(text.matches("Kilns and Clay").count(), 1, "{text}");
            assert_eq!(text.matches("After the bibliography.").count(), 1, "{text}");
            assert!(!text.contains("Shared Spaces"), "the old cached result must be replaced: {text}");
            assert_eq!(s.doc.body.len(), blocks, "updating must not add a title or an empty boundary paragraph");
            let ranges = s.doc.field_ranges(StoryRef::Body);
            assert_eq!(ranges.len(), 1);
            assert_eq!(ranges[0].instr, "BIBLIOGRAPHY");
            s = Session::new(wordcraft_docx::read(&wordcraft_docx::write(&s.doc).unwrap()).unwrap());
        }
    }

    #[test]
    fn empty_bibliography_reopen_can_gain_entries_without_merging_its_title() {
        let mut s = Session::new(Document::new());
        s.run("references.bibliography", &json!({"title": "Initially empty bibliography"})).unwrap();
        let mut s = Session::new(wordcraft_docx::read(&wordcraft_docx::write(&s.doc).unwrap()).unwrap());
        two_sources(&mut s, "Shared Spaces");
        s.run("references.updateFields", &json!({})).unwrap();
        let heading = s.doc.body.iter().filter_map(|block| block.as_para()).find(|p| p.plain_text() == "Initially empty bibliography");
        assert!(heading.is_some(), "the title remains its own paragraph");
        let doc = wordcraft_docx::read(&wordcraft_docx::write(&s.doc).unwrap()).unwrap();
        assert_eq!(doc.field_ranges(StoryRef::Body).len(), 1);
        let text = doc.plain_text(StoryRef::Body);
        assert_eq!(text.matches("Initially empty bibliography").count(), 1);
        assert_eq!(text.matches("Shared Spaces").count(), 1);
        assert_eq!(text.matches("Kilns and Clay").count(), 1);
    }

    #[test]
    fn citation_style_survives_docx_save_and_reopen() {
        let mut s = Session::new(wordcraft_doc::Document::new());
        s.run("references.citation", &json!({"source": {"author": "Rivera, Alex", "title": "Shared Spaces", "year": "2021"}})).unwrap();
        s.run("references.citationStyle", &json!({"style": "Harvard (Anglia)"})).unwrap();
        let bytes = wordcraft_docx::write(&s.doc).expect("docx write");
        let doc = wordcraft_docx::read(&bytes).expect("docx read");
        assert_eq!(doc.settings.bib_style.as_deref(), Some("Harvard (Anglia)"));
        assert_eq!(Session::new(doc.clone()).bib_style, "Harvard (Anglia)");
        let mut other = Session::new(wordcraft_doc::Document::new());
        other.set_document(doc);
        assert_eq!(other.bib_style, "Harvard (Anglia)");
        // A document without a recorded style is APA.
        assert_eq!(Session::new(wordcraft_doc::Document::new()).bib_style, "APA");
    }

    #[test]
    fn index_and_figures() {
        let mut s = Session::new(wordcraft_doc::Document::from_text("Kilns are hot.\nPresses are heavy.\nMore kilns."));
        s.run("select.text", &json!({"text": "Kilns"})).unwrap();
        s.run("references.markEntry", &json!({})).unwrap();
        s.run("select.text", &json!({"text": "Presses"})).unwrap();
        s.run("references.markEntry", &json!({})).unwrap();
        s.run("caret.docEnd", &json!({})).unwrap();
        s.run("text.newParagraph", &json!({})).unwrap();
        s.run("references.index", &json!({})).unwrap();
        let t = s.doc.plain_text(StoryRef::Body);
        assert!(t.contains("Kilns, 1") && t.contains("Presses, 1"), "{t}");
        s.run("caret.docStart", &json!({})).unwrap();
        s.run("references.caption", &json!({"label": "Figure", "text": "The kiln"})).unwrap();
        s.run("caret.docEnd", &json!({})).unwrap();
        s.run("references.tableOfFigures", &json!({})).unwrap();
        assert!(s.doc.plain_text(StoryRef::Body).contains("Figure 1: The kiln\t1"));
        let list = s.run("insert.crossReference", &json!({"to": "figure"})).unwrap();
        assert_eq!(list.as_array().unwrap().len(), 1);
        s.run("insert.crossReference", &json!({"to": "figure", "target": "Figure 1", "show": "number"})).unwrap();
        assert!(s.doc.plain_text(StoryRef::Body).contains("Figure 1"));
        // The number-only reference marks just "Figure 1" of the caption.
        let (name, _) = s.doc.bookmarks().into_iter().find(|(n, _)| n.starts_with("_Ref")).unwrap();
        assert_eq!(crate::cmd::references::bookmark_text(&s.doc, &name).as_deref(), Some("Figure 1"));
        assert!(fields(&s).contains(&(format!("REF {name} \\h"), "Figure 1".to_string())), "{:?}", fields(&s));
    }

    /// #661: References › Table of Authorities › Update Table picks up citations marked after the table.
    #[test]
    fn table_of_authorities_updates() {
        let mut s = Session::new(wordcraft_doc::Document::from_text("Smith v. Jones held it.\nDoe v. Roe agreed."));
        s.run("select.text", &json!({"text": "Smith v. Jones"})).unwrap();
        s.run("references.markCitation", &json!({})).unwrap();
        s.run("caret.docEnd", &json!({})).unwrap();
        s.run("text.newParagraph", &json!({})).unwrap();
        s.run("references.tableOfAuthorities", &json!({})).unwrap();
        let t = s.doc.plain_text(StoryRef::Body);
        assert!(t.contains("Smith v. Jones") && !t.contains("Doe v. Roe, "), "{t}");
        s.run("select.text", &json!({"text": "Doe v. Roe"})).unwrap();
        s.run("references.markCitation", &json!({})).unwrap();
        s.run("references.updateAuthorities", &json!({})).unwrap();
        let after = s.doc.plain_text(StoryRef::Body);
        assert!(after.matches("Doe v. Roe").count() > t.matches("Doe v. Roe").count(), "{after}");
    }

    /// Field instructions and results in the body, in order.
    fn fields(s: &Session) -> Vec<(String, String)> {
        let mut out = Vec::new();
        for path in s.doc.para_paths(StoryRef::Body) {
            for o in &s.doc.para(StoryRef::Body, &path).unwrap().objects {
                if let InlineObject::Field { instr, result, .. } = o {
                    out.push((instr.clone(), result.clone()));
                }
            }
        }
        out
    }

    #[test]
    fn cross_references_point_at_real_bookmarks() {
        let mut s = Session::new(wordcraft_doc::Document::from_text("Kilns\nSee here."));
        s.run("caret.docStart", &json!({})).unwrap();
        s.run("styles.apply", &json!({"style": "Heading 1"})).unwrap();
        s.run("caret.docEnd", &json!({})).unwrap();
        s.run("insert.crossReference", &json!({"to": "heading", "target": "Kilns"})).unwrap();
        // The heading gets a hidden bookmark, and the REF field names it (not a literal `_Ref`).
        let marks = s.doc.bookmarks();
        assert_eq!(marks.len(), 1, "{marks:?}");
        let name = marks[0].0.clone();
        assert!(name.starts_with("_Ref") && name.len() > 4, "{name}");
        assert_eq!(marks[0].1.path, s.doc.para_paths(StoryRef::Body)[0]);
        assert_eq!(fields(&s), vec![(format!("REF {name} \\h"), "Kilns".to_string())]);
        // A second reference to the same heading reuses the bookmark.
        s.run("insert.crossReference", &json!({"to": "heading", "target": "Kilns", "show": "page"})).unwrap();
        assert_eq!(s.doc.bookmarks().len(), 1);
        assert_eq!(fields(&s)[1], (format!("PAGEREF {name} \\h"), "1".to_string()));
        // Update Field follows the heading's new text.
        s.run("select.text", &json!({"text": "ilns"})).unwrap();
        s.run("text.insert", &json!({"text": "ILNS"})).unwrap();
        s.run("references.updateFields", &json!({})).unwrap();
        assert_eq!(fields(&s)[0].1, "KILNS");
        assert!(s.doc.plain_text(StoryRef::Body).ends_with("See here.KILNS1"), "{}", s.doc.plain_text(StoryRef::Body));
    }

    #[test]
    fn cross_reference_to_a_bookmark_shows_its_text() {
        let mut s = Session::new(wordcraft_doc::Document::from_text("Kilns are hot.\nSee "));
        s.run("select.text", &json!({"text": "Kilns"})).unwrap();
        s.run("insert.bookmark", &json!({"name": "Spot"})).unwrap();
        s.run("caret.docEnd", &json!({})).unwrap();
        s.run("insert.crossReference", &json!({"to": "bookmark", "target": "Spot"})).unwrap();
        assert_eq!(s.doc.bookmarks().len(), 1, "no extra bookmark for a named one");
        assert_eq!(fields(&s), vec![("REF Spot \\h".to_string(), "Kilns".to_string())]);
    }

    #[test]
    fn cross_reference_inside_its_own_target_paragraph() {
        let mut s = Session::new(wordcraft_doc::Document::from_text("Kilns"));
        s.run("styles.apply", &json!({"style": "Heading 1"})).unwrap();
        s.run("caret.docEnd", &json!({})).unwrap();
        s.run("insert.crossReference", &json!({"to": "heading", "target": "Kilns"})).unwrap();
        assert_eq!(s.doc.plain_text(StoryRef::Body), "KilnsKilns");
        // The field sits after the bookmark, so it doesn't mark itself.
        let name = s.doc.bookmarks()[0].0.clone();
        assert_eq!(crate::cmd::references::bookmark_text(&s.doc, &name).as_deref(), Some("Kilns"));
    }

    fn two_sources(s: &mut Session, a_title: &str) {
        let a = json!({"tag": "A", "author": "Rivera, Alex; Chen, Mei", "title": a_title, "year": "2021", "publisher": "Harbor Books", "city": "Portland"});
        let b = json!({"tag": "B", "author": "Okafor, Ngozi", "title": "Kilns and Clay", "year": "2019", "journal": "Studio Quarterly", "volume": "4", "pages": "10-20"});
        s.run("references.sources", &json!({"add": a})).unwrap();
        s.run("references.sources", &json!({"add": b})).unwrap();
    }

    #[test]
    fn author_date_styles_cite_and_list() {
        let mut s = Session::new(wordcraft_doc::Document::new());
        two_sources(&mut s, "Shared Spaces");
        s.run("references.citation", &json!({"tag": "A", "pages": "12"})).unwrap();
        s.run("references.citation", &json!({"tag": "B"})).unwrap();
        s.run("text.newParagraph", &json!({})).unwrap();
        s.run("references.bibliography", &json!({})).unwrap();
        let cases = [
            (
                "Harvard (Anglia)",
                "(Rivera and Chen, 2021, p. 12)(Okafor, 2019)",
                "Okafor, N., 2019. Kilns and Clay. Studio Quarterly, 4, pp. 10-20.\nRivera, A. and Chen, M., 2021. Shared Spaces. Portland: Harbor Books.",
            ),
            (
                "ISO 690 - First Element and Date",
                "(Rivera and Chen 2021, p. 12)(Okafor 2019)",
                "OKAFOR, Ngozi, 2019. Kilns and Clay. Studio Quarterly, 4, 10-20.\nRIVERA, Alex and CHEN, Mei, 2021. Shared Spaces. Portland: Harbor Books.",
            ),
            (
                "turabian",
                "(Rivera and Chen 2021, 12)(Okafor 2019)",
                "Okafor, Ngozi. “Kilns and Clay.” Studio Quarterly 4 (2019): 10-20.\nRivera, Alex, and Mei Chen. Shared Spaces. Portland: Harbor Books, 2021.",
            ),
        ];
        for (style, cites, list) in cases {
            s.run("references.citationStyle", &json!({"style": style})).unwrap();
            let t = s.doc.plain_text(StoryRef::Body);
            assert!(t.starts_with(cites) && t.contains(list), "{style}: {t}");
        }
        assert!(s.run("references.citationStyle", &json!({"style": "Vancouver"})).unwrap_err().to_string().contains("Turabian"));
        assert_eq!(s.bib_style, "Turabian", "an unknown style keeps the current one");
    }

    #[test]
    fn numbered_styles_count_in_order_of_first_citation() {
        let mut s = Session::new(wordcraft_doc::Document::new());
        two_sources(&mut s, "Shared Spaces");
        s.run("references.citationStyle", &json!({"style": "ISO 690 (numerical)"})).unwrap();
        s.run("text.insert", &json!({"text": "Later "})).unwrap();
        s.run("references.citation", &json!({"tag": "A", "pages": "12"})).unwrap();
        assert_eq!(fields(&s)[0].1, "(1, p. 12)");
        // Citing B earlier in the text makes it number 1 and renumbers A.
        s.run("caret.docStart", &json!({})).unwrap();
        s.run("references.citation", &json!({"tag": "B"})).unwrap();
        s.run("caret.docEnd", &json!({})).unwrap();
        s.run("text.newParagraph", &json!({})).unwrap();
        s.run("references.bibliography", &json!({})).unwrap();
        let cases = [
            (
                "ISO 690 (numerical)",
                "(1)Later (2, p. 12)",
                "1. OKAFOR, Ngozi. Kilns and Clay. Studio Quarterly. 2019, 4, 10-20.\n2. RIVERA, Alex and CHEN, Mei. Shared Spaces. Portland: Harbor Books, 2021.",
            ),
            (
                "GB/T 7714",
                "[1]Later [2, 12]",
                "[1] OKAFOR N. Kilns and Clay[J]. Studio Quarterly, 2019, 4: 10-20.\n[2] RIVERA A, CHEN M. Shared Spaces[M]. Portland: Harbor Books, 2021.",
            ),
            (
                "SIST02",
                "(1)Later (2, p. 12)",
                "1) Okafor, Ngozi. Kilns and Clay. Studio Quarterly. 2019, vol. 4, p. 10-20.\n2) Rivera, Alex; Chen, Mei. Shared Spaces. Portland, Harbor Books, 2021.",
            ),
            (
                "IEEE",
                "[1]Later [2, p. 12]",
                // Word008's SourceType contrasts establish that kind, rather
                // than a filled journal, selects Book/JournalArticle. Both
                // sources in this older fixture have empty kind (OOXML Book).
                "[1] N. Okafor, Kilns and Clay, vol. 4, 2019, pp. 10-20.\n[2] A. Rivera and M. Chen, Shared Spaces, Portland: Harbor Books, 2021.",
            ),
        ];
        for (style, cites, list) in cases {
            s.run("references.citationStyle", &json!({"style": style})).unwrap();
            let t = s.doc.plain_text(StoryRef::Body);
            if style == "IEEE" {
                // The measured two-cell table adds cell/row whitespace. Keep
                // every content and punctuation character in the expectation.
                let compact = |text: &str| text.chars().filter(|c| !c.is_whitespace()).collect::<String>();
                assert!(t.starts_with(cites) && compact(&t).contains(&compact(list)), "{style}: {t}");
            } else {
                assert!(t.starts_with(cites) && t.contains(list), "{style}: {t}");
            }
        }
    }

    #[test]
    fn gost_numbers_follow_its_name_or_title_sort() {
        let mut s = Session::new(wordcraft_doc::Document::new());
        two_sources(&mut s, "Atlas of Studios");
        s.run("references.citation", &json!({"tag": "A"})).unwrap();
        s.run("references.citation", &json!({"tag": "B", "pages": "15"})).unwrap();
        s.run("text.newParagraph", &json!({})).unwrap();
        s.run("references.bibliography", &json!({})).unwrap();
        let okafor = "Okafor N. Kilns and Clay // Studio Quarterly. – 2019. – Vol. 4. – P. 10-20.";
        let rivera = "Rivera A., Chen M. Atlas of Studios. – Portland : Harbor Books, 2021.";
        // Word's style and style-sheet names pick the same styles.
        s.run("references.citationStyle", &json!({"style": "\\GostName.XSL"})).unwrap();
        assert_eq!(s.bib_style, "GOST (name sort)");
        let t = s.doc.plain_text(StoryRef::Body);
        assert!(t.starts_with("[2][1, p. 15]") && t.contains(&format!("1. {okafor}\n2. {rivera}")), "{t}");
        s.run("references.citationStyle", &json!({"style": "GOST - Title Sort"})).unwrap();
        let t = s.doc.plain_text(StoryRef::Body);
        assert!(t.starts_with("[1][2, p. 15]") && t.contains(&format!("1. {rivera}\n2. {okafor}")), "{t}");
        assert_eq!(find_style("ISO 690 - Numerical Reference").map(|x| x.name), Some("ISO 690 (numerical)"));
        assert_eq!(find_style("Harvard - Anglia 2008").map(|x| x.name), Some("Harvard (Anglia)"));
        assert!(find_style("").is_none() && find_style("Vancouver").is_none());
    }
}
