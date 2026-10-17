//! Relative (font-independent) geometry from the four Exact spacing contrasts.
//! Source: table-spacing008/runs/word-20261016T2108040826228Z-ed58e3be.
use std::sync::Arc;

use wordcraft_doc::props::{Border, Borders, HeightRule, LineSpacing};
use wordcraft_doc::{Block, Document, Paragraph, Table, para_block};
use wordcraft_layout::{LayoutCache, LayoutOptions, Placed, layout};

struct PageGeometry {
    text_y: f32,
    cell_bottom: f32,
    frame_bottom: Option<f32>,
}

fn geometry(spacing: f32) -> Vec<PageGeometry> {
    let mut document = Document::new();
    document.settings.compat_mode = 15;
    document.last_section.page_w = 360.0;
    document.last_section.page_h = 360.0;
    document.last_section.margin_top = 36.0;
    document.last_section.margin_bottom = 36.0;
    document.last_section.margin_left = 36.0;
    document.last_section.margin_right = 36.0;
    document.styles.default_chr.font = Some("Calibri".into());
    document.styles.default_chr.size = Some(12.0);
    document.styles.default_para.space_before = Some(0.0);
    document.styles.default_para.space_after = Some(0.0);
    let mut table = Table::new(2, 2, 240.0);
    table.props.style = None;
    table.props.width = Some(240.0);
    table.props.fixed = true;
    table.props.cell_spacing = Some(spacing);
    table.props.cell_margins = Some([2.0; 4]);
    table.props.borders = Some(Borders::all(Border::single(1.0)));
    for row in &mut table.rows {
        row.props.height = Some(240.0);
        row.props.height_rule = HeightRule::Exact;
        row.props.cant_split = true;
        for cell in &mut row.cells {
            let mut p = Paragraph::with_text("Hjg", Default::default());
            p.props.line_spacing = Some(LineSpacing::Multiple(1.0));
            cell.blocks = vec![para_block(p)];
        }
    }
    // Match the control's following paragraph: before/after0 and inherited
    // line spacing, distinct from the cells' direct line240/auto setting.
    let mut after = Paragraph::with_text("SPACING-AFTER", Default::default());
    after.props.space_before = Some(0.0);
    after.props.space_after = Some(0.0);
    document.body = vec![Arc::new(Block::Table(table)), para_block(after)];
    let result = layout(&document, &mut LayoutCache::new(), &LayoutOptions::default());
    assert_eq!(result.pages.len(), 2);
    result
        .pages
        .iter()
        .map(|page| {
            let text_y = page
                .items
                .iter()
                .find_map(|item| match item {
                    Placed::Lines { path, y, .. } if path.0.len() == 4 && path.0[2] == 0 => Some(*y),
                    _ => None,
                })
                .expect("The row's first cell text must be present");
            let cell_bottom = page
                .items
                .iter()
                .find_map(|item| match item {
                    Placed::Cell { table, cell: 0, rect, .. } if table.0 == [0] => Some(rect.bottom()),
                    _ => None,
                })
                .expect("The row's first cell box must be present");
            let frame_bottom = page
                .items
                .iter()
                .filter_map(|item| match item {
                    Placed::Rule { x0, y0, x1, y1, border } if (x1 - x0).abs() >= 239.0 && (y1 - y0).abs() < 0.01 => Some(*y0 + border.width / 2.0),
                    _ => None,
                })
                .max_by(f32::total_cmp);
            PageGeometry { text_y, cell_bottom, frame_bottom }
        })
        .collect()
}

fn close(actual: f32, expected: f32) {
    assert!((actual - expected).abs() <= 0.06, "observed relative geometry {actual}, expected Word geometry {expected}");
}

#[test]
fn later_rows_start_a_new_page_with_half_the_initial_outer_gap() {
    let zero = geometry(0.0);
    for spacing in [1.5, 3.0] {
        let current = geometry(spacing);
        close(current[0].text_y - zero[0].text_y, 2.0 * spacing);
        close(current[1].text_y - zero[1].text_y, spacing);
    }
}

#[test]
fn exact_rows_keep_the_observed_one_point_end_difference_between_pages() {
    for spacing in [0.0, 0.75, 1.5, 3.0] {
        let pages = geometry(spacing);
        // PDF cell-bottom outer edges are279.075/280.075 on the two pages
        // for every spacing value. Relative coordinates cancel PDF origin/rounding.
        close(pages[1].cell_bottom - pages[0].cell_bottom, 1.0);
    }
}

#[test]
fn a_page_before_later_rows_closes_with_half_gap_and_the_final_page_with_full_gap() {
    for spacing in [1.5, 3.0] {
        let pages = geometry(spacing);
        close(pages[0].frame_bottom.expect("The first page needs an outer bottom frame") - pages[0].cell_bottom, spacing + 1.0);
        close(pages[1].frame_bottom.expect("The final page needs an outer bottom frame") - pages[1].cell_bottom, 2.0 * spacing + 1.0);
    }
}
