//! Independent review guards for the separated-cell layout path.
//! These are native behavioral contracts, not newly measured Word constants.

use std::sync::Arc;

use serde_json::json;
use wordcraft_doc::props::{Border, Borders, LineSpacing};
use wordcraft_doc::{Block, Document, Paragraph, Path, Pos, StoryRef, Table, para_block};
use wordcraft_engine::{Selection, Session};
use wordcraft_layout::Placed;

fn fixture(width: f32, spacing: f32, border: f32) -> (Document, Table) {
    let mut doc = Document::new();
    doc.settings.compat_mode = 15;
    doc.last_section.page_w = 612.0;
    doc.last_section.page_h = 792.0;
    doc.last_section.margin_left = 72.0;
    doc.last_section.margin_right = 72.0;
    doc.last_section.margin_top = 72.0;
    doc.last_section.margin_bottom = 72.0;
    doc.styles.default_chr.font = Some("Calibri".into());
    doc.styles.default_chr.size = Some(12.0);
    doc.styles.default_para.space_before = Some(0.0);
    doc.styles.default_para.space_after = Some(0.0);
    doc.styles.default_para.line_spacing = Some(LineSpacing::Multiple(1.0));
    let mut table = Table::new(1, 2, width);
    table.props.style = None;
    table.props.width = Some(width);
    table.props.fixed = true;
    table.props.cell_spacing = Some(spacing);
    table.props.cell_margins = Some([0.0; 4]);
    table.props.borders = Some(Borders::all(Border::single(border)));
    (doc, table)
}

fn session(mut doc: Document, table: Table) -> Session {
    doc.body = vec![Arc::new(Block::Table(table)), para_block(Paragraph::new())];
    let mut session = Session::new(doc);
    session.sel = Selection::caret(Pos { story: StoryRef::Body, path: Path(vec![0, 0, 0, 0]), off: 0 });
    session
}

#[test]
fn contents_autofit_with_spacing_keeps_unequal_single_words_on_one_line() {
    let (doc, mut table) = fixture(240.0, 1.5, 0.0);
    for (cell, text) in table.rows[0].cells.iter_mut().zip(["MMMM", "LONGWORDLONGWORDLONGWORD"]) {
        cell.blocks = vec![para_block(Paragraph::with_text(text, Default::default()))];
    }
    // Measure real glyphs through the public layout API, without a guessed 40/80pt fixture.
    let mut unspaced = table.clone();
    unspaced.props.cell_spacing = None;
    let content = wordcraft_layout::measure_table_columns(&doc, &unspaced);
    assert!(content[1].0 > content[0].0, "The fixture must exercise unequal word widths");
    assert!(content.iter().map(|c| c.1).sum::<f32>() + 9.0 < doc.last_section.text_width(), "Both words and all three gaps fit on the page");
    let mut session = session(doc, table);
    session.run("table.autofit", &json!({"mode": "contents"})).unwrap();
    let layout = session.layout();
    for (cell, minimum) in content.iter().enumerate() {
        let rect = layout
            .pages
            .iter()
            .flat_map(|page| &page.items)
            .find_map(|item| match item {
                Placed::Cell { table, row: 0, cell: ci, rect, .. } if table.0 == [0] && *ci == cell => Some(*rect),
                _ => None,
            })
            .expect("AutoFit must retain both cells");
        assert!(rect.w + 0.06 >= minimum.0, "AutoFit shrank cell{cell} below its measured single-word minimum: {} < {}", rect.w, minimum.0);
        let path = vec![0, 0, cell as u32, 0];
        let lines = layout
            .pages
            .iter()
            .flat_map(|page| &page.items)
            .find_map(|item| match item {
                Placed::Lines { path: actual, para, .. } if actual.0 == path => Some(para.lines.len()),
                _ => None,
            })
            .expect("Both words must be laid out");
        assert_eq!(lines, 1, "AutoFit must not wrap a single word when both columns fit the page");
    }
}

#[test]
fn narrow_separated_table_frame_contains_its_cell_boxes_even_when_gaps_exceed_the_preference() {
    for spacing in [0.0, 6.0] {
        let (doc, mut table) = fixture(20.0, spacing, 1.0);
        for cell in &mut table.rows[0].cells {
            cell.blocks = vec![para_block(Paragraph::with_text("A", Default::default()))];
        }
        let mut session = session(doc, table);
        let layout = session.layout();
        let frame = layout
            .pages
            .iter()
            .flat_map(|page| &page.items)
            .find_map(|item| match item {
                Placed::Rule { x0, x1, y0, y1, border } if (y1 - y0).abs() < 0.01 && (x1 - x0).abs() >= 19.9 => {
                    Some((x0.min(*x1) - border.width / 2.0, x0.max(*x1) + border.width / 2.0))
                }
                _ => None,
            })
            .expect("The synthetic table has a visible outer horizontal frame");
        let cells: Vec<_> = layout
            .pages
            .iter()
            .flat_map(|page| &page.items)
            .filter_map(|item| match item {
                Placed::Cell { table, rect, .. } if table.0 == [0] => Some(*rect),
                _ => None,
            })
            .collect();
        assert_eq!(cells.len(), 2);
        for cell in cells {
            assert!(cell.x.is_finite() && cell.right().is_finite() && cell.w >= 0.0);
            assert!(
                cell.x >= frame.0 - 0.06 && cell.right() <= frame.1 + 0.06,
                "spacing{spacing}: cell box {cell:?} lies outside the table footprint {frame:?}"
            );
        }
    }
}
