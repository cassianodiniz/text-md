//! The compatibility switch puts excess exact-line leading below the text.
//! Public OOXML semantics plus measured Word table/body contrasts; this does
//! not repair the separate baseline difference when the switch is absent.
use std::sync::Arc;

use wordcraft_doc::props::{CharProps, LineSpacing};
use wordcraft_doc::{Document, Paragraph, Settings, para_block};
use wordcraft_layout::{DocLayout, LayoutCache, LayoutOptions, Placed, layout};

fn settings_with_flag(settings: &Settings, flag: Option<bool>) -> Settings {
    // Deserialize the public JSON representation so these tests compile
    // before the missing model member is added (a behavioral, not compile RED).
    let mut value = serde_json::to_value(settings).unwrap();
    if let Some(flag) = flag {
        value["noExtraLineSpacing"] = flag.into();
    } else {
        value.as_object_mut().unwrap().remove("noExtraLineSpacing");
    }
    serde_json::from_value(value).unwrap()
}

fn document(spacing: LineSpacing, flag: Option<bool>) -> Document {
    let mut doc = Document::new();
    let chr = CharProps { font: Some("Calibri".into()), size: Some(12.0), ..Default::default() };
    let mut p = Paragraph::with_text("SPACING-A1\nSPACING-A1", chr.clone());
    p.mark = chr;
    p.props.line_spacing = Some(spacing);
    p.props.space_before = Some(0.0);
    p.props.space_after = Some(0.0);
    doc.body = vec![para_block(p)];
    doc.settings = settings_with_flag(&doc.settings, flag);
    doc
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

fn measured(doc: &Document) -> Arc<wordcraft_layout::para::ParaLayout> {
    paragraph(&layout(doc, &mut LayoutCache::new(), &LayoutOptions::default()))
}

fn close(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 0.0001, "actual={actual}, expected={expected}");
}

#[test]
fn explicit_true_uses_natural_first_baseline_and_keeps_exact_advance() {
    let natural = measured(&document(LineSpacing::Multiple(1.0), None));
    let exact = measured(&document(LineSpacing::Exactly(24.0), Some(true)));
    assert_eq!(exact.lines.len(), 2);
    for (i, line) in exact.lines.iter().enumerate() {
        close(line.height, 24.0);
        close(line.top, 24.0 * i as f32);
        close(line.baseline - line.top, natural.lines[0].baseline);
    }
}

#[test]
fn explicit_true_does_not_move_single_or_at_least_lines() {
    for spacing in [LineSpacing::Multiple(1.0), LineSpacing::Multiple(2.0), LineSpacing::AtLeast(24.0)] {
        let off = measured(&document(spacing, Some(false)));
        let on = measured(&document(spacing, Some(true)));
        assert_eq!(on.lines.len(), off.lines.len());
        for (a, b) in on.lines.iter().zip(&off.lines) {
            close(a.top, b.top);
            close(a.height, b.height);
            close(a.baseline, b.baseline);
            assert_eq!(a.xs, b.xs);
        }
    }
}

#[test]
fn absent_and_explicit_false_keep_existing_exact_layout() {
    let absent = measured(&document(LineSpacing::Exactly(24.0), None));
    let off = measured(&document(LineSpacing::Exactly(24.0), Some(false)));
    for (a, b) in absent.lines.iter().zip(&off.lines) {
        close(a.top, b.top);
        close(a.height, b.height);
        close(a.baseline, b.baseline);
        assert_eq!(a.xs, b.xs);
    }
}

#[test]
fn switching_compatibility_recomputes_cached_paragraphs_without_revision_change() {
    let mut doc = document(LineSpacing::Exactly(24.0), Some(false));
    let opts = LayoutOptions::default();
    let mut cache = LayoutCache::new();
    let off = paragraph(&layout(&doc, &mut cache, &opts));
    doc.settings = settings_with_flag(&doc.settings, Some(true));
    let on_warm = paragraph(&layout(&doc, &mut cache, &opts));
    let on_fresh = measured(&doc);
    assert!(on_fresh.lines[0].baseline < off.lines[0].baseline - 1.0, "Flag must affect the exact-height line before checking cache invalidation");
    for (a, b) in on_warm.lines.iter().zip(&on_fresh.lines) {
        close(a.baseline, b.baseline);
    }
    doc.settings = settings_with_flag(&doc.settings, Some(false));
    let off_again = paragraph(&layout(&doc, &mut cache, &opts));
    for (a, b) in off_again.lines.iter().zip(&off.lines) {
        close(a.baseline, b.baseline);
    }
}

#[test]
fn short_exact_line_places_baseline_at_line_bottom_as_measured_in_word() {
    let natural = measured(&document(LineSpacing::Multiple(1.0), None));
    assert!(natural.lines[0].height > 8.0);
    let on = measured(&document(LineSpacing::Exactly(8.0), Some(true)));
    assert_eq!(on.lines.len(), 2);
    for line in &on.lines {
        close(line.height, 8.0);
        // Word body starts at72, and the two baselines are80/88.
        // This observed offset is8, not the native descent-based offset5.
        close(line.baseline - line.top, 8.0);
    }
}

#[test]
fn final_empty_line_uses_same_exact_leading_policy_with_or_without_grid() {
    use wordcraft_doc::section::{DocGrid, DocGridType};
    let natural = measured(&document(LineSpacing::Multiple(1.0), None));
    for grid in [None, Some(DocGrid { kind: DocGridType::Lines, line_pitch: 36.0, ..Default::default() })] {
        let mut doc = document(LineSpacing::Exactly(24.0), Some(true));
        let chr = CharProps { font: Some("Calibri".into()), size: Some(12.0), ..Default::default() };
        let mut p = Paragraph::with_text("SPACING-A1\n", chr.clone());
        p.mark = chr;
        p.props.line_spacing = Some(LineSpacing::Exactly(24.0));
        p.props.snap_to_grid = Some(true);
        p.props.space_before = Some(0.0);
        p.props.space_after = Some(0.0);
        doc.body = vec![para_block(p)];
        doc.last_section.doc_grid = grid;
        let result = measured(&doc);
        assert_eq!(result.lines.len(), 2);
        for line in &result.lines {
            close(line.height, 24.0);
            close(line.baseline - line.top, natural.lines[0].baseline);
        }
    }
}
