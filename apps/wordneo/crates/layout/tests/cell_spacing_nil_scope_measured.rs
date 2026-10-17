//! Page1 ink contracts from already measured Word controls. Outside SOURCE.
//! No new rendering/oracle execution; no claim about unrelated nil borders.
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

fn control(
    outer_top: f32,
    header_top: Option<f32>,
    inside_h: f32,
    spacing: f32,
    bundle: bool,
    shaded: bool,
    (repeat_header, header_bottom): (bool, Option<f32>),
) -> DocLayout {
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
    table.props.cell_spacing = Some(spacing);
    table.props.cell_margins = Some([2.0; 4]);
    let mut borders = Borders::all(Border::single(1.0));
    borders.top = Some(if outer_top == 0.0 {
        Border { style: BorderStyle::None, width: 0.0, color: Some(Rgb(0, 0, 0)), space: 0.0 }
    } else {
        Border::single(outer_top)
    });
    borders.between = Some(if inside_h == 0.0 {
        Border { style: BorderStyle::None, width: 0.0, color: Some(Rgb(0, 0, 0)), space: 0.0 }
    } else {
        Border::single(inside_h)
    });
    if bundle {
        borders.left = Some(Border::single(2.0));
        borders.right = Some(Border::single(2.0));
        borders.bottom = Some(Border::single(2.0));
        borders.inside_v = Some(Border::single(2.0));
    }
    table.props.borders = Some(borders);
    for (ri, row) in table.rows.iter_mut().enumerate() {
        row.props.header = ri == 0 && repeat_header;
        for (ci, cell) in row.cells.iter_mut().enumerate() {
            cell.props.width = Some(120.0);
            cell.props.valign = VAlign::Top;
            cell.props.hide_mark = false;
            if shaded && ri == 0 {
                cell.props.shading = Some(if ci == 0 { Rgb(255, 224, 224) } else { Rgb(224, 255, 224) });
            }
            if ri == 0
                && let Some(top) = header_top
            {
                cell.props.borders = Some(Borders {
                    top: Some(if top == 0.0 {
                        Border { style: BorderStyle::None, width: 0.0, color: Some(Rgb(0, 0, 0)), space: 0.0 }
                    } else {
                        Border::single(top)
                    }),
                    ..Default::default()
                });
            }
            if ri == 0
                && let Some(bottom) = header_bottom
            {
                cell.props.borders.get_or_insert_with(Borders::default).bottom = Some(if bottom == 0.0 {
                    Border { style: BorderStyle::None, width: 0.0, color: Some(Rgb(0, 0, 0)), space: 0.0 }
                } else {
                    Border::single(bottom)
                });
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

fn text_y(result: &DocLayout, path_wanted: &[u32]) -> f32 {
    result.pages[0]
        .items
        .iter()
        .find_map(|item| match item {
            Placed::Lines { path, y, .. } if path.0.as_slice() == path_wanted => Some(*y),
            _ => None,
        })
        .expect("Measured marker paragraph must remain present")
}

fn header_top_rules(result: &DocLayout) -> Vec<(f32, f32, f32)> {
    let header_y = text_y(result, &[0, 0, 0, 0]);
    result.pages[0]
        .items
        .iter()
        .filter_map(|item| match item {
            Placed::Rule { x0, y0, x1, y1, border } if (100.0..121.0).contains(&(x1 - x0).abs()) && (y1 - y0).abs() < 0.01 && *y0 < header_y => {
                Some((*y0 - border.width / 2.0, *y0 + border.width / 2.0, border.width))
            }
            _ => None,
        })
        .collect()
}

#[test]
fn first_row_without_header_flag_keeps_inside_h_horizontal_paint() {
    let base = control(0.0, None, 1.0, 1.5, false, false, (true, None));
    let result = control(0.0, None, 1.0, 1.5, false, false, (false, None));
    // Word page1 is exactly invariant (all chars/Tms/font/size/bboxes), not
    // merely the first marker. Later paragraphs redistribute17/18/10.
    assert!((text_y(&base, &[0, 0, 0, 0]) - text_y(&result, &[0, 0, 0, 0])).abs() <= 0.06);
    let rules = header_top_rules(&result);
    assert_eq!(rules.len(), 2);
    for (top, bottom, width) in rules {
        assert!(
            (top - 38.0).abs() <= 0.06 && (bottom - 39.0).abs() <= 0.06 && (width - 1.0).abs() <= 0.06,
            "Word first-row H38..39 with no header flag; native{top}..{bottom}, width{width}"
        );
    }
}

fn measured_vertical_case(
    repeat: bool,
    bottom: Option<f32>,
    spacing: f32,
    expected_fill_top: f32,
    expected_fill_bottom: f32,
    expected_side_bottom: f32,
    expected_body_delta: f32,
) {
    let parent = control(1.0, Some(0.0), 1.0, spacing, false, true, (true, None));
    let result = control(1.0, Some(0.0), 1.0, spacing, false, true, (repeat, bottom));
    assert!((text_y(&parent, &[0, 0, 0, 0]) - text_y(&result, &[0, 0, 0, 0])).abs() <= 0.06, "First header text remains invariant");
    assert!(
        (text_y(&result, &[0, 1, 0, 0]) - text_y(&parent, &[0, 1, 0, 0]) - expected_body_delta).abs() <= 0.06,
        "Preserve the measured body displacement from actual bottom-band advance"
    );
    let header_y = text_y(&result, &[0, 0, 0, 0]);
    let body_y = text_y(&result, &[0, 1, 0, 0]);
    let cells: Vec<_> = result.pages[0]
        .items
        .iter()
        .filter_map(|it| match it {
            Placed::Cell { table, row: 0, rect, .. } if table.0 == [0] => Some(*rect),
            _ => None,
        })
        .collect();
    assert_eq!(cells.len(), 2);
    let left = cells.iter().map(|r| r.x).min_by(f32::total_cmp).unwrap();
    let right = cells.iter().map(|r| r.right()).max_by(f32::total_cmp).unwrap();
    let fills: Vec<_> = result.pages[0]
        .items
        .iter()
        .filter_map(|it| match it {
            Placed::Fill { rect, color } if *color == Rgb(255, 224, 224) || *color == Rgb(224, 255, 224) => Some(*rect),
            _ => None,
        })
        .collect();
    assert_eq!(fills.len(), 2);
    // Word actual colored paint remains39.995..58.620 for bottom1/2/nil;
    // the raw bounds do NOT define a nominal structural Word CellRect.
    for f in &fills {
        assert!(
            (f.y - expected_fill_top).abs() <= 0.06 && (f.bottom() - expected_fill_bottom).abs() <= 0.06,
            "Preserve real fill, independent of side paint: {}..{}",
            f.y,
            f.bottom()
        );
    }
    // At explicitspacing0, the outer right side lies inside the nominal
    // CellRect envelope. Exclude it by the public full-width outer top's
    // endpoints, rather than miscounting it as a fifth cellular side.
    let outer = result.pages[0]
        .items
        .iter()
        .find_map(|it| match it {
            Placed::Rule { x0, y0, x1, y1, .. } if (x1 - x0).abs() >= 239.0 && (y1 - y0).abs() < 0.01 => Some((*x0, *x1)),
            _ => None,
        })
        .expect("Measured outer table top remains visible");
    let sides: Vec<_> = result.pages[0]
        .items
        .iter()
        .filter_map(|it| match it {
            Placed::Rule { x0, y0, x1, y1, .. }
                if (x1 - x0).abs() < 0.01
                    && *x0 >= left - 0.06
                    && *x0 <= right + 0.06
                    && *y1 > header_y
                    && *y1 < body_y
                    && (*x0 - outer.0).abs() > 0.06
                    && (*x0 - outer.1).abs() > 0.06 =>
            {
                Some((*y0, *y1))
            }
            _ => None,
        })
        .collect();
    assert_eq!(sides.len(), 4);
    assert!(header_top_rules(&result).is_empty(), "Explicit tcTopnil continues to override insideH1");
    for (a, b) in sides {
        assert!(
            (a - 36.0).abs() <= 0.06 && (b - expected_side_bottom).abs() <= 0.06,
            "Word side36..{expected_side_bottom}; native{a}..{b}, both endpoints matter"
        );
    }
}

#[test]
fn first_row_without_header_flag_keeps_direct_nil_top_vertical_endpoints() {
    measured_vertical_case(false, None, 1.5, 39.995, 58.620, 58.620, 0.0);
}
#[test]
fn direct_bottom_two_keeps_vertical_end_at_fill_end() {
    measured_vertical_case(true, Some(2.0), 1.5, 39.995, 58.620, 58.620, 1.0);
}
#[test]
fn direct_bottom_nil_extends_vertical_past_fill_by_measured_one_half() {
    // Literal measured Word endpoint, not a claimed gap/2 rule. Scaling of
    // this1.5pt extension is now measured again at spacing3 below.
    measured_vertical_case(true, Some(0.0), 1.5, 39.995, 58.620, 60.120, -1.0);
}

#[test]
fn nil_bottom_spacing_three_extends_sides_three_beyond_fill() {
    // Word measured35.995..64.620 versus fill42.995..61.620.
    // Together with spacing1.5, this supports half-gap scaling in this family.
    measured_vertical_case(true, Some(0.0), 3.0, 42.995, 61.620, 64.620, -1.0);
}
#[test]
fn explicit_zero_spacing_outer_nil_keeps_inside_h_one_at_35_to_36() {
    let result = control(0.0, None, 1.0, 0.0, false, false, (true, None));
    // Zero remains Some(0); it is never a collapsed None fixture.
    let rules = header_top_rules(&result);
    assert_eq!(rules.len(), 2);
    for (a, b, width) in rules {
        assert!(
            (a - 35.0).abs() <= 0.06 && (b - 36.0).abs() <= 0.06 && (width - 1.0).abs() <= 0.06,
            "Word explicitzero H35..36; native{a}..{b}, width{width}"
        );
    }
}
#[test]
fn explicit_zero_spacing_direct_nil_top_still_joins_outer_and_fill_bottom() {
    measured_vertical_case(true, None, 0.0, 36.995, 55.620, 55.620, 0.0);
}

fn fallback_scoped_sides(repeat_header: bool, spacing: f32, expected_top: f32, expected_bottom: f32) {
    let result = control(0.0, None, 1.0, spacing, false, false, (repeat_header, None));
    let reference = control(0.0, None, 0.0, spacing, false, false, (repeat_header, None));
    let header_y = text_y(&result, &[0, 0, 0, 0]);
    let body_y = text_y(&result, &[0, 1, 0, 0]);
    assert!((header_y - text_y(&reference, &[0, 0, 0, 0])).abs() <= 0.06, "Fallback side ink must not reserve extra text space");
    let paths = |layout: &DocLayout| {
        let mut paths: Vec<_> = layout
            .pages
            .iter()
            .flat_map(|page| &page.items)
            .filter_map(|item| match item {
                Placed::Lines { path, .. } => Some(path.0.clone()),
                _ => None,
            })
            .collect();
        paths.sort();
        paths
    };
    assert_eq!(paths(&result), paths(&reference), "Do not lose text or accumulate repeated header paragraphs");
    assert!(!result.pages[0].items.iter().any(|item| matches!(item, Placed::Fill { .. })), "The measured unshaded control must not acquire fills");
    let rects: Vec<_> = result.pages[0]
        .items
        .iter()
        .filter_map(|item| match item {
            Placed::Cell { table, row: 0, rect, .. } if table.0 == [0] => Some(*rect),
            _ => None,
        })
        .collect();
    assert_eq!(rects.len(), 2);
    let left = rects.iter().map(|r| r.x).min_by(f32::total_cmp).unwrap();
    let right = rects.iter().map(|r| r.right()).max_by(f32::total_cmp).unwrap();
    // At explicit spacing0 the outer right side can lie inside CellRect.
    // Reuse measured_vertical_case's full-width edge exclusion; here the
    // page-closing edge remains visible even when the outer top is nil.
    let outer = result.pages[0]
        .items
        .iter()
        .find_map(|item| match item {
            Placed::Rule { x0, y0, x1, y1, .. } if (x1 - x0).abs() >= 239.0 && (y1 - y0).abs() < 0.01 => Some((*x0, *x1)),
            _ => None,
        })
        .expect("Measured full-width outer edge remains visible");
    let sides: Vec<_> = result.pages[0]
        .items
        .iter()
        .filter_map(|item| match item {
            Placed::Rule { x0, y0, x1, y1, .. }
                if (x1 - x0).abs() < 0.01
                    && *x0 >= left - 0.06
                    && *x0 <= right + 0.06
                    && *y1 > header_y
                    && *y1 < body_y
                    && (*x0 - outer.0).abs() > 0.06
                    && (*x0 - outer.1).abs() > 0.06 =>
            {
                Some((*y0, *y1))
            }
            _ => None,
        })
        .collect();
    assert_eq!(sides.len(), 4, "Exclude the two outer-frame sides");
    for (top, bottom) in sides {
        assert!(
            (top - expected_top).abs() <= 0.06 && (bottom - expected_bottom).abs() <= 0.06,
            "Measured Word fallback side {expected_top}..{expected_bottom}; native {top}..{bottom}; no compensating offset"
        );
    }
}

#[test]
fn fallback_without_header_flag_keeps_measured_side_endpoints() {
    // Existing nil-scope-h-no-header, same first-page geometry as its parent.
    fallback_scoped_sides(false, 1.5, 38.995, 57.620);
}

#[test]
fn fallback_explicit_zero_spacing_keeps_measured_side_endpoints() {
    // Existing nil-zero-h-spacing0; Some(0) must remain separated from None.
    fallback_scoped_sides(true, 0.0, 35.995, 54.620);
}
