//! Fixed-height line-box baselines from the existing synthetic Word controls.
//! Only Auto/Baseline are repaired in this batch; other alignments keep their
//! previous behavior until their separately observed policies are implemented.

use std::sync::Arc;

use serde_json::json;
use wordcraft_doc::props::{CharProps, LineSpacing, ParaProps};
use wordcraft_doc::{Document, Paragraph, Path, StoryRef, Style, para_block};
use wordcraft_layout::{DocLayout, LayoutCache, LayoutOptions, Placed, layout};

fn alignment(props: &ParaProps, value: Option<&str>) -> ParaProps {
    // This public JSON seam compiles before TextAlignment is added to L1.
    let mut serialized = serde_json::to_value(props).unwrap();
    if let Some(value) = value {
        serialized["textAlignment"] = json!(value);
    } else {
        serialized.as_object_mut().unwrap().remove("textAlignment");
    }
    serde_json::from_value(serialized).unwrap()
}

fn document(spacing: LineSpacing, size: f32, mode: Option<&str>, flag: Option<bool>, terminal_break: bool) -> Document {
    let mut document = Document::new();
    let chr = CharProps { font: Some("Calibri".into()), size: Some(size), ..Default::default() };
    let mut paragraph = Paragraph::with_text(if terminal_break { "SPACING-A1\n" } else { "SPACING-A1\nSPACING-A1" }, chr.clone());
    paragraph.mark = chr;
    paragraph.props.line_spacing = Some(spacing);
    paragraph.props.space_before = Some(0.0);
    paragraph.props.space_after = Some(0.0);
    paragraph.props = alignment(&paragraph.props, mode);
    document.body = vec![para_block(paragraph)];
    document.settings.no_extra_line_spacing = flag;
    document
}

fn paragraph(result: &DocLayout) -> Arc<wordcraft_layout::para::ParaLayout> {
    assert_eq!(result.pages.len(), 1);
    result.pages[0]
        .items
        .iter()
        .find_map(|item| match item {
            Placed::Lines { path, para, .. } if path.0.as_slice() == [0] => Some(para.clone()),
            _ => None,
        })
        .unwrap()
}

fn measured(document: &Document) -> Arc<wordcraft_layout::para::ParaLayout> {
    paragraph(&layout(document, &mut LayoutCache::new(), &LayoutOptions::default()))
}

fn close_contract(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 0.0001, "actual={actual}, expected={expected}");
}

fn close_word(actual: f32, expected: f32) {
    // Observation precision only: no offset or fitted constant is applied.
    assert!((actual - expected).abs() <= 0.04, "Word offset={expected}, native offset={actual}");
}

fn word_exact_case(height: f32, size: f32, word_offset: f32) {
    for mode in [None, Some("auto"), Some("baseline")] {
        for flag in [None, Some(false)] {
            let result = measured(&document(LineSpacing::Exactly(height), size, mode, flag, false));
            assert_eq!(result.lines.len(), 2);
            for (index, line) in result.lines.iter().enumerate() {
                close_contract(line.height, height);
                close_contract(line.top, height * index as f32);
                close_word(line.baseline - line.top, word_offset);
            }
        }
    }
}

#[test]
fn default_exact8_calibri12_matches_the_measured_word_line_box() {
    // Body top72; first baseline78.4 in the already captured absent/false case.
    word_exact_case(8.0, 12.0, 6.4);
}

#[test]
fn default_exact16_calibri12_matches_the_measured_word_line_box() {
    // Existing table top72 plus padding2; first baseline86.8.
    word_exact_case(16.0, 12.0, 12.8);
}

#[test]
fn default_exact28_calibri12_matches_the_measured_word_line_box() {
    word_exact_case(28.0, 12.0, 22.42);
}

#[test]
fn default_exact24_calibri8_keeps_the_measured_font_independent_baseline() {
    word_exact_case(24.0, 8.0, 19.22);
}

#[test]
fn default_exact24_calibri16_keeps_the_measured_font_independent_baseline() {
    word_exact_case(24.0, 16.0, 19.22);
}

#[test]
fn unsupported_top_center_bottom_keep_the_previous_exact_policy() {
    // These explicitly aligned policies have separate Word differences. Do not
    // accidentally apply the Auto/Baseline repair to those still unsupported.
    let legacy = measured(&document(LineSpacing::AtLeast(24.0), 12.0, None, None, false));
    for mode in ["top", "center", "bottom"] {
        let exact = measured(&document(LineSpacing::Exactly(24.0), 12.0, Some(mode), None, false));
        for (line, previous) in exact.lines.iter().zip(&legacy.lines) {
            close_contract(line.height, previous.height);
            close_contract(line.baseline - line.top, previous.baseline - previous.top);
            assert_eq!(line.xs, previous.xs);
        }
    }
}

