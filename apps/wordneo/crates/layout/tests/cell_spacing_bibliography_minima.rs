//! Reproduction of probeA008's exported synthetic IEEE table.
//! Evidence: bibliography-investigation/candidate008/runs/
//! 20261016T214937679488Z-8a534ff5/initial.{docx,json,pdf}.
//! Preserve tblW100%, first-cell tcW1%, automatic content cells and0.75pt spacing/padding.
use std::sync::Arc;

use wordcraft_doc::props::CharProps;
use wordcraft_doc::styles::Style;
use wordcraft_doc::{Block, Document, Paragraph, Table, para_block};
use wordcraft_layout::{LayoutCache, LayoutOptions, Placed, layout, measure_table_columns};

fn fixture() -> (Document, Table) {
    let mut doc = Document::new();
    doc.settings.compat_mode = 15;
    doc.last_section.page_w = 612.0;
    doc.last_section.page_h = 792.0;
    doc.last_section.margin_left = 72.0;
    doc.last_section.margin_right = 72.0;
    doc.last_section.margin_top = 72.0;
    doc.last_section.margin_bottom = 72.0;
    // Same exported default style and Bibliografia->Normal inheritance as probeA.
    doc.styles.default_chr.font = Some("Aptos".into());
    doc.styles.default_chr.size = Some(12.0);
    doc.styles.upsert(Style { id: "Bibliografia".into(), name: "Bibliografia".into(), based_on: Some("Normal".into()), ..Default::default() });
    let mut table = Table::new(3, 2, 468.0);
    table.props.style = None;
    table.props.width = None;
    table.props.width_pct = Some(100.0);
    table.props.fixed = false;
    table.props.borders = None;
    table.props.cell_margins = Some([0.75; 4]);
    table.props.cell_spacing = Some(0.75);
    table.grid = vec![4.68, 463.32];
    let sources = [
        "T. Exemplo, \"Synthetic Article for WordNeo Audit,\" Fictional Journal, vol. 7, no. 2, pp. 101-109, 14 09 2031.",
        "L. Inventada and R. Exemplo, Synthetic Atlas for WordNeo Audit, 3 ed., vol. 2, N. Ficticio, Ed., Imaginary City: Fictional Press, 2030, pp. 11-29.",
        "C. Inventado, Uncited Source Must Survive, 3 ed., vol. 2, N. Ficticio, Ed., Imaginary City: Fictional Press, 2032, pp. 11-29.",
    ];
    for (ri, row) in table.rows.iter_mut().enumerate() {
        for (ci, cell) in row.cells.iter_mut().enumerate() {
            cell.props.width = None;
            cell.props.width_pct = if ci == 0 { Some(1.0) } else { None };
            cell.props.hide_mark = true;
        }
        row.cells[0].blocks = vec![para_block(
            Paragraph::with_text(&format!("[{}] ", ri + 1), CharProps { no_proof: Some(true), ..Default::default() }).styled("Bibliografia"),
        )];
        let mut content = Paragraph::new().styled("Bibliografia");
        if ri == 0 {
            // Match the exported article's19-character italic journal run.
            content.insert_text(0, &sources[ri][..51], &CharProps { no_proof: Some(true), ..Default::default() }).unwrap();
            content
                .insert_text(
                    content.len(),
                    &sources[ri][51..70],
                    &CharProps { no_proof: Some(true), italic: Some(true), italic_cs: Some(true), ..Default::default() },
                )
                .unwrap();
            content.insert_text(content.len(), &sources[ri][70..], &CharProps { no_proof: Some(true), ..Default::default() }).unwrap();
        } else {
            content.insert_text(0, sources[ri], &CharProps { no_proof: Some(true), ..Default::default() }).unwrap();
        }
        row.cells[1].blocks = vec![para_block(content)];
    }
    (doc, table)
}

#[test]
fn percentage_bibliography_labels_remain_on_one_line_with_the_exported_one_percent_preference() {
    let (mut doc, table) = fixture();
    doc.body = vec![Arc::new(Block::Table(table)), para_block(Paragraph::new())];
    let result = layout(&doc, &mut LayoutCache::new(), &LayoutOptions::default());
    for row in 0..3 {
        let lines = result
            .pages
            .iter()
            .flat_map(|page| &page.items)
            .find_map(|item| match item {
                Placed::Lines { path, para, .. } if path.0 == [0, row, 0, 0] => Some(para.lines.len()),
                _ => None,
            })
            .expect("Every bibliography label must be laid out");
        assert_eq!(lines, 1, "IEEE label[{}] must not split its closing bracket onto another line", row + 1);
    }
}

#[test]
fn preferred_nonfixed_columns_keep_their_real_text_minima_after_reserving_cell_spacing() {
    let (mut doc, table) = fixture();
    let mut unspaced = table.clone();
    unspaced.props.cell_spacing = None;
    let minima = measure_table_columns(&doc, &unspaced);
    assert!(minima.iter().map(|c| c.0).sum::<f32>() + 4.5 < doc.last_section.text_width());
    doc.body = vec![Arc::new(Block::Table(table)), para_block(Paragraph::new())];
    let result = layout(&doc, &mut LayoutCache::new(), &LayoutOptions::default());
    for (column, minimum) in minima.iter().enumerate() {
        let cell = result
            .pages
            .iter()
            .flat_map(|page| &page.items)
            .find_map(|item| match item {
                Placed::Cell { table, row: 0, cell, rect, .. } if table.0 == [0] && *cell == column => Some(*rect),
                _ => None,
            })
            .expect("Both bibliography columns must be present");
        assert!(
            cell.w + 0.06 >= minimum.0,
            "preferred nonfixed column{column} fell below its measured text minimum after spacing: {} < {}",
            cell.w,
            minimum.0
        );
    }
}
