//! Literal Word narrow-table observations, not constants computed by the solver.
//! Sources: independent-core-review/runs/word-*212007*-4370759c and word-*212156*-77b794ed.
use std::sync::Arc;

use wordcraft_doc::props::{Border, Borders, LineSpacing};
use wordcraft_doc::{Block, Document, Paragraph, Table, para_block};
use wordcraft_layout::{LayoutCache, LayoutOptions, Placed, layout};

#[test]
fn fixed_narrow_tables_expand_to_the_observed_cell_and_outer_frame_widths() {
    // border, margin, Word cell-box width, Word grid sum, Word outer ink extent.
    for (border, margin, box_width, footprint, ink_width) in
        [(0.0, 0.0, 1.0, 38.0, None), (1.0, 0.0, 3.0, 42.0, Some(43.0)), (2.0, 0.0, 5.0, 46.0, Some(48.0)), (1.0, 2.0, 7.0, 50.0, Some(51.0))]
    {
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
        table.props.cell_margins = Some([margin; 4]);
        table.props.borders = Some(Borders::all(Border::single(border)));
        for cell in &mut table.rows[0].cells {
            let mut p = Paragraph::with_text("A", Default::default());
            p.props.line_spacing = Some(LineSpacing::Multiple(1.0));
            cell.blocks = vec![para_block(p)];
        }
        doc.body = vec![Arc::new(Block::Table(table)), para_block(Paragraph::with_text("SPACING-AFTER", Default::default()))];
        let result = layout(&doc, &mut LayoutCache::new(), &LayoutOptions::default());
        assert_eq!(result.pages.len(), 1);
        let mut cells: Vec<_> = result.pages[0]
            .items
            .iter()
            .filter_map(|item| match item {
                Placed::Cell { table, rect, .. } if table.0 == [0] => Some(*rect),
                _ => None,
            })
            .collect();
        cells.sort_by(|a, b| a.x.total_cmp(&b.x));
        assert_eq!(cells.len(), 2);
        for cell in &cells {
            assert!((cell.w - box_width).abs() <= 0.06, "border{border}/margin{margin}: Word cell width{box_width}, actual{}", cell.w);
        }
        // The two outside gaps are12pt each. This also checks the nil-border
        // case, where the PDF cannot expose an invisible table outline.
        let extent_from_cells = cells[1].right() - cells[0].x + 24.0;
        assert!(
            (extent_from_cells - footprint).abs() <= 0.06,
            "border{border}/margin{margin}: Word grid sum{footprint}, box/gap extent{extent_from_cells}"
        );
        if let Some(ink_width) = ink_width {
            let actual = result.pages[0]
                .items
                .iter()
                .filter_map(|item| match item {
                    Placed::Rule { x0, x1, y0, y1, border } if (y1 - y0).abs() < 0.01 => Some((x1 - x0).abs() + border.width),
                    _ => None,
                })
                .max_by(f32::total_cmp)
                .expect("A visible table frame must be present");
            assert!((actual - ink_width).abs() <= 0.06, "border{border}/margin{margin}: Word outer ink extent{ink_width}, actual{actual}");
        }
    }
}