#[test]
fn explicit_no_extra_keeps_the_existing_font_dependent_and_short_line_policy() {
    for (height, size) in [(24.0, 8.0), (24.0, 12.0), (24.0, 16.0), (8.0, 12.0)] {
        let natural = measured(&document(LineSpacing::Multiple(1.0), size, None, None, false));
        let exact = measured(&document(LineSpacing::Exactly(height), size, Some("baseline"), Some(true), false));
        for line in &exact.lines {
            close_contract(line.height, height);
            close_contract(line.baseline - line.top, natural.lines[0].baseline.min(height));
        }
    }
}

#[test]
fn single_double_and_at_least_are_unchanged_by_the_new_alignment_gate() {
    for spacing in [LineSpacing::Multiple(1.0), LineSpacing::Multiple(2.0), LineSpacing::AtLeast(24.0)] {
        let previous = measured(&document(spacing, 12.0, Some("top"), None, false));
        for mode in [Some("auto"), Some("baseline")] {
            for flag in [None, Some(true)] {
                let current = measured(&document(spacing, 12.0, mode, flag, false));
                for (line, old) in current.lines.iter().zip(&previous.lines) {
                    close_contract(line.height, old.height);
                    close_contract(line.top, old.top);
                    close_contract(line.baseline, old.baseline);
                    assert_eq!(line.xs, old.xs);
                }
            }
        }
    }
}

#[test]
fn terminal_empty_line_shares_the_measured_default_exact_line_box_policy() {
    use wordcraft_doc::section::{DocGrid, DocGridType};
    for grid in [None, Some(DocGrid { kind: DocGridType::Lines, line_pitch: 36.0, ..Default::default() })] {
        let mut doc = document(LineSpacing::Exactly(24.0), 12.0, Some("baseline"), None, true);
        doc.para_mut(StoryRef::Body, &Path::top(0)).unwrap().props.snap_to_grid = Some(true);
        doc.last_section.doc_grid = grid;
        let result = measured(&doc);
        assert_eq!(result.lines.len(), 2, "preserve the terminal empty line");
        close_word(result.lines[0].baseline - result.lines[0].top, 19.22);
        for line in &result.lines {
            close_contract(line.height, 24.0);
            close_contract(line.baseline - line.top, result.lines[0].baseline - result.lines[0].top);
        }
    }
}

#[test]
fn style_inheritance_explicit_auto_and_warm_cache_use_the_effective_alignment() {
    let mut doc = document(LineSpacing::Exactly(24.0), 12.0, None, None, false);
    let base = |mode: &str| Style {
        id: "ExactLineBoxBase".into(),
        name: "Synthetic exact line box base".into(),
        para: alignment(&ParaProps::default(), Some(mode)),
        ..Default::default()
    };
    doc.styles.upsert(base("top"));
    doc.styles.upsert(Style {
        id: "ExactLineBoxChild".into(),
        name: "Synthetic exact line box child".into(),
        based_on: Some("ExactLineBoxBase".into()),
        ..Default::default()
    });
    doc.para_mut(StoryRef::Body, &Path::top(0)).unwrap().props.style = Some("ExactLineBoxChild".into());
    let mut cache = LayoutCache::new();
    let options = LayoutOptions::default();
    let legacy = measured(&document(LineSpacing::AtLeast(24.0), 12.0, None, None, false));
    let inherited_top = paragraph(&layout(&doc, &mut cache, &options));
    close_contract(inherited_top.lines[0].baseline, legacy.lines[0].baseline);
    let rev = doc.para(StoryRef::Body, &Path::top(0)).unwrap().rev;
    doc.styles.upsert(base("baseline"));
    assert_eq!(
        doc.para(StoryRef::Body, &Path::top(0)).unwrap().rev,
        rev,
        "a style change must invalidate the style environment without touching paragraph revision"
    );
    let inherited_baseline = paragraph(&layout(&doc, &mut cache, &options));
    close_word(inherited_baseline.lines[0].baseline, 19.22);
    close_contract(inherited_baseline.lines[0].baseline, measured(&doc).lines[0].baseline);
    doc.styles.upsert(base("top"));
    close_contract(paragraph(&layout(&doc, &mut cache, &options)).lines[0].baseline, legacy.lines[0].baseline);
    let p = doc.para_mut(StoryRef::Body, &Path::top(0)).unwrap();
    p.props = alignment(&p.props, Some("auto"));
    p.touch();
    let explicit_auto = paragraph(&layout(&doc, &mut cache, &options));
    close_word(explicit_auto.lines[0].baseline, 19.22);
    close_contract(explicit_auto.lines[0].baseline, measured(&doc).lines[0].baseline);
}
