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

fn control(outer_top: f32, header_top: Option<f32>, inside_h: f32, spacing: f32, bundle: bool, shaded: bool) -> DocLayout {
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
        row.props.header = ri == 0;
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
            Placed::Rule { x0, y0, x1, y1, border } if (100.0..116.0).contains(&(x1 - x0).abs()) && (y1 - y0).abs() < 0.01 && *y0 < header_y => {
                Some((*y0 - border.width / 2.0, *y0 + border.width / 2.0, border.width))
            }
            _ => None,
        })
        .collect()
}

fn observed_h(inside_h: f32, bundle: bool, expected: Option<(f32, f32)>) {
    let result = control(0.0, None, inside_h, 1.5, bundle, false);
    let rules = header_top_rules(&result);
    if let Some((top, bottom)) = expected {
        assert_eq!(rules.len(), 2, "Word paints two cell top horizontals: {rules:?}");
        for (a, b, width) in rules {
            assert!(
                (a - top).abs() <= 0.06 && (b - bottom).abs() <= 0.06 && (width - (bottom - top)).abs() <= 0.06,
                "Word physical top/bottom {top}/{bottom}; native {a}/{b}, width{width}"
            );
        }
    } else {
        assert!(rules.is_empty(), "Explicit nil insideH leaves no cell horizontal: {rules:?}");
    }
}

#[test]
fn outer_nil_cell_top_paints_inside_h_one_at_38_to_39() {
    observed_h(1.0, false, Some((38.0, 39.0)));
}
#[test]
fn outer_nil_cell_top_paints_inside_h_two_at_37_to_39() {
    observed_h(2.0, false, Some((37.0, 39.0)));
}
#[test]
fn outer_nil_and_inside_h_nil_do_not_invent_cell_top() {
    observed_h(0.0, false, None);
}
#[test]
fn doubling_competing_bands_keeps_inside_h_one() {
    observed_h(1.0, true, Some((38.0, 39.0)));
}

#[test]
fn inside_h_paint_changes_do_not_move_first_header_paragraph() {
    // Word A1 Tm y307.58, glyphtop43.42, Calibri12, all three variants.
    // Native D y307.351562 is separately recorded, not substituted for Word.
    // This relative contract prevents a paint fix from reserving extra own_top.
    let base = control(0.0, None, 0.0, 1.5, false, false);
    let y = text_y(&base, &[0, 0, 0, 0]);
    for h in [1.0, 2.0] {
        let other = control(0.0, None, h, 1.5, false, false);
        assert!((text_y(&other, &[0, 0, 0, 0]) - y).abs() <= 0.06, "insideH{h} changed the header paragraph position");
    }
}

fn observed_v(outer: f32, spacing: f32, fill_top: f32, fill_bottom: f32) {
    let result = control(outer, Some(0.0), 1.0, spacing, false, true);
    // These Word shading-only controls preserve every character/Tm/font/size/
    // bbox/page assignment exactly versus their unshaded parent. The union of
    // actual colored paint is observed, not a guessed nominal Word CellRect.
    let header_y = text_y(&result, &[0, 0, 0, 0]);
    let body_y = text_y(&result, &[0, 1, 0, 0]);
    let rects: Vec<_> = result.pages[0]
        .items
        .iter()
        .filter_map(|it| match it {
            Placed::Cell { table, row: 0, rect, .. } if table.0 == [0] => Some(*rect),
            _ => None,
        })
        .collect();
    assert_eq!(rects.len(), 2);
    let left = rects.iter().map(|r| r.x).min_by(f32::total_cmp).unwrap();
    let right = rects.iter().map(|r| r.right()).max_by(f32::total_cmp).unwrap();
    let fills: Vec<_> = result.pages[0]
        .items
        .iter()
        .filter_map(|it| match it {
            Placed::Fill { rect, color } if *color == Rgb(255, 224, 224) || *color == Rgb(224, 255, 224) => Some(*rect),
            _ => None,
        })
        .collect();
    assert_eq!(fills.len(), 2);
    for f in &fills {
        assert!(
            (f.y - fill_top).abs() <= 0.06 && (f.bottom() - fill_bottom).abs() <= 0.06,
            "Do not translate/resize fill to imitate side ink: Word{fill_top}..{fill_bottom}, native{}..{}",
            f.y,
            f.bottom()
        );
    }
    let sides: Vec<_> = result.pages[0]
        .items
        .iter()
        .filter_map(|it| match it {
            Placed::Rule { x0, y0, x1, y1, .. }
                if (x1 - x0).abs() < 0.01 && *x0 >= left - 0.06 && *x0 <= right + 0.06 && *y1 > header_y && *y1 < body_y =>
            {
                Some((*x0, *y0, *y1))
            }
            _ => None,
        })
        .collect();
    assert_eq!(sides.len(), 4, "Exclude the two outer-frame sides");
    // Direct tcTop nil overrides insideH1: no cellular horizontal may appear.
    assert!(header_top_rules(&result).is_empty());
    for (_, a, b) in sides {
        // All three Word controls: vertical ink starts35.995 (~outertop36)
        // and ends exactly at fill_bottom; its top does NOT follow fill_top.
        assert!((a - 36.0).abs() <= 0.06 && (b - fill_bottom).abs() <= 0.06, "Word side36..{fill_bottom}; native{a}..{b}; top AND bottom must match");
        for f in &fills {
            assert!((b - f.bottom()).abs() <= 0.06, "Side must stop at fill bottom, not one border band later");
        }
    }
}

