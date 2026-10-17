//! Independent physical strokes from the narrow Word controls.
//! Nominal cell advance is not the same as the distance between border centers.
//! Sources: independent-core-review/runs/word-*212007*-4370759c and word-*212156*-77b794ed.
use std::sync::Arc;

use wordcraft_doc::props::{Border, Borders, LineSpacing};
use wordcraft_doc::{Block, Document, Paragraph, Table, para_block};
use wordcraft_layout::{LayoutCache, LayoutOptions, Placed, layout};

fn check_cell_ink(border: f32, margin: f32, expected_centers: f32, expected_ink: f32) {
    let mut doc = Document::new();
    doc.settings.compat_mode = 15;
    doc.last_section.margin_left = 72.0;
    doc.last_section.margin_right = 72.0;
    doc.last_section.margin_top = 72.0;
    doc.styles.default_chr.font = Some("Calibri".into());
    doc.styles.default_chr.size = Some(12.0);
    doc.styles.default_para.space_before = Some(0.0);
    doc.styles.default_para.space_after = Some(0.0);
    let mut table = Table::new(1, 2, 20.0);
    table.props.style = None;
    table.props.width = Some(20.0);
    table.props.fixed = true;
    table.props.cell_spacing = Some(6.0);
    table.props.cell_margins = Some([margin; 4]);
    table.props.borders = Some(Borders::all(Border::single(border)));
    for cell in &mut table.rows[0].cells {
        let mut p = Paragraph::with_text("A", Default::default());
        p.props.line_spacing = Some(LineSpacing::Multiple(1.0));
        cell.blocks = vec![para_block(p)];
    }
    doc.body = vec![Arc::new(Block::Table(table)), para_block(Paragraph::with_text("SPACING-AFTER", Default::default()))];
    let result = layout(&doc, &mut LayoutCache::new(), &LayoutOptions::default());
    assert_eq!(result.pages.len(), 1);
    let cell = result.pages[0]
        .items
        .iter()
        .find_map(|item| match item {
            Placed::Cell { table, row: 0, cell: 0, rect, .. } if table.0 == [0] => Some(*rect),
            _ => None,
        })
        .expect("The first cell must be present");
    // Use the public cell only to locate its two vertical strokes. The two
    // twelve-point gaps isolate them from the outer frame and the next cell.
    let strokes: Vec<(f32, f32)> = result.pages[0]
        .items
        .iter()
        .filter_map(|item| match item {
            Placed::Rule { x0, x1, y0, y1, border }
                if (x1 - x0).abs() < 0.01
                    && (y1 - y0).abs() > 1.0
                    && *x0 >= cell.x - 2.0 * border.width
                    && *x0 <= cell.right() + 2.0 * border.width =>
            {
                Some((*x0, border.width))
            }
            _ => None,
        })
        .collect();
    assert_eq!(strokes.len(), 2, "Locate exactly the first cell's left/right strokes: {strokes:?}");
    let left_center = strokes.iter().map(|s| s.0).min_by(f32::total_cmp).unwrap();
    let right_center = strokes.iter().map(|s| s.0).max_by(f32::total_cmp).unwrap();
    let ink_left = strokes.iter().map(|s| s.0 - s.1 / 2.0).min_by(f32::total_cmp).unwrap();
    let ink_right = strokes.iter().map(|s| s.0 + s.1 / 2.0).max_by(f32::total_cmp).unwrap();
    assert!(
        (right_center - left_center - expected_centers).abs() <= 0.06,
        "border{border}/margin{margin}: Word centers span{expected_centers}pt, native{}",
        right_center - left_center
    );
    assert!(
        (ink_right - ink_left - expected_ink).abs() <= 0.06,
        "border{border}/margin{margin}: Word ink spans{expected_ink}pt, native{}",
        ink_right - ink_left
    );
}

#[test]
fn one_point_border_zero_margin_has_two_point_centers_and_three_point_ink() {
    // PDF vertical strips84.525..85.525 and86.525..87.525.
    check_cell_ink(1.0, 0.0, 2.0, 3.0);
}

#[test]
fn two_point_border_zero_margin_has_three_point_centers_and_five_point_ink() {
    // PDF vertical strips85.025..87.025 and88.025..90.025.
    check_cell_ink(2.0, 0.0, 3.0, 5.0);
}

#[test]
fn one_point_border_two_point_margin_has_six_point_centers_and_seven_point_ink() {
    // PDF vertical strips84.525..85.525 and90.525..91.525.
    check_cell_ink(1.0, 2.0, 6.0, 7.0);
}
