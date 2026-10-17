//! The final three Word controls discriminate asymmetric closing bands and fill.
//! Evidence: independent-core-review/runs/word-20261016T2130434393299Z-5bdad9ce.
//! Tests compare physical rules/fill separately from nominal cell advance.
use std::sync::Arc;

use wordcraft_doc::props::{Border, Borders, LineSpacing, Rgb};
use wordcraft_doc::{Block, Document, Paragraph, Table, para_block};
use wordcraft_geom::Rect;
use wordcraft_layout::{DocLayout, LayoutCache, LayoutOptions, Placed, layout};

fn render(left: f32, right: f32, shading: bool) -> DocLayout {
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
    table.props.cell_margins = Some([0.0; 4]);
    table.props.borders = Some(Borders::all(Border::single(1.0)));
    for (ci, cell) in table.rows[0].cells.iter_mut().enumerate() {
        if left != 1.0 || right != 1.0 {
            cell.props.borders = Some(Borders { left: Some(Border::single(left)), right: Some(Border::single(right)), ..Default::default() });
        }
        if shading {
            cell.props.shading = Some(if ci == 0 { Rgb(255, 224, 224) } else { Rgb(224, 255, 224) });
        }
        let mut p = Paragraph::with_text("A", Default::default());
        p.props.line_spacing = Some(LineSpacing::Multiple(1.0));
        cell.blocks = vec![para_block(p)];
    }
    doc.body = vec![Arc::new(Block::Table(table)), para_block(Paragraph::with_text("SPACING-AFTER", Default::default()))];
    let result = layout(&doc, &mut LayoutCache::new(), &LayoutOptions::default());
    assert_eq!(result.pages.len(), 1);
    result
}

fn first_cell(result: &DocLayout) -> Rect {
    result.pages[0]
        .items
        .iter()
        .find_map(|item| match item {
            Placed::Cell { table, row: 0, cell: 0, rect, .. } if table.0 == [0] => Some(*rect),
            _ => None,
        })
        .expect("The first nominal cell must be present")
}

fn close(actual: f32, expected: f32, label: &str) {
    assert!((actual - expected).abs() <= 0.06, "{label}: native{actual}, Word{expected}");
}

fn check_asymmetric(left: f32, right: f32, expected_left_padding: f32) {
    let result = render(left, right, false);
    let cell = first_cell(&result);
    close(cell.w, 4.0, "nominal grid-derived advance");
    let mut strokes: Vec<(f32, f32)> = result.pages[0]
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
    strokes.sort_by(|a, b| a.0.total_cmp(&b.0));
    assert_eq!(strokes.len(), 2);
    let text_x = result.pages[0]
        .items
        .iter()
        .find_map(|item| match item {
            Placed::Lines { path, x, .. } if path.0 == [0, 0, 0, 0] => Some(*x),
            _ => None,
        })
        .expect("The first A marker must be present");
    // Word preserves the existing left-center/text relation. Only closing
    // geometry is wrong in the old implementation; do not move both edges.
    close(text_x - strokes[0].0, expected_left_padding, "text minus left stroke center");
    close(strokes[1].0 - strokes[0].0, 2.5, "distance between asymmetric border centers");
    close(strokes[1].0 + strokes[1].1 / 2.0 - strokes[0].0 + strokes[0].1 / 2.0, 4.0, "outer cell ink extent");
}

#[test]
fn left_one_right_two_borders_keep_the_observed_centers_and_ink() {
    check_asymmetric(1.0, 2.0, 0.5);
}

#[test]
fn left_two_right_one_borders_keep_the_observed_centers_and_ink() {
    check_asymmetric(2.0, 1.0, 1.0);
}

#[test]
fn shaded_one_point_bordered_cell_paints_only_the_observed_inner_fill() {
    let result = render(1.0, 1.0, true);
    let cell = first_cell(&result);
    let fill = result.pages[0]
        .items
        .iter()
        .find_map(|item| match item {
            Placed::Fill { rect, color } if *color == Rgb(255, 224, 224) => Some(*rect),
            _ => None,
        })
        .expect("The first cell's red shading must be present");
    // Word red fill85.525..86.525; first left stroke center85.025.
    close(fill.x - cell.x, 0.5, "fill starts inside left border");
    close(fill.w, 1.0, "fill width between border ink");
    // Preserve the already observed top/bottom clipping while fixing x/width.
    close(fill.y - cell.y, 1.0, "fill starts below the top border");
    close(cell.bottom() - fill.bottom(), 1.0, "fill ends above the bottom border");
}
