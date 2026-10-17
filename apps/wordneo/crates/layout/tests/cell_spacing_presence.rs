//! Word distinguishes omitted spacing from present zero, directly or in a table style.
//! Source evidence: table-spacing008/runs/*205150* and *205426*.
use std::sync::Arc;

use wordcraft_doc::props::{Border, Borders};
use wordcraft_doc::styles::{Style, StyleKind, TableStyleParts};
use wordcraft_doc::{Block, Document, Paragraph, Table, para_block};
use wordcraft_layout::{LayoutCache, LayoutOptions, Placed, layout};

fn positions(direct: Option<f32>, inherited: Option<f32>) -> Vec<(f32, f32)> {
    let mut document = Document::new();
    document.settings.compat_mode = 15;
    document.last_section.margin_top = 72.0;
    document.last_section.margin_left = 72.0;
    document.styles.default_chr.font = Some("Calibri".into());
    document.styles.default_chr.size = Some(12.0);
    document.styles.default_para.space_before = Some(0.0);
    document.styles.default_para.space_after = Some(0.0);
    let mut table = Table::new(2, 2, 240.0);
    table.props.style = None;
    table.props.width = Some(240.0);
    table.props.fixed = true;
    table.props.cell_margins = Some([2.0; 4]);
    table.props.cell_spacing = direct;
    table.props.borders = Some(Borders::all(Border::single(2.0)));
    if let Some(spacing) = inherited {
        document.styles.styles.push(Style {
            id: "Spacing008Probe".into(),
            kind: StyleKind::Table,
            table: Some(TableStyleParts { cell_spacing: Some(spacing), ..Default::default() }),
            ..Default::default()
        });
        table.props.style = Some("Spacing008Probe".into());
    }
    for row in &mut table.rows {
        for cell in &mut row.cells {
            cell.blocks = vec![para_block(Paragraph::with_text("Hjg", Default::default()))];
        }
    }
    document.body = vec![Arc::new(Block::Table(table)), para_block(Paragraph::with_text("AFTER", Default::default()))];
    let result = layout(&document, &mut LayoutCache::new(), &LayoutOptions::default());
    assert_eq!(result.pages.len(), 1);
    [&[0, 0, 0, 0][..], &[0, 1, 0, 0][..], &[1][..]]
        .iter()
        .map(|wanted| {
            result.pages[0]
                .items
                .iter()
                .find_map(|item| match item {
                    Placed::Lines { path, x, y, .. } if path.0.as_slice() == *wanted => Some((*x, *y)),
                    _ => None,
                })
                .expect("All presence-control markers must be laid out")
        })
        .collect()
}

fn close(actual: f32, expected: f32) {
    assert!((actual - expected).abs() <= 0.06, "measured delta {actual}, expected Word delta {expected}");
}

#[test]
fn explicit_zero_spacing_keeps_the_table_frame_separate_from_cell_frames() {
    let omitted = positions(None, None);
    let present = positions(Some(0.0), None);
    close(present[0].0 - omitted[0].0, 1.0);
    close(present[0].1 - omitted[0].1, 2.0);
    close(present[1].1 - omitted[1].1, 4.0);
    close(present[2].1 - omitted[2].1, 6.0);
    close(present[2].0 - omitted[2].0, 0.0);
}

#[test]
fn inherited_zero_activates_the_same_mode_and_direct_zero_overrides_positive_spacing() {
    let omitted = positions(None, None);
    let direct_zero = positions(Some(0.0), None);
    let inherited_zero = positions(None, Some(0.0));
    let overriding_zero = positions(Some(0.0), Some(0.75));
    close(inherited_zero[0].1 - omitted[0].1, 2.0);
    for (actual, expected) in inherited_zero.iter().zip(&direct_zero).chain(overriding_zero.iter().zip(&direct_zero)) {
        close(actual.0, expected.0);
        close(actual.1, expected.1);
    }
}
