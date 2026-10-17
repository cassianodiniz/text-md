//! Public geometry from the existing repeat-header-split Word oracle only.
//! Prepared outside SOURCE while the probeC build is frozen. No formula for
//! other border widths, gaps or multiple header rows is asserted here.
use std::sync::Arc;

use wordcraft_doc::props::{Align, Border, BorderStyle, Borders, CharProps, LineSpacing, Rgb, VAlign};
use wordcraft_doc::{Block, Document, Paragraph, Table, para_block};
use wordcraft_layout::{DocLayout, LayoutCache, LayoutOptions, Placed, layout};

fn paragraph(text: &str, cell: bool) -> Paragraph {
    // The input DOCX has direct run font/size, no paragraph-mark rPr and
    // Aptos12 document defaults. Do not change the document font to Calibri.
    let mut p = Paragraph::with_text(
        text,
        CharProps {
            font: Some("Calibri".into()),
            font_cs: if cell { Some("Calibri".into()) } else { None },
            font_ea: if cell { Some("Calibri".into()) } else { None },
            size: Some(12.0),
            size_cs: if cell { Some(12.0) } else { None },
            ..Default::default()
        },
    );
    p.mark = CharProps::default();
    p.props.space_before = Some(0.0);
    p.props.space_after = Some(0.0);
    if cell {
        p.props.align = Some(Align::Left);
        p.props.line_spacing = Some(LineSpacing::Multiple(1.0));
    }
    p
}

fn control() -> DocLayout {
    control_with_top(1.0, None)
}

fn control_with_top(outer_top: f32, header_top: Option<f32>) -> DocLayout {
    let mut doc = Document::new();
    doc.settings.compat_mode = 15;
    doc.last_section.page_w = 360.0;
    doc.last_section.page_h = 360.0;
    doc.last_section.margin_top = 36.0;
    doc.last_section.margin_bottom = 36.0;
    doc.last_section.margin_left = 36.0;
    doc.last_section.margin_right = 36.0;
    doc.last_section.header = 36.0;
    doc.last_section.footer = 36.0;
    let mut table = Table::new(3, 2, 240.0);
    table.props.style = None;
    table.props.width = Some(240.0);
    table.props.fixed = true;
    table.props.align = Some(Align::Left);
    table.props.indent = Some(0.0);
    table.props.cell_spacing = Some(1.5);
    table.props.cell_margins = Some([2.0; 4]);
    let mut borders = Borders::all(Border::single(1.0));
    borders.top = Some(if outer_top == 0.0 {
        Border { style: BorderStyle::None, width: 0.0, color: Some(Rgb(0, 0, 0)), space: 0.0 }
    } else {
        Border::single(outer_top)
    });
    table.props.borders = Some(borders);
    for (ri, row) in table.rows.iter_mut().enumerate() {
        row.props.header = ri == 0;
        for (ci, cell) in row.cells.iter_mut().enumerate() {
            cell.props.width = Some(120.0);
            cell.props.valign = VAlign::Top;
            cell.props.hide_mark = false;
            if ri == 0
                && let Some(top) = header_top
            {
                cell.props.borders = Some(Borders { top: Some(Border::single(top)), ..Default::default() });
            }
            let count = if ri == 1 { 45 } else { 1 };
            cell.blocks = (0..count)
                .map(|pi| para_block(paragraph(&format!("{}{:02}-{:02}", if ci == 0 { 'A' } else { 'B' }, ri + 1, pi + 1), true)))
                .collect();
        }
    }
    doc.body = vec![Arc::new(Block::Table(table)), para_block(paragraph("SPACING-AFTER", false))];
    let result = layout(&doc, &mut LayoutCache::new(), &LayoutOptions::default());
    assert_eq!(result.pages.len(), 3, "The measured Word control has three pages");
    result
}

struct Geometry {
    header_y: f32,
    first_body_y: f32,
    body_top_center: f32,
    frame_bottom: f32,
}

fn geometry(result: &DocLayout) -> Vec<Geometry> {
    geometry_with_distribution(result, [(0, 17), (17, 34), (34, 45)])
}

