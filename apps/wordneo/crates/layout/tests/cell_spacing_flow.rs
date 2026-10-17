//! Word black-box nested sizing and page-frame controls prepared before these guards.
//! Source: table-spacing008/runs/word-20261016T2103021502782Z-b8ba4a62.
use std::sync::Arc;

use wordcraft_doc::props::{Border, Borders, HeightRule, LineSpacing};
use wordcraft_doc::{Block, Document, Paragraph, Table, para_block};
use wordcraft_layout::{DocLayout, LayoutCache, LayoutOptions, Placed, layout};

fn document() -> Document {
    let mut doc = Document::new();
    doc.settings.compat_mode = 15;
    doc.styles.default_chr.font = Some("Calibri".into());
    doc.styles.default_chr.size = Some(12.0);
    doc.styles.default_para.space_before = Some(0.0);
    doc.styles.default_para.space_after = Some(0.0);
    doc
}

fn automatic_table(rows: usize, cols: usize, margins: f32) -> Table {
    let mut table = Table::new(rows, cols, 240.0);
    table.props.style = None;
    table.props.cell_margins = Some([margins; 4]);
    table.props.borders = Some(Borders::all(Border::single(0.0)));
    table
}

fn cell_paragraph(text: &str) -> Arc<Block> {
    let mut paragraph = Paragraph::with_text(text, Default::default());
    // Match the control's direct w:spacing line240/auto. The body AFTER paragraph
    // retains its separate inherited line-spacing setting.
    paragraph.props.line_spacing = Some(LineSpacing::Multiple(1.0));
    para_block(paragraph)
}

fn nested(spacing: Option<f32>) -> (wordcraft_geom::Rect, Vec<wordcraft_geom::Rect>) {
    let mut doc = document();
    let mut outer = automatic_table(1, 1, 0.0);
    let mut inner = automatic_table(1, 2, 2.0);
    // The Word controls omit every nested tcW. Table::new otherwise supplies
    // preferred cell widths, which would exercise the preferred-grid path.
    for table in [&mut outer, &mut inner] {
        for row in &mut table.rows {
            for cell in &mut row.cells {
                cell.props.width = None;
                cell.props.width_pct = None;
            }
        }
    }
    inner.props.cell_spacing = spacing;
    for (cell, marker) in inner.rows[0].cells.iter_mut().zip(["A1", "B1"]) {
        cell.blocks = vec![cell_paragraph(marker)];
    }
    outer.rows[0].cells[0].blocks = vec![Arc::new(Block::Table(inner)), cell_paragraph("")];
    doc.body = vec![Arc::new(Block::Table(outer))];
    let result = layout(&doc, &mut LayoutCache::new(), &LayoutOptions::default());
    assert_eq!(result.pages.len(), 1);
    let mut parent = None;
    let mut children = Vec::new();
    for item in &result.pages[0].items {
        if let Placed::Cell { table, rect, .. } = item {
            if table.0 == [0] {
                parent = Some(*rect);
            } else if table.0 == [0, 0, 0, 0] {
                children.push(*rect);
            }
        }
    }
    children.sort_by(|a, b| a.x.total_cmp(&b.x));
    assert_eq!(children.len(), 2);
    (parent.expect("An outer cell must be laid out"), children)
}

#[test]
fn nested_automatic_parent_grows_to_include_its_childs_three_spacing_gaps() {
    let (baseline, baseline_cells) = nested(None);
    let (zero, _) = nested(Some(0.0));
    let (positive, cells) = nested(Some(1.5));
    assert!((zero.w - baseline.w).abs() <= 0.06, "Word's nil-border absent and zero nested controls have identical width");
    assert!((positive.w - baseline.w - 9.0).abs() <= 0.06, "Word's parent gray box grows 33.7 to42.7pt, reserving three3pt gaps");
    assert!((cells[0].x - baseline_cells[0].x - 3.0).abs() <= 0.06);
    assert!((cells[1].x - baseline_cells[1].x - 6.0).abs() <= 0.06);
    assert!(
        cells[1].right() + 3.0 <= positive.right() + 0.06,
        "the parent's automatic width must contain the whole child including its trailing gap"
    );
}

fn paged_table(whole_rows: bool, repeated_header: bool) -> DocLayout {
    let mut doc = document();
    doc.last_section.page_w = 360.0;
    doc.last_section.page_h = 360.0;
    doc.last_section.margin_top = 36.0;
    doc.last_section.margin_bottom = 36.0;
    doc.last_section.margin_left = 36.0;
    doc.last_section.margin_right = 36.0;
    let rows = if whole_rows {
        2
    } else if repeated_header {
        3
    } else {
        1
    };
    let mut table = automatic_table(rows, 2, 2.0);
    table.props.width = Some(240.0);
    table.props.fixed = true;
    table.props.cell_spacing = Some(1.5);
    table.props.borders = Some(Borders::all(Border::single(1.0)));
    for (ri, row) in table.rows.iter_mut().enumerate() {
        if whole_rows {
            row.props.height = Some(240.0);
            row.props.height_rule = HeightRule::Exact;
            row.props.cant_split = true;
        }
        row.props.header = repeated_header && ri == 0;
        for (ci, cell) in row.cells.iter_mut().enumerate() {
            let count = if !whole_rows && (!repeated_header || ri == 1) { 45 } else { 1 };
            cell.blocks = (0..count).map(|pi| cell_paragraph(&format!("{}{:02}-{:02}", if ci == 0 { 'A' } else { 'B' }, ri + 1, pi + 1))).collect();
        }
    }
    doc.body = vec![Arc::new(Block::Table(table)), para_block(Paragraph::with_text("SPACING-AFTER", Default::default()))];
    layout(&doc, &mut LayoutCache::new(), &LayoutOptions::default())
}

fn assert_page_frames(result: &DocLayout, expected_pages: usize) {
    assert_eq!(result.pages.len(), expected_pages);
    for (pi, page) in result.pages.iter().enumerate() {
        let mut frame_edges: Vec<f32> = page
            .items
            .iter()
            .filter_map(|item| match item {
                Placed::Rule { x0, y0, x1, y1, .. } if (y0 - y1).abs() < 0.01 && (x1 - x0).abs() >= 239.0 => Some(*y0),
                _ => None,
            })
            .collect();
        frame_edges.sort_by(f32::total_cmp);
        frame_edges.dedup_by(|a, b| (*a - *b).abs() < 0.06);
        assert_eq!(frame_edges.len(), 2, "Word closes and reopens the complete table frame on page{}: {frame_edges:?}", pi + 1);
    }
}

#[test]
fn whole_rows_close_and_reopen_the_separate_table_frame_at_a_page_boundary() {
    assert_page_frames(&paged_table(true, false), 2);
}

#[test]
fn split_rows_keep_two_outer_table_frame_edges_on_every_page_with_or_without_headers() {
    for header in [false, true] {
        assert_page_frames(&paged_table(false, header), 3);
    }
}
