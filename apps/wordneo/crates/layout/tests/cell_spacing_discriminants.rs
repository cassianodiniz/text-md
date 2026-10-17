//! Independent no-wrap variable-grid Word controls, separate from the first 2x2 REDs.
//! Source evidence: table-spacing008/runs/word-20261016T2042323463191Z-d4340e30.
use std::sync::Arc;

use wordcraft_doc::props::{Border, Borders, HeightRule};
use wordcraft_doc::{Block, Document, Paragraph, Table, para_block};
use wordcraft_layout::{LayoutCache, LayoutOptions, Placed, layout};

fn positions(grid: &[f32], rows: usize, spacing: f32, height: Option<HeightRule>) -> Vec<(f32, f32)> {
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
    let mut table = Table::new(rows, grid.len(), 240.0);
    table.grid = grid.to_vec();
    table.props.style = None;
    table.props.width = Some(240.0);
    table.props.fixed = true;
    table.props.cell_margins = Some([2.0; 4]);
    table.props.cell_spacing = Some(spacing);
    table.props.borders = Some(Borders::all(Border::single(0.0)));
    for (ri, row) in table.rows.iter_mut().enumerate() {
        if let Some(rule) = height {
            row.props.height = Some(20.0);
            row.props.height_rule = rule;
        }
        for (ci, cell) in row.cells.iter_mut().enumerate() {
            // Match each control's direct preferred cell width; Word recalculates the grid.
            cell.props.width = Some(grid[ci]);
            let marker = format!("{}{}", char::from(b'A' + ci as u8), ri + 1);
            cell.blocks = vec![para_block(Paragraph::with_text(&marker, Default::default()))];
        }
    }
    document.body = vec![Arc::new(Block::Table(table)), para_block(Paragraph::with_text("SPACING-AFTER", Default::default()))];
    let result = layout(&document, &mut LayoutCache::new(), &LayoutOptions::default());
    assert_eq!(result.pages.len(), 1);
    let mut paths: Vec<Vec<u32>> = (0..rows).flat_map(|r| (0..grid.len()).map(move |c| vec![0, r as u32, c as u32, 0])).collect();
    paths.push(vec![1]);
    paths
        .iter()
        .map(|wanted| {
            result.pages[0]
                .items
                .iter()
                .find_map(|item| match item {
                    Placed::Lines { path, x, y, .. } if path.0 == *wanted => Some((*x, *y)),
                    _ => None,
                })
                .expect("Every short synthetic marker must be laid out")
        })
        .collect()
}

fn close(actual: f32, expected: f32, context: &str) {
    assert!((actual - expected).abs() <= 0.06, "{context}: measured delta {actual}, expected Word delta {expected}");
}

#[test]
fn three_equal_columns_reserve_their_four_gaps_inside_the_fixed_width() {
    let grid = [80.0, 80.0, 80.0];
    let baseline = positions(&grid, 3, 0.0, None);
    let actual = positions(&grid, 3, 1.5, None);
    for row in 0..3 {
        for (col, expected) in [3.0, 2.0, 1.0].into_iter().enumerate() {
            let i = row * 3 + col;
            close(actual[i].0 - baseline[i].0, expected, "equal-column horizontal spacing");
            close(actual[i].1 - baseline[i].1, 3.0 * (row + 1) as f32, "three-row vertical spacing");
        }
    }
    close(actual[9].1 - baseline[9].1, 12.0, "after three automatic rows");
}

#[test]
fn unequal_columns_keep_the_observed_preferred_width_reconciliation() {
    for (grid, expected) in [(vec![60.0, 180.0], vec![3.0, 3.9]), (vec![60.0, 120.0, 60.0], vec![3.0, 3.1, -0.1])] {
        let baseline = positions(&grid, 2, 0.0, None);
        let actual = positions(&grid, 2, 1.5, None);
        for row in 0..2 {
            for (col, wanted) in expected.iter().enumerate() {
                let i = row * grid.len() + col;
                close(actual[i].0 - baseline[i].0, *wanted, "unequal preferred-grid spacing");
            }
        }
    }
}

#[test]
fn exact_row_heights_absorb_internal_spacing_without_advancing_the_next_row() {
    let grid = [120.0, 120.0];
    let baseline = positions(&grid, 2, 0.0, Some(HeightRule::Exact));
    let actual = positions(&grid, 2, 1.5, Some(HeightRule::Exact));
    for i in 0..4 {
        close(actual[i].1 - baseline[i].1, 3.0, "exact-height row baseline");
    }
    close(actual[4].1 - baseline[4].1, 4.5, "after two exact rows");
}

#[test]
fn minimum_row_heights_keep_the_inter_row_and_trailing_spacing() {
    let grid = [120.0, 120.0];
    let baseline = positions(&grid, 2, 0.0, Some(HeightRule::AtLeast));
    let actual = positions(&grid, 2, 1.5, Some(HeightRule::AtLeast));
    for i in 0..4 {
        close(actual[i].1 - baseline[i].1, 3.0 * (i / 2 + 1) as f32, "minimum-height row baseline");
    }
    close(actual[4].1 - baseline[4].1, 9.0, "after two minimum-height rows");
}