fn geometry_with_distribution(result: &DocLayout, distribution: [(u32, u32); 3]) -> Vec<Geometry> {
    result
        .pages
        .iter()
        .zip(distribution)
        .map(|(page, (start, end))| {
            let header_y = page
                .items
                .iter()
                .find_map(|item| match item {
                    Placed::Lines { path, y, .. } if path.0 == [0, 0, 0, 0] => Some(*y),
                    _ => None,
                })
                .expect("The measured repeated header must be present");
            let mut body: Vec<(u32, f32)> = page
                .items
                .iter()
                .filter_map(|item| match item {
                    Placed::Lines { path, y, .. } if path.0.len() == 4 && path.0[..3] == [0, 1, 0] => Some((path.0[3], *y)),
                    _ => None,
                })
                .collect();
            body.sort_by_key(|(paragraph, _)| *paragraph);
            assert_eq!(body.iter().map(|(paragraph, _)| *paragraph).collect::<Vec<_>>(), (start..end).collect::<Vec<u32>>());
            let cell_top = page
                .items
                .iter()
                .find_map(|item| match item {
                    Placed::Cell { table, row: 1, cell: 0, rect, .. } if table.0 == [0] => Some(rect.y),
                    _ => None,
                })
                .expect("The first body cell must be present");
            let body_top_center = page
                .items
                .iter()
                .filter_map(|item| match item {
                    Placed::Rule { x0, y0, x1, y1, .. }
                        if (110.0..120.0).contains(&(x1 - x0).abs())
                            && (y1 - y0).abs() < 0.01
                            && *y0 >= cell_top - 0.06
                            && *y0 <= cell_top + 1.06 =>
                    {
                        Some(*y0)
                    }
                    _ => None,
                })
                .min_by(f32::total_cmp)
                .expect("The body's top horizontal cell border must be drawn");
            let frame_bottom = page
                .items
                .iter()
                .filter_map(|item| match item {
                    Placed::Rule { x0, y0, x1, y1, border } if (x1 - x0).abs() >= 239.0 && (y1 - y0).abs() < 0.01 => Some(*y0 + border.width / 2.0),
                    _ => None,
                })
                .max_by(f32::total_cmp)
                .expect("The outer frame must close on every page");
            Geometry { header_y, first_body_y: body[0].1, body_top_center, frame_bottom }
        })
        .collect()
}

#[test]
fn repeated_header_continuations_keep_the_observed_extra_one_point_before_body_text() {
    let pages = geometry(&control());
    // Word: header45.42 on every page; first body69.02/70.02/70.02.
    // Same text font: differences of origins cancel its glyph-top offset.
    let initial_distance = pages[0].first_body_y - pages[0].header_y;
    for page in &pages[1..] {
        // Raw Word rectangles: body top ink63.62..64.62 on every page.
        // Thus the extra point belongs inside the continuation; shifting the
        // whole cell or its top border would introduce another mismatch.
        assert!((page.body_top_center - pages[0].body_top_center).abs() <= 0.06, "The body-cell top border must stay in place");
        assert!((page.header_y - pages[0].header_y).abs() <= 0.06, "Do not translate the repeated header to compensate");
        let delta = page.first_body_y - page.header_y - initial_distance;
        assert!((delta - 1.0).abs() <= 0.06, "Word continuation increases body-header distance by1pt; native{delta}");
    }
}

#[test]
fn equally_populated_header_pages_keep_the_observed_one_point_bottom_frame_difference() {
    let pages = geometry(&control());
    // Existing PDF: pages1/2 each contain17 body paragraphs. Word outer ink
    // ends321.275/322.275. Comparing pages cancels common line/font metrics.
    let delta = pages[1].frame_bottom - pages[0].frame_bottom;
    assert!((delta - 1.0).abs() <= 0.06, "Word second-page frame ends1pt below first; native{delta}");
}

fn assert_continuation_inset(pages: &[Geometry], expected: f32) {
    let initial_distance = pages[0].first_body_y - pages[0].header_y;
    for page in &pages[1..] {
        assert!((page.header_y - pages[0].header_y).abs() <= 0.06, "Repeated header must stay physically in place");
        assert!((page.body_top_center - pages[0].body_top_center).abs() <= 0.06, "Body top border must stay physically in place");
        let delta = page.first_body_y - page.header_y - initial_distance;
        assert!((delta - expected).abs() <= 0.06, "Measured Word continuation inset{expected}pt; native{delta}");
    }
}

#[test]
fn nil_outer_table_top_has_no_extra_continuation_inset() {
    // New Word oracle: outer-top nil, inherited header top nil, body top1.
    // Header43.42; first body67.02 on all pages; counts17/17/11.
    let pages = geometry(&control_with_top(0.0, None));
    assert_continuation_inset(&pages, 0.0);
}

#[test]
fn outer_table_top_two_with_direct_header_top_one_has_two_point_inset_and_measured_distribution() {
    // Pair parent is outer-top2; direct header top1 breaks the inheritance.
    // Word header46.42, body70.02/72.02/72.02, counts17/16/12.
    let pages = geometry_with_distribution(&control_with_top(2.0, Some(1.0)), [(0, 17), (17, 33), (33, 45)]);
    assert_continuation_inset(&pages, 2.0);
}

#[test]
fn direct_header_top_two_does_not_replace_the_one_point_outer_top_inset() {
    // Word outer-top1/header-top2: header46.42 on every page, body70.02
    // then71.02/71.02; the body's top ink64.62..65.62 stays fixed.
    let pages = geometry(&control_with_top(1.0, Some(2.0)));
    assert_continuation_inset(&pages, 1.0);
}
