//! Synthetic bidi-only pairs measured in Word, without assuming mirror symmetry.
//! Evidence: table-spacing008/independent-core-review/runs/
//! word-20261016T2116282077612Z-901d5d7f (PDFs and generated document.xml).
use std::sync::Arc;

use wordcraft_doc::props::{Border, Borders, LineSpacing, Rgb};
use wordcraft_doc::{Block, Document, Paragraph, Table, para_block};
use wordcraft_layout::{DocLayout, LayoutCache, LayoutOptions, Placed, layout};

fn document() -> Document {
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
    doc
}

fn paragraph(text: &str) -> Arc<Block> {
    let mut para = Paragraph::with_text(text, Default::default());
    para.props.line_spacing = Some(LineSpacing::Multiple(1.0));
    para_block(para)
}

fn placed(table: Table) -> DocLayout {
    let mut doc = document();
    doc.body = vec![Arc::new(Block::Table(table)), para_block(Paragraph::with_text("SPACING-AFTER", Default::default()))];
    let result = layout(&doc, &mut LayoutCache::new(), &LayoutOptions::default());
    assert_eq!(result.pages.len(), 1);
    result
}

fn line_x(result: &DocLayout, wanted: &[u32]) -> f32 {
    result.pages[0]
        .items
        .iter()
        .find_map(|item| match item {
            Placed::Lines { path, x, .. } if path.0 == wanted => Some(*x),
            _ => None,
        })
        .expect("The synthetic marker must be laid out")
}

fn outer_edges(result: &DocLayout) -> (f32, f32) {
    let vertical: Vec<f32> = result.pages[0]
        .items
        .iter()
        .filter_map(|item| match item {
            Placed::Rule { x0, y0, x1, y1, border } if (x0 - x1).abs() < 0.001 && (y1 - y0).abs() > 1.0 && border.is_visible() => Some(*x0),
            _ => None,
        })
        .collect();
    assert!(!vertical.is_empty());
    (vertical.iter().copied().fold(f32::INFINITY, f32::min), vertical.iter().copied().fold(f32::NEG_INFINITY, f32::max))
}

fn close(actual: f32, expected: f32, context: &str) {
    assert!((actual - expected).abs() <= 0.06, "{context}: actual {actual}, Word measurement {expected}");
}

fn symmetric(rtl: bool) -> DocLayout {
    let mut table = Table::new(2, 2, 240.0);
    table.props.style = None;
    table.props.width = Some(240.0);
    table.props.fixed = true;
    table.props.bidi_visual = rtl;
    table.props.cell_spacing = Some(1.5);
    table.props.cell_margins = Some([2.0; 4]);
    table.props.borders = Some(Borders::all(Border::single(2.0)));
    for (ri, row) in table.rows.iter_mut().enumerate() {
        for (ci, cell) in row.cells.iter_mut().enumerate() {
            cell.blocks = vec![paragraph(&format!("SPACING-{}{}", if ci == 0 { 'A' } else { 'B' }, ri + 1))];
        }
    }
    placed(table)
}

#[test]
fn separated_rtl_bordered_table_matches_the_observed_bidi_only_position_deltas() {
    let ltr = symmetric(false);
    let rtl = symmetric(true);
    // PDF first-row x: LTR 80.025/198.520, RTL 423.580/305.050.
    // The generated XML keeps the same grid, tcW, margins and borders.
    close(line_x(&rtl, &[0, 0, 0, 0]) - line_x(&ltr, &[0, 0, 0, 0]), 343.555, "logical first column");
    close(line_x(&rtl, &[0, 0, 1, 0]) - line_x(&ltr, &[0, 0, 1, 0]), 106.530, "logical second column");
    let (ltr_left, ltr_right) = outer_edges(&ltr);
    let (rtl_left, rtl_right) = outer_edges(&rtl);
    // Border centers: 73.025..313.050 and 298.050..538.100.
    close(rtl_left - ltr_left, 225.025, "outer left border");
    close(rtl_right - ltr_right, 225.050, "outer right border");
    close(line_x(&rtl, &[1]) - line_x(&ltr, &[1]), 0.0, "following paragraph");
}

fn asymmetric(rtl: bool) -> DocLayout {
    let mut table = Table::new(2, 3, 360.0);
    table.grid = vec![80.0, 160.0, 120.0];
    table.props.style = None;
    table.props.width = Some(360.0);
    table.props.fixed = true;
    table.props.bidi_visual = rtl;
    table.props.cell_spacing = Some(2.25);
    table.props.cell_margins = Some([2.0; 4]);
    table.props.borders = Some(Borders::all(Border::single(0.0)));
    for (ri, row) in table.rows.iter_mut().enumerate() {
        for (ci, cell) in row.cells.iter_mut().enumerate() {
            cell.props.width = Some([80.0, 160.0, 120.0][ci]);
            let (left, right) = [(0.0, 8.0), (7.0, 2.0), (1.0, 1.0)][ci];
            cell.props.margins = Some([2.0, left, 2.0, right]);
            cell.props.shading = Some([Rgb(255, 224, 224), Rgb(224, 255, 224), Rgb(224, 224, 255)][ci]);
            cell.blocks = vec![paragraph(&format!("{}{}", ['A', 'B', 'C'][ci], ri + 1))];
        }
    }
    placed(table)
}

#[test]
fn separated_rtl_cells_keep_the_observed_physical_left_padding() {
    for rtl in [false, true] {
        let result = asymmetric(rtl);
        for (ci, expected) in [0.0, 7.0, 1.0].into_iter().enumerate() {
            let rect_x = result.pages[0]
                .items
                .iter()
                .find_map(|item| match item {
                    Placed::Cell { table, row: 0, cell, rect, .. } if table.0 == [0] && *cell == ci => Some(rect.x),
                    _ => None,
                })
                .expect("Every first-row cell must have a box");
            // PDF text minus same-color outer cell-fill x is 0/7/1 pt in both
            // directions; the bidi-only XML change does not swap tcMar left/right.
            close(line_x(&result, &[0, 0, ci as u32, 0]) - rect_x, expected, "physical left padding");
        }
    }
}
