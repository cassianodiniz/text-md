//! Independent Word black-box spacing controls. Only observed deltas are pinned here.
//! Source evidence: table-spacing008/runs/word-20261016T2039069288987Z-c5264c78.
use std::sync::Arc;

use wordcraft_doc::props::{Border, Borders};
use wordcraft_doc::table::Table;
use wordcraft_doc::{Block, Document, Paragraph, para_block};
use wordcraft_layout::{LayoutCache, LayoutOptions, Placed, layout};

fn locations(spacing: f32, border: f32) -> Vec<(f32, f32)> {
    let mut document = Document::new();
    document.settings.compat_mode = 15;
    document.last_section.page_w = 612.0;
    document.last_section.page_h = 792.0;
    document.last_section.margin_left = 72.0;
    document.last_section.margin_top = 72.0;
    document.last_section.margin_right = 72.0;
    document.last_section.margin_bottom = 72.0;
    document.styles.default_chr.font = Some("Calibri".into());
    document.styles.default_chr.size = Some(12.0);
    document.styles.default_para.space_before = Some(0.0);
    document.styles.default_para.space_after = Some(0.0);
    let mut table = Table::new(2, 2, 240.0);
    table.props.style = None;
    table.props.width = Some(240.0);
    table.props.fixed = true;
    table.props.cell_margins = Some([2.0; 4]);
    table.props.cell_spacing = Some(spacing);
    table.props.borders = Some(Borders::all(Border::single(border)));
    for (ri, row) in table.rows.iter_mut().enumerate() {
        for (ci, cell) in row.cells.iter_mut().enumerate() {
            cell.blocks =
                vec![para_block(Paragraph::with_text(&format!("SPACING-{}{}", if ci == 0 { 'A' } else { 'B' }, ri + 1), Default::default()))];
        }
    }
    document.body = vec![Arc::new(Block::Table(table)), para_block(Paragraph::with_text("SPACING-AFTER", Default::default()))];
    let result = layout(&document, &mut LayoutCache::new(), &LayoutOptions::default());
    assert_eq!(result.pages.len(), 1);
    let paths: &[&[u32]] = &[&[0, 0, 0, 0], &[0, 0, 1, 0], &[0, 1, 0, 0], &[0, 1, 1, 0], &[1]];
    paths
        .iter()
        .map(|wanted| {
            result.pages[0]
                .items
                .iter()
                .find_map(|item| match item {
                    Placed::Lines { path, x, y, .. } if path.0.as_slice() == *wanted => Some((*x, *y)),
                    _ => None,
                })
                .expect("Every synthetic marker must be laid out")
        })
        .collect()
}

fn close(actual: f32, expected: f32, context: &str) {
    assert!((actual - expected).abs() <= 0.06, "{context}: measured delta {actual}, expected Word delta {expected}");
}

#[test]
fn cell_spacing_moves_each_row_and_downstream_content_by_the_observed_gaps() {
    for border in [0.0, 2.0] {
        let baseline = locations(0.0, border);
        for spacing in [0.75, 1.5, 3.0] {
            let actual = locations(spacing, border);
            // Actual PDFs show a 2*s top gap, another between rows and another after.
            // Baseline font measurement cancels because only spacing changes.
            for i in 0..4 {
                let row = i / 2;
                close(actual[i].1 - baseline[i].1, 2.0 * spacing * (row + 1) as f32, "row vertical position");
            }
            close(actual[4].1 - baseline[4].1, 6.0 * spacing, "paragraph after table");
        }
    }
}

#[test]
fn fixed_equal_columns_reserve_spacing_inside_the_declared_table_width() {
    let baseline = locations(0.0, 0.0);
    for spacing in [0.75, 1.5, 3.0] {
        let actual = locations(spacing, 0.0);
        for row in 0..2 {
            close(actual[row * 2].0 - baseline[row * 2].0, 2.0 * spacing, "first column x");
            close(actual[row * 2 + 1].0 - baseline[row * 2 + 1].0, spacing, "second column x");
        }
        close(actual[4].0 - baseline[4].0, 0.0, "post-table x must not change");
    }
}

#[test]
fn separated_two_point_borders_add_the_observed_half_border_horizontal_clearance() {
    let baseline = locations(0.0, 2.0);
    for spacing in [0.75, 1.5, 3.0] {
        let actual = locations(spacing, 2.0);
        for row in 0..2 {
            close(actual[row * 2].0 - baseline[row * 2].0, 2.0 * spacing + 1.0, "first bordered column x");
            close(actual[row * 2 + 1].0 - baseline[row * 2 + 1].0, spacing + 1.0, "second bordered column x");
        }
    }
}