#[test]
fn direct_nil_header_side_endpoints_and_fill_spacing_one_half_outer_one() {
    observed_v(1.0, 1.5, 39.995, 58.620);
}
#[test]
fn direct_nil_header_side_endpoints_and_fill_spacing_three_outer_one() {
    observed_v(1.0, 3.0, 42.995, 61.620);
}
#[test]
fn direct_nil_header_side_endpoints_and_fill_spacing_one_half_outer_two() {
    observed_v(2.0, 1.5, 40.995, 59.620);
}

fn fallback_side_endpoints(inside_h: f32, bundle: bool, expected_top: f32, expected_bottom: f32) {
    let result = control(0.0, None, inside_h, 1.5, bundle, false);
    let reference = control(0.0, None, 1.0, 1.5, bundle, false);
    let header_y = text_y(&result, &[0, 0, 0, 0]);
    let body_y = text_y(&result, &[0, 1, 0, 0]);
    assert!((header_y - text_y(&reference, &[0, 0, 0, 0])).abs() <= 0.06, "Fallback paint must not move the header text");
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
    assert_eq!(paths(&result), paths(&reference), "Every text paragraph, including repeated headers, remains present");
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
    let sides: Vec<_> = result.pages[0]
        .items
        .iter()
        .filter_map(|item| match item {
            Placed::Rule { x0, y0, x1, y1, .. }
                if (x1 - x0).abs() < 0.01 && *x0 >= left - 0.06 && *x0 <= right + 0.06 && *y1 > header_y && *y1 < body_y =>
            {
                Some((*y0, *y1))
            }
            _ => None,
        })
        .collect();
    assert_eq!(sides.len(), 4, "Compare the four cellular sides, excluding the outer frame");
    for (top, bottom) in sides {
        assert!(
            (top - expected_top).abs() <= 0.06 && (bottom - expected_bottom).abs() <= 0.06,
            "Measured Word fallback side {expected_top}..{expected_bottom}; native {top}..{bottom}; retain both endpoints without offset"
        );
    }
}

#[test]
fn fallback_nil_inside_h_sides_join_outer_start_and_measured_end() {
    // Existing nil-ink-h-insideHnil Word page1 ink; no new oracle.
    fallback_side_endpoints(0.0, false, 35.995, 59.120);
}

#[test]
fn fallback_inside_h_two_sides_stop_before_reserved_bottom_band() {
    // Existing nil-ink-h-insideH2; the horizontal37..39 has its own guard.
    fallback_side_endpoints(2.0, false, 38.995, 57.620);
}

#[test]
fn fallback_competing_bands_two_sides_keep_measured_cell_end() {
    // Existing nil-ink-h-competing-bands2; outer/vertical widths are2pt.
    fallback_side_endpoints(1.0, true, 38.995, 57.620);
}
