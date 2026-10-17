//! Faithful public-layout reproduction of flow/nested-absent, not a general
//! rule for empty paragraphs. The control has no hideMark, no spacing and no
//! preferred widths; a direct Calibri12/line240 empty paragraph follows its child.
//! Word parent gray and child red/green fills all end at90.620pt.
use std::sync::Arc;

use wordcraft_doc::props::{Align, Border, BorderStyle, Borders, CharProps, LineSpacing, Rgb};
use wordcraft_doc::{Block, Document, Paragraph, Table, para_block};
use wordcraft_layout::{DocLayout, LayoutCache, LayoutOptions, Placed, layout};

fn calibri() -> CharProps {
    CharProps {
        font: Some("Calibri".into()),
        font_cs: Some("Calibri".into()),
        font_ea: Some("Calibri".into()),
        size: Some(12.0),
        size_cs: Some(12.0),
        ..Default::default()
    }
}

fn make_paragraph(text: &str) -> Paragraph {
    let mut paragraph = Paragraph::with_text(text, calibri());
    // The DOCX has direct run formatting, but no pPr/rPr paragraph-mark
    // formatting. An empty run contributes no run to the parsed model.
    paragraph.mark = CharProps::default();
    paragraph.props.align = Some(Align::Left);
    paragraph.props.space_before = Some(0.0);
    paragraph.props.space_after = Some(0.0);
    paragraph.props.line_spacing = Some(LineSpacing::Multiple(1.0));
    paragraph
}

fn cell_paragraph(text: &str) -> Arc<Block> {
    para_block(make_paragraph(text))
}

fn automatic_table(columns: usize, margins: f32) -> Table {
    let mut table = Table::new(1, columns, 240.0);
    table.props.style = None;
    table.props.width = None;
    table.props.width_pct = None;
    table.props.fixed = false;
    table.props.align = Some(Align::Left);
    table.props.indent = Some(0.0);
    table.props.cell_spacing = None;
    table.props.cell_margins = Some([margins; 4]);
    table.props.borders = Some(Borders::all(Border { style: BorderStyle::None, width: 0.0, color: Some(Rgb(0, 0, 0)), space: 0.0 }));
    for cell in &mut table.rows[0].cells {
        cell.props.width = None;
        cell.props.width_pct = None;
        cell.props.hide_mark = false;
    }
    table
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tail {
    Base,
    Before6,
    After6,
    TextX,
    TwoEmpty,
}

fn control(tail: Tail) -> DocLayout {
    let mut doc = Document::new();
    doc.settings.compat_mode = 15;
    doc.last_section.page_w = 612.0;
    doc.last_section.page_h = 792.0;
    doc.last_section.margin_left = 72.0;
    doc.last_section.margin_right = 72.0;
    doc.last_section.margin_top = 72.0;
    doc.last_section.margin_bottom = 72.0;
    // Preserve Aptos12 mark/defaults. Nonempty runs directly specify Calibri12;
    // the authored empty run contributes no font to the parsed paragraph mark.
    let mut outer = automatic_table(1, 0.0);
    let mut inner = automatic_table(2, 2.0);
    outer.rows[0].cells[0].props.shading = Some(Rgb(232, 232, 232));
    for ((cell, text), color) in inner.rows[0].cells.iter_mut().zip(["A1", "B1"]).zip([Rgb(255, 224, 224), Rgb(224, 255, 224)]) {
        cell.props.shading = Some(color);
        cell.blocks = vec![cell_paragraph(text)];
    }
    let mut final_para = make_paragraph(if tail == Tail::TextX { "X" } else { "" });
    if tail == Tail::Before6 {
        final_para.props.space_before = Some(6.0);
    }
    if tail == Tail::After6 {
        final_para.props.space_after = Some(6.0);
    }
    outer.rows[0].cells[0].blocks = vec![Arc::new(Block::Table(inner)), para_block(final_para)];
    if tail == Tail::TwoEmpty {
        outer.rows[0].cells[0].blocks.push(cell_paragraph(""));
    }
    let mut after = Paragraph::with_text("SPACING-AFTER", CharProps { font: Some("Calibri".into()), size: Some(12.0), ..Default::default() });
    after.mark = CharProps::default();
    after.props.space_before = Some(0.0);
    after.props.space_after = Some(0.0);
    doc.body = vec![Arc::new(Block::Table(outer)), para_block(after)];
    let result = layout(&doc, &mut LayoutCache::new(), &LayoutOptions::default());
    assert_eq!(result.pages.len(), 1);
    result
}

fn assert_parent_ends_with_child(tail: Tail) {
    let result = control(tail);
    let fill = |wanted| {
        result.pages[0]
            .items
            .iter()
            .find_map(|item| match item {
                Placed::Fill { rect, color } if *color == wanted => Some(*rect),
                _ => None,
            })
            .expect("All three control colors must be present")
    };
    let parent = fill(Rgb(232, 232, 232));
    let children = [fill(Rgb(255, 224, 224)), fill(Rgb(224, 255, 224))];
    for child in children {
        assert!(
            (parent.bottom() - child.bottom()).abs() <= 0.06,
            "Word gray parent and both child fills end together; native parent{} versus child{}",
            parent.bottom(),
            child.bottom()
        );
    }
}

#[test]
fn nested_absent_parent_fill_ends_with_the_child_despite_its_required_empty_trailing_paragraph() {
    assert_parent_ends_with_child(Tail::Base);
}

#[test]
fn final_empty_paragraph_before_six_does_not_extend_the_nested_control() {
    assert_parent_ends_with_child(Tail::Before6);
}

#[test]
fn final_empty_paragraph_after_six_does_not_extend_the_nested_control() {
    assert_parent_ends_with_child(Tail::After6);
}

fn after_y(result: &DocLayout) -> f32 {
    result.pages[0]
        .items
        .iter()
        .find_map(|item| match item {
            Placed::Lines { path, y, .. } if path.0 == [1] => Some(*y),
            _ => None,
        })
        .expect("The following body paragraph must remain present")
}

#[test]
fn a_nonempty_x_tail_keeps_its_observed_extra_line_instead_of_being_suppressed() {
    let baseline = control(Tail::Base);
    let actual = control(Tail::TextX);
    // Same following text/font in both PDFs:107.72 minus93.03. Comparing
    // the public line origins cancels the font-dependent glyph-top offset.
    let delta = after_y(&actual) - after_y(&baseline);
    assert!((delta - 14.69).abs() <= 0.06, "Word X tail advances AFTER by14.69pt; native{delta}");
    assert!(
        actual.pages[0].items.iter().any(|item| matches!(item,
            Placed::Lines { path, para, .. } if path.0 == [0, 0, 0, 1] && para.text_len == 1 && !para.glyphs.is_empty()
        )),
        "The X paragraph must remain laid out"
    );
}

#[test]
fn two_authored_empty_paragraphs_keep_both_observed_lines() {
    let baseline = control(Tail::Base);
    let actual = control(Tail::TwoEmpty);
    // Word AFTER122.33 minus93.03: both empty paragraphs consume height.
    let delta = after_y(&actual) - after_y(&baseline);
    assert!((delta - 29.30).abs() <= 0.06, "Word two-empty tail advances AFTER by29.30pt; native{delta}");
    for block in [1, 2] {
        assert!(
            actual.pages[0].items.iter().any(|item| matches!(item,
                Placed::Lines { path, .. } if path.0 == [0, 0, 0, block]
            )),
            "Both authored empty tail paragraphs must remain laid out"
        );
    }
}
