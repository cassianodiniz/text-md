//! Word's four bidi-only anchor contrasts isolate border width from cell spacing.
//! With spacing30dxa, Word RTL text x is423.58/305.05 for borders0/1/2pt.
//! This guard tests that invariance, not a fitted absolute translation.
use std::sync::Arc;

use wordcraft_doc::props::{Border, Borders, LineSpacing};
use wordcraft_doc::{Block, Document, Paragraph, Table, para_block};
use wordcraft_layout::{LayoutCache, LayoutOptions, Placed, layout};

fn rtl_positions(border: f32) -> [f32; 2] {
    let mut doc = Document::new();
    doc.settings.compat_mode = 15;
    doc.last_section.page_w = 612.0;
    doc.last_section.page_h = 792.0;
    doc.last_section.margin_left = 72.0;
    doc.last_section.margin_right = 72.0;
    doc.last_section.margin_top = 72.0;
    doc.last_section.margin_bottom = 72.0;
    doc.styles.default_chr.font = Some("Calibri".into());
    doc.styles.default_chr.size = Some(12.0);
    doc.styles.default_para.space_before = Some(0.0);
    doc.styles.default_para.space_after = Some(0.0);
    let mut table = Table::new(2, 2, 240.0);
    table.props.style = None;
    table.props.width = Some(240.0);
    table.props.fixed = true;
    table.props.bidi_visual = true;
    table.props.cell_spacing = Some(1.5);
    table.props.cell_margins = Some([2.0; 4]);
    table.props.borders = Some(Borders::all(Border::single(border)));
    for (ri, row) in table.rows.iter_mut().enumerate() {
        for (ci, cell) in row.cells.iter_mut().enumerate() {
            let mut paragraph = Paragraph::with_text(&format!("SPACING-{}{}", if ci == 0 { 'A' } else { 'B' }, ri + 1), Default::default());
            paragraph.props.line_spacing = Some(LineSpacing::Multiple(1.0));
            cell.blocks = vec![para_block(paragraph)];
        }
    }
    doc.body = vec![Arc::new(Block::Table(table)), para_block(Paragraph::with_text("SPACING-AFTER", Default::default()))];
    let result = layout(&doc, &mut LayoutCache::new(), &LayoutOptions::default());
    assert_eq!(result.pages.len(), 1);
    [0, 1].map(|cell| {
        result.pages[0]
            .items
            .iter()
            .find_map(|item| match item {
                Placed::Lines { path, x, .. } if path.0 == [0, 0, cell, 0] => Some(*x),
                _ => None,
            })
            .expect("Both first-row RTL markers must be present")
    })
}

#[test]
fn separated_rtl_text_anchor_is_invariant_under_the_measured_border_width_changes() {
    let zero = rtl_positions(0.0);
    for border in [1.0, 2.0] {
        let current = rtl_positions(border);
        for cell in 0..2 {
            assert!(
                (current[cell] - zero[cell]).abs() <= 0.06,
                "Word RTL cell{cell} stays at the same x for border0/1/2; border{border} changed native x by{}",
                current[cell] - zero[cell]
            );
        }
    }
}
