//! Public AutoFit behavior with effective spacing inherited through a table-style chain.
//! Whole-word expectations use measured native glyph minima, not a copied solver.
use std::sync::Arc;

use serde_json::json;
use wordcraft_doc::props::{Border, Borders, LineSpacing};
use wordcraft_doc::styles::{Style, StyleKind, TableStyleParts};
use wordcraft_doc::{Block, Document, Paragraph, Path, Pos, StoryRef, Table, para_block};
use wordcraft_engine::{Selection, Session};
use wordcraft_layout::Placed;

#[test]
fn contents_autofit_with_inherited_spacing_keeps_unequal_words_whole() {
    let mut doc = Document::new();
    doc.settings.compat_mode = 15;
    doc.last_section.page_w = 612.0;
    doc.last_section.margin_left = 72.0;
    doc.last_section.margin_right = 72.0;
    doc.styles.default_chr.font = Some("Calibri".into());
    doc.styles.default_chr.size = Some(12.0);
    doc.styles.default_para.space_before = Some(0.0);
    doc.styles.default_para.space_after = Some(0.0);
    doc.styles.default_para.line_spacing = Some(LineSpacing::Multiple(1.0));
    doc.styles.styles.push(Style {
        id: "SpacingContentsBase".into(),
        kind: StyleKind::Table,
        table: Some(TableStyleParts { cell_spacing: Some(1.5), ..Default::default() }),
        ..Default::default()
    });
    doc.styles.styles.push(Style {
        id: "SpacingContentsDerived".into(),
        kind: StyleKind::Table,
        based_on: Some("SpacingContentsBase".into()),
        table: Some(TableStyleParts::default()),
        ..Default::default()
    });
    let mut table = Table::new(1, 2, 240.0);
    table.props.style = Some("SpacingContentsDerived".into());
    table.props.fixed = true;
    table.props.width = Some(240.0);
    table.props.cell_spacing = None;
    table.props.cell_margins = Some([0.0; 4]);
    table.props.borders = Some(Borders::all(Border::single(0.0)));
    for (cell, text) in table.rows[0].cells.iter_mut().zip(["MMMM", "LONGWORDLONGWORDLONGWORD"]) {
        cell.blocks = vec![para_block(Paragraph::with_text(text, Default::default()))];
    }
    let mut unspaced = table.clone();
    unspaced.props.style = None;
    let minimums = wordcraft_layout::measure_table_columns(&doc, &unspaced);
    assert!(minimums[1].0 > minimums[0].0);
    assert!(minimums.iter().map(|c| c.1).sum::<f32>() + 9.0 < doc.last_section.text_width());
    doc.body = vec![Arc::new(Block::Table(table)), para_block(Paragraph::new())];
    let mut session = Session::new(doc);
    session.sel = Selection::caret(Pos { story: StoryRef::Body, path: Path(vec![0, 0, 0, 0]), off: 0 });
    session.run("table.autofit", &json!({"mode": "contents"})).unwrap();
    let layout = session.layout();
    for (cell, minimum) in minimums.iter().enumerate() {
        let rect = layout
            .pages
            .iter()
            .flat_map(|page| &page.items)
            .find_map(|item| match item {
                Placed::Cell { table, row: 0, cell: ci, rect, .. } if table.0 == [0] && *ci == cell => Some(*rect),
                _ => None,
            })
            .expect("AutoFit must retain both cells");
        assert!(
            rect.w + 0.06 >= minimum.0,
            "Inherited spacing must not shrink cell{cell} below its measured word minimum: {} < {}",
            rect.w,
            minimum.0
        );
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
        assert_eq!(lines, 1, "Inherited-spacing AutoFit must keep the complete word on one line when the page has room");
    }
}
