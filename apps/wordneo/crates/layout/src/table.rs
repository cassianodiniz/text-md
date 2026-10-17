//! Table layout: column widths, cell boxes, row heights, merged cells, borders and shading,
//! table-style conditional formatting. Rows are laid out as free-standing boxes; pagination
//! moves whole rows or splits them between lines (`split_row`).

use std::collections::HashMap;

use wordcraft_doc::numbering::Counters;
use wordcraft_doc::props::{Align, Border, BorderStyle, Borders, HeightRule, Rgb, TextDirection, VAlign, VMerge};
use wordcraft_doc::styles::{TableStyleParts, TableStyleProps};
use wordcraft_doc::{Block, Blocks, Paragraph, Path, StoryRef, Table};
use wordcraft_geom::Rect;

use crate::para::{CellText, ClKind, ParaLayout};
use crate::{BoxLayout, Ctx, Placed, layout_box};

#[derive(Clone)]
pub struct RowLayout {
    pub height: f32,
    /// Items relative to the table's top-left (x) and the row's top (y).
    pub items: Vec<Placed>,
    pub(crate) bands: RowBands,
}

#[derive(Clone, Default)]
pub(crate) struct RowBands {
    top: f32,
    own_top: f32,
    margin_top: f32,
    margin_bottom: f32,
    bottom: f32,
    bottom_in_advance: bool,
    /// Present spacing (including explicit zero) separates table and cell frames.
    separated: bool,
    /// Non-cell space around this row; retained when a row is fragmented.
    leading: f32,
    trailing: f32,
    table_frame: Option<TableFrame>,
    outer_rules: Vec<usize>,
    frames: Vec<usize>,
    top_rules: Vec<usize>,
    bottom_rules: Vec<usize>,
    pub(crate) fills: Vec<usize>,
    pub(crate) paragraph_ends: HashMap<Path, f32>,
}

#[derive(Clone)]
struct TableFrame {
    left: f32,
    right: f32,
    borders: Borders,
    gap: f32,
    first: bool,
    last: bool,
    exact: bool,
}

fn border_band(border: Option<Border>) -> f32 {
    border.filter(Border::is_visible).map_or(0.0, |border| border.width.clamp(0.0, 12.0))
}

impl RowLayout {
    /// A closing border outside the row's advance still needs room at a page edge.
    pub(crate) fn closing_band(&self) -> f32 {
        if let Some(frame) = &self.bands.table_frame {
            let crop = if frame.exact && !frame.last { self.bands.bottom } else { 0.0 };
            return (frame.gap / 2.0 + border_band(frame.borders.bottom) - self.bands.trailing - crop).max(0.0);
        }
        if self.bands.bottom_in_advance { 0.0 } else { self.bands.bottom }
    }

    pub(crate) fn has_separate_frame(&self) -> bool {
        self.bands.table_frame.is_some()
    }

    /// Reopen the table around a whole row or continuation already shaped above.
    /// A repeated header supplies this outer top itself.
    pub(crate) fn at_page_start(&self, repeated_header: bool) -> Option<Self> {
        let frame = self.bands.table_frame.as_ref()?;
        if repeated_header || frame.first {
            return None;
        }
        let mut row = self.clone();
        let leading = border_band(frame.borders.top) + frame.gap / 2.0;
        let delta = leading - row.bands.leading;
        for (i, item) in row.items.iter_mut().enumerate() {
            if row.bands.outer_rules.contains(&i) {
                if let Placed::Rule { y0, y1, .. } = item {
                    if (*y1 - *y0).abs() > 0.01 {
                        *y1 += delta;
                    } else if row.bands.bottom_rules.contains(&i) {
                        *y0 += delta;
                        *y1 += delta;
                    }
                }
            } else {
                item.translate(0.0, delta);
            }
        }
        row.height += delta;
        row.bands.top += delta;
        row.bands.own_top += delta;
        row.bands.leading = leading;
        for end in row.bands.paragraph_ends.values_mut() {
            *end += delta;
        }
        let has_top = row.bands.outer_rules.iter().any(|i| row.bands.top_rules.contains(i));
        if !has_top && let Some(border) = frame.borders.top.filter(Border::is_visible) {
            let y = border_band(Some(border)) / 2.0;
            row.bands.top_rules.push(row.items.len());
            row.bands.outer_rules.push(row.items.len());
            row.items.push(Placed::Rule { x0: frame.left, y0: y, x1: frame.right, y1: y, border });
        }
        Some(row)
    }

    /// A continuation under a repeated header reserves the table's outer top
    /// band inside its cell. Its top border stays fixed; only its content and
    /// bottom grow. This is not the header cell's own top or bottom border.
    pub(crate) fn after_repeated_header(&self) -> Option<Self> {
        let frame = self.bands.table_frame.as_ref()?;
        let inset = border_band(frame.borders.top);
        if inset <= 0.0 {
            return None;
        }
        let mut row = self.clone();
        for (i, item) in row.items.iter_mut().enumerate() {
            if row.bands.frames.contains(&i) {
                match item {
                    Placed::Cell { rect, .. } => rect.h += inset,
                    Placed::Rule { y1, .. } => *y1 += inset,
                    _ => {}
                }
            } else if row.bands.fills.contains(&i) {
                if let Placed::Fill { rect, .. } = item {
                    rect.h += inset;
                }
            } else if !row.bands.top_rules.contains(&i) {
                item.translate(0.0, inset);
            }
        }
        row.height += inset;
        row.bands.top += inset;
        // Keep own_top unchanged: split_row removes this physical inset when
        // it normalizes the next continuation from first_moved. Each page
        // receives its own band, rather than accumulating it on page three.
        for end in row.bands.paragraph_ends.values_mut() {
            *end += inset;
        }
        Some(row)
    }

    /// Close a physical page without changing the logical row's already shaped
    /// paragraphs. Interior boundaries use half a gap; the table's last row
    /// retains its full final gap, even when that row itself is fragmented.
    pub(crate) fn at_page_end(&self) -> Option<Self> {
        let frame = self.bands.table_frame.as_ref()?;
        if frame.last {
            return None;
        }
        let mut row = self.clone();
        // Exact rows keep the cell's closing band in their contiguous advance;
        // Word omits that band from the earlier page's physical cell box.
        let crop = if frame.exact { row.bands.bottom } else { 0.0 };
        let trailing = frame.gap / 2.0 + border_band(frame.borders.bottom);
        let delta = trailing - row.bands.trailing - crop;
        for (i, item) in row.items.iter_mut().enumerate() {
            if row.bands.outer_rules.contains(&i) {
                if let Placed::Rule { y0, y1, .. } = item
                    && (*y1 - *y0).abs() > 0.01
                {
                    *y1 += delta;
                }
            } else if row.bands.frames.contains(&i) {
                match item {
                    Placed::Cell { rect, .. } => rect.h = (rect.h - crop).max(0.0),
                    Placed::Rule { y1, .. } => *y1 -= crop,
                    _ => {}
                }
            } else if row.bands.fills.contains(&i) {
                if let Placed::Fill { rect, .. } = item {
                    rect.h = (rect.h - crop).max(0.0);
                }
            } else if row.bands.bottom_rules.contains(&i) {
                item.translate(0.0, -crop);
            }
        }
        row.height += delta;
        row.bands.trailing = trailing;
        if let Some(border) = frame.borders.bottom.filter(Border::is_visible) {
            let y = row.height - border_band(Some(border)) / 2.0;
            row.bands.bottom_rules.push(row.items.len());
            row.bands.outer_rules.push(row.items.len());
            row.items.push(Placed::Rule { x0: frame.left, y0: y, x1: frame.right, y1: y, border });
        }
        Some(row)
    }

    pub(crate) fn bottom_band(&self) -> f32 {
        self.bands.bottom
    }

    /// Change only the physical predecessor of a whole row. The inherited band
    /// is cropped from its top, including Exact rows; existing alignment slack,
    /// shaped text and numbering remain unchanged. Nested content moves as content.
    pub(crate) fn after_boundary(&self, previous_bottom: f32) -> Option<Self> {
        if self.bands.separated {
            // Separated cells keep both of their own borders; there is no collapsed
            // border to replace with the previous row's bottom band.
            return None;
        }
        let top = self.bands.own_top.max(previous_bottom);
        let crop = self.bands.top - top;
        if crop.abs() < 0.01 {
            return None;
        }
        let mut items = self.items.clone();
        for (i, item) in items.iter_mut().enumerate() {
            if self.bands.frames.binary_search(&i).is_ok() {
                match item {
                    Placed::Cell { rect, .. } => rect.h = (rect.h - crop).max(0.0),
                    Placed::Rule { y1, .. } => *y1 = (*y1 - crop).max(0.0),
                    _ => {}
                }
            } else if self.bands.top_rules.binary_search(&i).is_err() {
                item.translate(0.0, -crop);
            }
        }
        let mut bands = self.bands.clone();
        bands.top = top;
        for end in bands.paragraph_ends.values_mut() {
            *end -= crop;
        }
        Some(Self { height: (self.height - crop).max(0.0), items, bands })
    }
}

pub struct TableLayout {
    /// Table x offset within the column.
    pub x: f32,
    pub width: f32,
    pub rows: Vec<RowLayout>,
}

const DEFAULT_MARGINS: [f32; 4] = [0.0, 5.4, 0.0, 5.4];

/// The longest line turned text gets when its row height isn't exact (a Letter page's body
/// height): longer text wraps.
const TURNED_MAX: f32 = 648.0;
/// The longest line turned text ever gets (Word's largest page, 22 inches).
const TURNED_LIMIT: f32 = 1584.0;

/// A cell whose text is turned, between the two passes of [`layout_table`].
struct Turned {
    turn: TextDirection,
    /// Line length it was laid out with, and how far its lines reach across the cell.
    len: f32,
    across: f32,
    /// List counters and equation count before the cell, to lay it out again the same way.
    snap: (Counters, u32),
    /// Its formatting region (header row, total row, first column, last column, row band, column
    /// band) and cell index in the row.
    region: Region,
    ci: usize,
    /// Top margin plus top border band.
    top: f32,
}

/// How long the lines laid out in `items` are without the room they didn't use: the length
/// turned text needs so that nothing wraps.
fn natural_length(items: &[Placed]) -> f32 {
    let mut len = 0.0f32;
    for it in items {
        let Placed::Lines { para, l0, l1, x, .. } = it else { continue };
        let indents = para.rp.indent_left.max(0.0) + para.rp.indent_right.max(0.0) + para.rp.indent_first.max(0.0);
        for l in para.lines.get(*l0..*l1).unwrap_or(&[]) {
            let (mut lo, mut hi) = (f32::MAX, f32::MIN);
            for v in &l.vis {
                lo = lo.min(v.x);
                hi = hi.max(v.x + v.w);
            }
            if l.vis.is_empty()
                && let (Some(a), Some(b)) = (l.xs.first(), l.xs.last())
            {
                (lo, hi) = (*a, *b);
            }
            let hy = l.hyphen.map_or(0.0, |h| h.2);
            if hi >= lo {
                len = len.max(x.max(0.0) + hi - lo + hy + indents);
            }
        }
    }
    // A little slack, so laying the text out again at this length doesn't wrap it.
    len + 0.5
}

/// The table's style with its based-on chain merged.
fn table_style(ctx: &Ctx, t: &Table) -> Option<TableStyleProps> {
    ctx.doc.styles.table_style(t.props.style.as_deref()?)
}

/// The table's default cell margins: its own, else its style's, else Word's.
fn default_margins(t: &Table, style: Option<&TableStyleProps>) -> [f32; 4] {
    t.props.cell_margins.or(style.and_then(|s| s.parts.cell_margins)).unwrap_or(DEFAULT_MARGINS)
}

/// Presence matters: Word renders an explicitly specified zero (including a
/// style's zero) with separate table/cell frames. Omission retains collapsed borders.
fn effective_spacing(t: &Table, style: Option<&TableStyleProps>) -> Option<f32> {
    t.props.cell_spacing.or(style.and_then(|s| s.parts.cell_spacing)).map(|s| if s.is_finite() { s.clamp(0.0, 1584.0) } else { 0.0 })
}

/// Reserve physical gaps inside a preferred fixed-width table. The one-point
/// content allowance is empirical (Word synthetic width/margin controls); sub-twip
/// differences against Word's serialized grid/PDF remain separately tracked.
fn separated_widths(t: &Table, style: Option<&TableStyleProps>, grid: &mut [f32], gap: f32, text_minima: Option<&[(f32, f32)]>) {
    if gap <= 0.0 || grid.is_empty() {
        return;
    }
    let defaults = default_margins(t, style);
    let mut minima: Vec<Option<f32>> = vec![None; grid.len()];
    let mut physical_minima: Vec<Option<f32>> = vec![None; grid.len()];
    for (ri, row) in t.rows.iter().enumerate() {
        let mut g = 0usize;
        for cell in &row.cells {
            let span = cell.span();
            if span == 1
                && let Some(minimum) = minima.get_mut(g)
            {
                let m = cell.props.margins.unwrap_or(defaults);
                let value = m[1].max(0.0) + m[3].max(0.0) + 1.0;
                *minimum = Some(minimum.unwrap_or(0.0).max(value));
                // Word's narrow fixed-table controls retain the full two cell
                // border bands as well as the declared padding and1pt allowance.
                // Keep these physical floors separate from the empirical weights
                // used to reconcile preferred grids at ordinary widths.
                let (left, right) = horizontal_insets(t, style, ri, g, cell, grid.len());
                let border_bands = 2.0 * (left + right - m[1].max(0.0) - m[3].max(0.0)).max(0.0);
                let physical = physical_minima[g].unwrap_or(0.0).max(value + border_bands);
                physical_minima[g] = Some(physical);
            }
            g = g.saturating_add(span);
        }
    }
    let fallback = defaults[1].max(0.0) + defaults[3].max(0.0) + 1.0;
    let minima: Vec<f32> = minima.into_iter().map(|m| m.unwrap_or(fallback)).collect();
    let physical_minima: Vec<(f32, f32)> = physical_minima
        .into_iter()
        .enumerate()
        .map(|(i, m)| {
            // A preferred nonfixed table must retain its measured word minima
            // after the gaps contract its preferred grid. Combine both floors
            // so making room for text cannot take a border's physical minimum.
            let text = text_minima.and_then(|columns| columns.get(i)).map_or(0.0, |column| column.0);
            let minimum = m.unwrap_or(fallback).max(text);
            (minimum, minimum)
        })
        .collect();
    let total: f32 = grid.iter().sum();
    let floor: f32 = minima.iter().sum();
    let elastic: f32 = grid.iter().zip(&minima).map(|(w, m)| (*w - *m).max(0.0)).sum();
    let room = (total - gap * (grid.len() + 1) as f32 - floor).max(0.0);
    let scale = if elastic > 0.0 { (room / elastic).min(1.0) } else { 0.0 };
    for (width, minimum) in grid.iter_mut().zip(minima) {
        *width = minimum + (*width - minimum).max(0.0) * scale;
    }
    fit_column_minimums(grid, &physical_minima);
}

/// Cell margins cannot put text inside a visible horizontal border's half-band. Word's
/// AutoFit grid therefore includes the larger of that half-band and the cell's padding.
fn horizontal_insets(t: &Table, style: Option<&TableStyleProps>, ri: usize, g: usize, cell: &wordcraft_doc::Cell, ncols: usize) -> (f32, f32) {
    let parts = style.map(|s| &s.parts);
    let mut table = parts.and_then(|p| p.borders).unwrap_or_default();
    if let Some(own) = t.props.borders {
        table.overlay(&own);
    }
    let (first, last) = (g == 0, g.saturating_add(cell.span()) >= ncols);
    let region = region_of(t, style, ri, g, cell.span(), ncols);
    let regional = parts
        .map(|p| region_borders(p, (ri, t.rows.len()), (first, last), [region.0, region.1, region.2, region.3, region.4, region.5]))
        .unwrap_or_default();
    let own = cell.props.borders.unwrap_or_default();
    let left = own.left.or(if first { regional.left } else { regional.inside_v }).or(if first { table.left } else { table.inside_v });
    let right = own.right.or(if last { regional.right } else { regional.inside_v }).or(if last { table.right } else { table.inside_v });
    let half = |border: Option<Border>| border.filter(Border::is_visible).map_or(0.0, |b| b.width.clamp(0.0, 12.0) / 2.0);
    let margins = cell.props.margins.unwrap_or_else(|| default_margins(t, style));
    if effective_spacing(t, style).is_some_and(|s| s > 0.0) {
        (margins[1].max(0.0) + half(left), margins[3].max(0.0) + half(right))
    } else {
        (margins[1].max(half(left)), margins[3].max(half(right)))
    }
}

/// The margins (top, left, bottom, right; points) inside cell `cell` of row `row`: its own, else
/// the table's defaults, as the layout uses them. The app's ruler measures a cell's indents from
/// them.
pub fn cell_margins(doc: &wordcraft_doc::Document, t: &Table, row: usize, cell: usize) -> [f32; 4] {
    let style = t.props.style.as_deref().and_then(|id| doc.styles.table_style(id));
    t.rows.get(row).and_then(|r| r.cells.get(cell)).and_then(|c| c.props.margins).unwrap_or_else(|| default_margins(t, style.as_ref()))
}

/// The most paragraphs [`measure_table_columns`] lays out: a huge table is measured by its first
/// rows only, so AutoFit stays quick.
const MEASURE_BUDGET: usize = 4000;
/// The line length paragraphs are measured at: wide enough that nothing real wraps.
const MEASURE_WIDE: f32 = 15_840.0;
/// What an empty cell's text needs (about a paragraph mark), points.
const EMPTY_TEXT: f32 = 12.0;

/// What one paragraph needs: the longest run of text without a break opportunity (`min`) and
/// its longest line when nothing wraps (`max`), indents included.
fn measure_para(p: &wordcraft_doc::Paragraph, env: &crate::para::ParaEnv) -> (f32, f32) {
    use crate::para::ClKind;
    let pl = crate::para::layout_para(p, env);
    let rp = &pl.rp;
    let indents = rp.indent_left.max(0.0) + rp.indent_right.max(0.0) + rp.indent_first.max(0.0);
    let (mut word, mut min) = (0.0f32, 0.0f32);
    for c in &pl.clusters {
        match c.kind {
            ClKind::Text | ClKind::Object(_) => word += c.adv.max(0.0),
            ClKind::Marker => {}
            _ => {
                min = min.max(word);
                word = 0.0;
            }
        }
        if c.break_after {
            min = min.max(word);
            word = 0.0;
        }
    }
    min = min.max(word);
    let mut max = 0.0f32;
    for l in &pl.lines {
        let (mut lo, mut hi) = (f32::MAX, f32::MIN);
        for k in l.c0..l.c1 {
            let (Some(c), Some(a), Some(b)) = (pl.clusters.get(k), l.cl_left(k), l.cl_right(k)) else { continue };
            lo = lo.min(a.min(b));
            if !matches!(c.kind, ClKind::Space | ClKind::Marker | ClKind::LineBreak | ClKind::PageBreak | ClKind::ColumnBreak) {
                hi = hi.max(a.max(b));
            }
        }
        if hi > lo {
            max = max.max(hi - lo + l.hyphen.map_or(0.0, |h| h.2));
        }
    }
    let min = if min.is_finite() { min } else { 0.0 };
    let max = if max.is_finite() { max.max(min) } else { min };
    (min + indents, max + indents)
}

/// What each grid column of `t` needs for AutoFit to Contents, as (narrowest, widest) in points
/// with cell margins included: the narrowest fits each cell's longest word (text that can't
/// wrap), the widest fits every paragraph on one line. A cell spanning several columns spreads
/// what its columns lack over them. Paragraphs are laid out with the table style's formatting;
/// after [`MEASURE_BUDGET`] paragraphs the remaining rows are skipped.
pub fn measure_table_columns(doc: &wordcraft_doc::Document, t: &Table) -> Vec<(f32, f32)> {
    use wordcraft_doc::Block;
    let style = t.props.style.as_deref().and_then(|id| doc.styles.table_style(id));
    let margins_def = default_margins(t, style.as_ref());
    let ncols = t.cols().clamp(1, wordcraft_doc::table::MAX_COLS);
    let fields = crate::FieldCtx::default();
    let mut cols = vec![(0.0f32, 0.0f32); ncols];
    // Cells spanning several columns: (first column, span, min, max), applied after the others.
    let mut spanned: Vec<(usize, usize, f32, f32)> = Vec::new();
    let mut budget = MEASURE_BUDGET;
    'rows: for (ri, row) in t.rows.iter().enumerate() {
        let mut g = 0usize;
        for cell in &row.cells {
            let span = cell.span();
            let (left, right) = horizontal_insets(t, style.as_ref(), ri, g, cell, ncols);
            let side = left.max(0.0) + right.max(0.0);
            let text = style.as_ref().map(|st| region_text(st, region_of(t, style.as_ref(), ri, g, span, ncols)));
            let env = crate::para::ParaEnv {
                doc,
                width: MEASURE_WIDE,
                label: None,
                fields: &fields,
                show_hidden: false,
                field_codes: false,
                hide_deleted: false,
                table: text.as_ref(),
                proofing: None,
                // Measurement uses a fixed environment (like show_hidden/proofing above).
                numeral: crate::NumeralMode::default(),
                exclusions: &[],
                eq_number: 0,
                read_line_spacing: None,
                grid: crate::para::Grid::default(),
                fit_objects: false,
            };
            let (mut min, mut max) = (EMPTY_TEXT, EMPTY_TEXT);
            if cell.props.vmerge != VMerge::Continue && !cell.props.text_direction.is_turned() {
                for b in &cell.blocks {
                    match &**b {
                        Block::Para(p) => {
                            if budget == 0 {
                                break 'rows;
                            }
                            budget -= 1;
                            let (a, b) = measure_para(p, &env);
                            min = min.max(a);
                            max = max.max(b);
                        }
                        // A nested table keeps its own column widths.
                        Block::Table(inner) => {
                            let w: f32 = inner.grid.iter().filter(|w| w.is_finite()).map(|w| w.clamp(0.0, 1584.0)).sum();
                            min = min.max(w);
                            max = max.max(w);
                        }
                    }
                }
            }
            let (min, max) = ((min + side).min(1584.0), (max + side).min(1584.0));
            if span == 1 {
                if let Some(c) = cols.get_mut(g) {
                    c.0 = c.0.max(min);
                    c.1 = c.1.max(max);
                }
            } else {
                spanned.push((g, span, min, max));
            }
            g += span;
        }
    }
    for (g, span, min, max) in spanned {
        let Some(cs) = cols.get_mut(g..(g + span).min(ncols)) else { continue };
        if cs.is_empty() {
            continue;
        }
        let n = cs.len() as f32;
        let (have_min, have_max) = cs.iter().fold((0.0, 0.0), |(a, b), c| (a + c.0, b + c.1));
        for c in cs.iter_mut() {
            c.0 += ((min - have_min) / n).max(0.0);
            c.1 += ((max - have_max) / n).max(0.0);
            c.1 = c.1.max(c.0);
        }
    }
    // A column no cell starts in still needs room for its margins.
    let floor = margins_def[1].max(0.0) + margins_def[3].max(0.0) + EMPTY_TEXT;
    cols.iter_mut().for_each(|c| {
        c.0 = c.0.max(floor);
        c.1 = c.1.max(c.0);
    });
    if let Some(spacing) = effective_spacing(t, style.as_ref()) {
        let gap = 2.0 * spacing;
        for (i, column) in cols.iter_mut().enumerate() {
            let reserve = gap + if i == 0 { gap / 2.0 } else { 0.0 } + if i + 1 == ncols { gap / 2.0 } else { 0.0 };
            column.0 += reserve;
            column.1 += reserve;
        }
    }
    cols
}

/// Column widths for AutoFit to Contents, the way Word shares the room: every column gets its
/// widest content when that all fits in `avail`; otherwise each gets its narrowest plus a share
/// of the remaining room in proportion to how much wider its content would like to be. When
/// even the narrowest widths don't fit, the columns keep them (the table runs past the margin,
/// as in Word).
pub fn autofit_widths(cols: &[(f32, f32)], avail: f32) -> Vec<f32> {
    let clean = |v: f32| if v.is_finite() { v.clamp(0.0, 1584.0) } else { 0.0 };
    let cols: Vec<(f32, f32)> = cols.iter().map(|&(a, b)| (clean(a), clean(b).max(clean(a)))).collect();
    let avail = clean(avail);
    let (sum_min, sum_max) = cols.iter().fold((0.0f32, 0.0f32), |(a, b), c| (a + c.0, b + c.1));
    if sum_max <= avail {
        return cols.iter().map(|c| c.1).collect();
    }
    if sum_min >= avail || sum_max <= sum_min {
        return cols.iter().map(|c| c.0).collect();
    }
    let k = (avail - sum_min) / (sum_max - sum_min);
    cols.iter().map(|c| c.0 + (c.1 - c.0) * k).collect()
}

/// The first cell's left margin: how far its text sits inside the table's edge.
pub(crate) fn first_cell_left_margin(ctx: &Ctx, t: &Table) -> f32 {
    first_cell_left_margin_in(t, default_margins(t, table_style(ctx, t).as_ref()))
}

fn first_cell_left_margin_in(t: &Table, def: [f32; 4]) -> f32 {
    t.rows.first().and_then(|r| r.cells.first()).and_then(|c| c.props.margins).unwrap_or(def)[1]
}

/// A cell's borders from the table style's conditional regions it is in (`regions`: header row,
/// total row, first column, last column, row band, column band), later regions winning. Row
/// regions give their left/right edges at the table's sides and `inside_v` between cells; column
/// regions their top/bottom edges at the table's top and bottom and `between` between rows.
fn region_borders(p: &TableStyleParts, (ri, nrows): (usize, usize), (first_g, last_g): (bool, bool), regions: [bool; 6]) -> Borders {
    let [header, total, first_col, last_col, band, col_band] = regions;
    let row = |b: Option<Borders>| {
        b.map(|b| Borders {
            top: b.top,
            bottom: b.bottom,
            left: if first_g { b.left } else { b.inside_v },
            right: if last_g { b.right } else { b.inside_v },
            between: None,
            inside_v: None,
        })
    };
    let col = |b: Option<Borders>| {
        b.map(|b| Borders {
            // Conditional column insideH nil/none leaves the global grid in
            // Word; explicit outer edges and direct cell borders still cancel.
            top: if ri == 0 { b.top } else { b.between.filter(|b| b.style != BorderStyle::None) },
            bottom: if ri + 1 >= nrows { b.bottom } else { b.between.filter(|b| b.style != BorderStyle::None) },
            left: b.left,
            right: b.right,
            between: None,
            inside_v: None,
        })
    };
    let mut out = Borders::default();
    for (on, b) in [
        (col_band, col(p.col_band_borders)),
        (band, row(p.band_borders)),
        (first_col, col(p.first_col_borders)),
        (last_col, col(p.last_col_borders)),
        (header, row(p.header_borders)),
        (total, row(p.total_borders)),
    ] {
        if on && let Some(b) = b {
            out.overlay(&b);
        }
    }
    out
}

pub fn layout_table(ctx: &mut Ctx, story: StoryRef, t: &Table, path: &[u32], avail: f32, depth: usize) -> TableLayout {
    let style = table_style(ctx, t);
    let parts = style.as_ref().map(|s| &s.parts);
    let margins_def = default_margins(t, style.as_ref());
    let spacing = effective_spacing(t, style.as_ref());
    let separated = spacing.is_some();
    let gap = spacing.unwrap_or(0.0) * 2.0;
    // Cell text formatting per region, built once per table.
    let mut cell_text: Vec<(Region, CellText)> = Vec::new();
    let ncols = t.cols().max(1);
    let gap_budget = gap * (ncols + 1) as f32;
    let automatic = is_automatic(t);
    // Column widths.
    let mut grid: Vec<f32> = if t.grid.len() == ncols {
        t.grid.iter().map(|w| if w.is_finite() { w.max(4.0) } else { 72.0 }).collect()
    } else {
        vec![avail / ncols as f32; ncols]
    };
    let mut total: f32 = grid.iter().sum();
    // Before Word 2013 the text margins bound the edge cells' text, so a percentage
    // table's containing width also includes their effective start/end padding.
    let percentage_avail = if ctx.doc.settings.compat_mode < 15 {
        let left = first_cell_left_margin_in(t, margins_def).max(0.0);
        let right = t.rows.first().and_then(|r| r.cells.last()).and_then(|c| c.props.margins).unwrap_or(margins_def)[3].max(0.0);
        avail + left + right
    } else {
        avail
    };
    let target = match (t.props.width_pct, t.props.width) {
        (Some(p), _) if p > 0.0 => Some(percentage_avail * (p / 100.0).min(1.0)),
        (_, Some(w)) if w > 0.0 => Some(w),
        _ => None,
    };
    if let Some(tw) = target {
        let k = tw / total.max(1.0);
        grid.iter_mut().for_each(|w| *w *= k);
        total = tw;
    }
    if total > avail * 1.5 && !t.props.fixed {
        let k = avail / total.max(1.0);
        grid.iter_mut().for_each(|w| *w *= k);
        total = avail;
    }
    let mut preferred_minima = None;
    if !t.props.fixed {
        // Measuring lays paragraphs out once more; it mustn't count their equations twice.
        let eq_count = ctx.eq_count;
        let content = column_content(ctx, t, style.as_ref(), depth);
        ctx.eq_count = eq_count;
        if let Some(content) = content {
            autofit(&mut grid, &content, t, if separated { (avail - gap_budget).max(4.0) } else { avail });
            // Never wider than the room it had, however long an unbreakable word.
            let fitted: f32 = grid.iter().sum();
            let cap = avail.max(total);
            if fitted > cap {
                grid.iter_mut().for_each(|w| *w *= cap / fitted);
            }
            total = grid.iter().sum();
            if gap > 0.0 && !automatic {
                // Reuse this measurement at the post-gap boundary rather than
                // shaping/counting the same paragraphs a second time.
                preferred_minima = Some(content);
            }
        }
    }
    if separated {
        if automatic && !t.props.fixed {
            // An automatic table fits its measured contents plus the gaps. Preferred
            // widths instead keep their outer width and reserve the gaps inside it.
            total += gap_budget;
        } else {
            separated_widths(t, style.as_ref(), &mut grid, gap, preferred_minima.as_deref());
            // A preferred width smaller than all gaps and physical cell minima
            // cannot contain the table. Expand its frame with the realized grid.
            total = total.max(grid.iter().sum::<f32>() + gap_budget);
        }
    }
    let mut colx = Vec::with_capacity(ncols + 1);
    let mut acc = if separated { gap } else { 0.0 };
    for w in &grid {
        colx.push(acc);
        acc += w + if separated { gap } else { 0.0 };
    }
    colx.push(acc);
    // Word 2013 and later (compatibility mode 15) put the table's border at the margin (plus its
    // indent); earlier modes line the first cell's text up with it instead.
    let indent = t.props.indent.unwrap_or(0.0);
    // The style's borders, overlaid by the table's own side by side.
    let mut tborders = parts.and_then(|p| p.borders).unwrap_or_default();
    if let Some(own) = t.props.borders {
        tborders.overlay(&own);
    }
    let first_cell = t.rows.first().and_then(|r| r.cells.first());
    // A right-to-left table (`w:bidiVisual`) mirrors: its first column stands at the right, its
    // start alignment and indent are measured from the right margin, and each cell's start
    // (left) margin and border are on its right.
    let rtl = t.props.bidi_visual;
    // How far in from the start margin the table's edge sits when it is start-aligned.
    let mut start_offset = if ctx.doc.settings.compat_mode >= 15 {
        // The border is centred on the edge, so half of it sits outside: Word moves the table in
        // by that half.
        let border = first_cell.and_then(|c| c.props.borders.and_then(|b| b.left)).or(tborders.left);
        indent + border.filter(Border::is_visible).map_or(0.0, |b| b.width.clamp(0.0, 12.0) / 2.0)
    } else {
        indent - first_cell_left_margin_in(t, margins_def)
    };
    // The additional separated-frame half-band is internal to the table. RTL
    // start alignment anchors with the original offset, so changing only its
    // border width does not move the physical text positions.
    let rtl_anchor_offset = start_offset;
    if separated && ctx.doc.settings.compat_mode >= 15 {
        let border = first_cell.and_then(|c| c.props.borders.and_then(|b| b.left)).or(tborders.left);
        start_offset += border.filter(Border::is_visible).map_or(0.0, |b| b.width.clamp(0.0, 12.0) / 2.0);
    }
    let x = match (t.props.align, rtl) {
        (Some(Align::Center), _) => (avail - total) / 2.0,
        // End alignment: the right margin, or the left one in a right-to-left table.
        (Some(Align::Right), false) => avail - total,
        (Some(Align::Right), true) => 0.0,
        (_, true) => avail - total - rtl_anchor_offset,
        (_, false) => start_offset,
    };
    let nrows = t.rows.len();
    let header_rows = t.props.look.header_row;
    // Grid columns holding a vertical-merge continuation, per row. A border
    // inside such a merge does not form a horizontal row boundary.
    let cont: Vec<Vec<usize>> = t
        .rows
        .iter()
        .map(|r| {
            let mut gx = 0;
            r.cells
                .iter()
                .filter_map(|x| {
                    let g = gx;
                    gx += x.span();
                    (x.props.vmerge == VMerge::Continue).then_some(g)
                })
                .collect()
        })
        .collect();
    // First pass: lay out every cell's content.
    struct CellBox {
        items: Vec<Placed>,
        paragraph_ends: HashMap<Path, f32>,
        h: f32,
        x: f32,
        w: f32,
        valign: VAlign,
        fill: Option<Rgb>,
        borders: Borders,
        /// Cell top paint may differ from the band reserved by the outer table.
        top_pen: Option<Border>,
        /// A cell/conditional nil top is distinct from a nil insideH fallback.
        nil_top_override: bool,
        /// Measured first-row insideH fallback under a nil outer table top.
        fallback_top_sides: bool,
        vmerge: VMerge,
        margins: [f32; 4],
        g: usize,
        span: usize,
        /// Turned text: `items` are still in the turned frame (placed in the second pass).
        turned: Option<Turned>,
    }
    let mut rows: Vec<Vec<CellBox>> = Vec::with_capacity(nrows);
    let mut heights: Vec<f32> = Vec::with_capacity(nrows);
    let mut row_tops = Vec::with_capacity(nrows);
    let mut row_spaces = Vec::with_capacity(nrows);
    for (ri, row) in t.rows.iter().enumerate() {
        let mut g = 0usize;
        let mut cells = Vec::with_capacity(row.cells.len());
        let mut rh = 0.0f32;
        // The tallest cell margins plus border bands in the row.
        let mut row_insets = 0.0f32;
        // Every cell an empty one with a hidden end-of-cell mark: the row is only its borders.
        let mut collapsed = true;
        let is_header = header_rows && ri == 0;
        let is_total = t.props.look.total_row && ri + 1 == nrows && nrows > 1;
        let band = banded(t, style.as_ref(), ri);
        for (ci, cell) in row.cells.iter().enumerate() {
            let span = cell.span();
            let x0 = colx.get(g).copied().unwrap_or(acc);
            let x1 = colx.get((g + span).min(ncols)).copied().unwrap_or(acc) - if separated { gap } else { 0.0 };
            // Right to left: grid columns run from the right edge. In separated
            // tables the explicit left/right padding remains physical, as in
            // Word's bidi-only controls; retain the collapsed-table behavior.
            let (x0, x1) = if rtl { ((acc - x1).max(0.0), (acc - x0).max(0.0)) } else { (x0, x1) };
            let margins = cell.props.margins.unwrap_or(margins_def);
            let mut margins = if rtl && !separated { [margins[0], margins[3], margins[2], margins[1]] } else { margins };
            let mut fill = cell.props.shading;
            let region = region_of(t, style.as_ref(), ri, g, span, ncols);
            let (_, _, first_col, last_col, _, col_band) = region;
            // Conditional formatting, later regions winning (ECMA-376 §17.7.6): whole table,
            // column bands, row bands, first/last column, header/total row.
            if let Some(p) = parts {
                let row_fill = if is_header {
                    p.header_fill
                } else if is_total {
                    p.total_fill
                } else {
                    None
                };
                let col_fill = if first_col {
                    p.first_col_fill
                } else if last_col {
                    p.last_col_fill
                } else {
                    None
                };
                fill = fill
                    .or(row_fill)
                    .or(col_fill)
                    .or(if band { p.band_fill } else { None })
                    .or(if col_band { p.col_band_fill } else { None })
                    .or(p.fill);
            }
            if let Some(st) = &style
                && !cell_text.iter().any(|(r, _)| *r == region)
            {
                cell_text.push((region, region_text(st, region)));
            }
            let text = cell_text.iter().find(|(r, _)| *r == region).map(|(_, c)| c);
            let mut cpath = path.to_vec();
            cpath.push(ri as u32);
            cpath.push(ci as u32);
            // Effective borders: cell > table (outer vs inside).
            let tb = tborders;
            let edge =
                |own: Option<Border>, outer: bool, outer_b: Option<Border>, inner_b: Option<Border>| own.or(if outer { outer_b } else { inner_b });
            let cb = cell.props.borders.unwrap_or_default();
            let (first_g, last_g) = (g == 0, g + span >= ncols);
            let rb = parts
                .map(|p| region_borders(p, (ri, nrows), (first_g, last_g), [is_header, is_total, first_col, last_col, band, col_band]))
                .unwrap_or_default();
            let mut borders = Borders {
                top: edge(cb.top.or(rb.top), ri == 0, tb.top, tb.between),
                bottom: edge(cb.bottom.or(rb.bottom), ri + 1 == nrows, tb.bottom, tb.between),
                left: edge(cb.left.or(if first_g { rb.left } else { rb.inside_v }), first_g, tb.left, tb.inside_v),
                right: edge(cb.right.or(if last_g { rb.right } else { rb.inside_v }), last_g, tb.right, tb.inside_v),
                between: None,
                inside_v: None,
            };
            // Retain the provenance of a cell or conditional top override;
            // Some(nil) is not the absence of a pen and must not fall back.
            let mut cell_top_override = cb.top.or(rb.top);
            if is_total
                && parts.is_none_or(|p| p.total_borders.is_none_or(|b| b.top.is_none()))
                && let Some(b) = parts.and_then(|p| p.total_border_top)
            {
                borders.top = Some(b);
                cell_top_override = Some(b);
            }
            // Keep borders unchanged for measurements. First-row cell paint
            // defaults to insideH, independently of the outer reserved band.
            let top_pen = if separated && ri == 0 { cell_top_override.or(tb.between) } else { borders.top };
            let nil_top_override = separated && ri == 0 && cell_top_override.is_some_and(|b| b.style == BorderStyle::None);
            let fallback_top_sides = separated
                && ri == 0
                && cell_top_override.is_none()
                && tb.top.is_some_and(|b| b.style == BorderStyle::None)
                && top_pen.is_some_and(|b| b.style == BorderStyle::None || b.is_visible());
            // The logical start (left) and end (right) borders, drawn mirrored in an RTL table.
            if rtl {
                std::mem::swap(&mut borders.left, &mut borders.right);
            }
            // Word keeps the cell's text clear of its top border (and the last row's of its bottom
            // border): the border's width sits on top of the cell margin.
            let (band_t, band_b) = (border_band(borders.top), if separated || ri + 1 == nrows { border_band(borders.bottom) } else { 0.0 });
            if spacing.is_some_and(|s| s > 0.0) {
                margins[1] = margins[1].max(0.0) + border_band(borders.left) / 2.0;
                margins[3] = margins[3].max(0.0) + border_band(borders.right) / 2.0;
            } else {
                margins[1] = margins[1].max(border_band(borders.left) / 2.0);
                margins[3] = margins[3].max(border_band(borders.right) / 2.0);
            }
            let cw = (x1 - x0 - margins[1] - margins[3]).max(4.0);
            let insets = margins[0] + margins[2] + band_t + band_b;
            let turn = cell.props.text_direction;
            let mut turned = None;
            let (items, paragraph_ends, h) = if cell.props.vmerge == VMerge::Continue {
                (Vec::new(), HashMap::new(), 0.0)
            } else if turn.is_turned() {
                collapsed = false;
                // Turned text: its lines run along the cell's height. An exact row height sets
                // their length; otherwise they get the length the text needs unwrapped and the row
                // grows to fit (the second pass lays them out again if the row ends up taller).
                let snap = (ctx.counters.clone(), ctx.eq_count);
                let len = match (row.props.height, row.props.height_rule) {
                    (Some(h), HeightRule::Exact) if h > 0.0 => (h - insets).clamp(4.0, TURNED_LIMIT),
                    _ => {
                        let probe = layout_box(ctx, story, &cell.blocks, &cpath, TURNED_MAX, text, depth, None).items;
                        (ctx.counters, ctx.eq_count) = (snap.0.clone(), snap.1);
                        natural_length(&probe).clamp(4.0, TURNED_MAX)
                    }
                };
                let BoxLayout { items, height, floats_bottom, .. } = layout_box(ctx, story, &cell.blocks, &cpath, len, text, depth, None);
                let across = height.max(floats_bottom);
                turned = Some(Turned { turn, len, across, snap, region, ci, top: margins[0] + band_t });
                (items, HashMap::new(), len)
            } else {
                let BoxLayout { mut items, mut height, floats_bottom, last_top, mut paragraph_ends, .. } =
                    layout_box(ctx, story, &cell.blocks, &cpath, cw, text, depth, None);
                // A hidden end-of-cell mark: an empty last paragraph takes no room (Word for
                // Windows: a row of empty such cells is only its borders).
                if cell.props.hide_mark && cell.blocks.last().and_then(|b| b.as_para()).is_some_and(|p| p.text.is_empty()) {
                    height = last_top;
                } else {
                    collapsed = false;
                }
                // Word's required empty paragraph directly after a nested in-flow table
                // carries the cell end without adding a line or before/after spacing.
                // Keep its laid-out items and paragraph-end metadata for editing; only
                // its height contribution is removed. Two trailing paragraphs do not
                // qualify, nor paragraphs with content or independently drawn features.
                let table_tail = cell.blocks.split_last().is_some_and(|(last, previous)| {
                    let Some(p) = last.as_para() else { return false };
                    p.text.is_empty()
                        && p.runs.is_empty()
                        && p.objects.is_empty()
                        && p.mark.is_empty()
                        && p.section.is_none()
                        && matches!(previous.last().map(|b| &**b), Some(Block::Table(t)) if t.props.float.is_none() && !t.rows.is_empty())
                        && items.iter().any(|it| {
                            matches!(it,
                                Placed::Lines { path: pp, para, .. }
                                    if pp.0.len() == cpath.len() + 1
                                        && pp.0.starts_with(&cpath)
                                        && pp.0.last().copied() == Some((cell.blocks.len() - 1) as u32)
                                        && para.label.is_none()
                                        && para.glyphs.is_empty()
                                        && !para.has_page_fields
                                        && para.notes.is_empty()
                                        && para.rp.numbering.is_none()
                                        && para.rp.frame.is_none()
                                        && para.rp.borders.is_none()
                                        && para.rp.shading.is_none()
                                        && !para.rp.page_break_before
                            )
                        })
                });
                if table_tail {
                    height = last_top;
                }
                for it in &mut items {
                    it.translate(x0 + margins[1], margins[0] + band_t);
                }
                for end in paragraph_ends.values_mut() {
                    *end += margins[0] + band_t;
                }
                // The cell grows to hold its floating tables.
                (items, paragraph_ends, height.max(floats_bottom))
            };
            let h = h + insets;
            cells.push(CellBox {
                items,
                paragraph_ends,
                h,
                x: x0,
                w: x1 - x0,
                valign: cell.props.valign,
                fill,
                borders,
                top_pen,
                nil_top_override,
                fallback_top_sides,
                vmerge: cell.props.vmerge,
                margins,
                g,
                span,
                turned,
            });
            g += span;
        }
        // Horizontal borders reserve one band across the row, even when only a
        // smaller-font cell owns the border. Normalize before taking the tallest
        // cell, and move every baseline by the same top band.
        let previous_bottom = if separated {
            0.0
        } else {
            rows.last().map_or(0.0, |previous| {
                previous
                    .iter()
                    .filter(|c| c.vmerge == VMerge::None || !cont.get(ri).is_some_and(|v| v.contains(&c.g)))
                    .map(|c| border_band(c.borders.bottom))
                    .fold(0.0f32, f32::max)
            })
        };
        let own_top = cells.iter().filter(|c| c.vmerge != VMerge::Continue).map(|c| border_band(c.borders.top)).fold(0.0f32, f32::max);
        let outer_top = if separated && ri == 0 { border_band(tborders.top) } else { 0.0 };
        let outer_bottom = if separated && ri + 1 == nrows { border_band(tborders.bottom) } else { 0.0 };
        let leading = if separated { gap + outer_top } else { 0.0 };
        let trailing = if separated && ri + 1 == nrows { gap + outer_bottom } else { 0.0 };
        let top_band = own_top.max(previous_bottom) + leading;
        let bottom_band = if separated || ri + 1 == nrows { cells.iter().map(|c| border_band(c.borders.bottom)).fold(0.0f32, f32::max) } else { 0.0 };
        // Word shares the largest vertical cell margins across the row, even
        // when the cell supplying the bottom margin contains only short text.
        let margin_top = cells.iter().map(|c| c.margins[0]).fold(0.0f32, f32::max);
        let margin_bottom = cells.iter().map(|c| c.margins[2]).fold(0.0f32, f32::max);
        row_tops.push((own_top + leading, top_band, bottom_band, margin_top, margin_bottom));
        row_spaces.push((leading, trailing));
        for c in &mut cells {
            let extra_top = top_band - border_band(c.borders.top) + margin_top - c.margins[0];
            let own_bottom = if separated || ri + 1 == nrows { border_band(c.borders.bottom) } else { 0.0 };
            c.h += extra_top + bottom_band - own_bottom + margin_bottom - c.margins[2];
            c.margins[0] = margin_top;
            c.margins[2] = margin_bottom;
            if let Some(turned) = &mut c.turned {
                turned.top += extra_top;
            } else {
                for item in &mut c.items {
                    item.translate(0.0, extra_top);
                }
                for end in c.paragraph_ends.values_mut() {
                    *end += extra_top;
                }
            }
            if c.vmerge != VMerge::Restart {
                rh = rh.max(c.h);
            }
            row_insets = row_insets.max(c.margins[0] + c.margins[2] + top_band + bottom_band);
        }
        // Word adds the cells' top and bottom margins and border bands to an at-least row height:
        // a 30pt row with 5pt margins and 0.5pt borders is 40.5pt tall even when its text needs less.
        let rh = match (row.props.height, row.props.height_rule) {
            // Word's present-spacing Exact controls absorb the inter-row gap in
            // the declared row advance. The final half-gap is outside the rows.
            (Some(h), HeightRule::Exact) if separated && h > 0.0 => {
                h + margin_top + bottom_band + outer_top + outer_bottom + if ri + 1 == nrows { 1.5 * gap } else { 0.0 }
            }
            // The final bottom band follows the declared exact height; the top
            // band remains inside it (Word 20pt exact-row controls).
            (Some(h), HeightRule::Exact) if h > 0.0 => h + bottom_band,
            (Some(h), _) if h > 0.0 => rh.max(h + row_insets) + trailing,
            _ => rh + trailing,
        };
        heights.push(if collapsed && !cells.is_empty() { rh.max(0.0) } else { rh.max(4.0) });
        rows.push(cells);
    }
    // Vertically merged cells: grow the last row of the merge if the content is taller.
    for ri in 0..rows.len() {
        let restarts: Vec<(usize, f32, usize)> =
            rows.get(ri).map(|r| r.iter().filter(|c| c.vmerge == VMerge::Restart).map(|c| (c.g, c.h, c.span)).collect()).unwrap_or_default();
        for (g, h, _) in restarts {
            let mut end = ri;
            while rows.get(end + 1).is_some_and(|r| r.iter().any(|c| c.g == g && c.vmerge == VMerge::Continue)) {
                end += 1;
            }
            let have: f32 = heights.get(ri..=end).map(|s| s.iter().sum()).unwrap_or(0.0);
            if h > have
                && let Some(last) = heights.get_mut(end)
            {
                *last += h - have;
            }
        }
    }
    // Second pass: place cells with their row height, fills and borders.
    let mut out = Vec::with_capacity(rows.len());
    for (ri, cells) in rows.into_iter().enumerate() {
        let rh = heights.get(ri).copied().unwrap_or(0.0);
        let mut items = Vec::new();
        let (own_top, top, _, margin_top, margin_bottom) = row_tops[ri];
        let (leading, trailing) = row_spaces[ri];
        let mut bands = RowBands {
            top,
            own_top,
            margin_top,
            margin_bottom,
            bottom_in_advance: separated || ri + 1 == nrows,
            separated,
            leading,
            trailing,
            ..Default::default()
        };
        for (ci, c) in cells.into_iter().enumerate() {
            // A restart cell spans several rows: its height is their sum.
            let mut span_h = rh;
            let mut end_row = ri;
            if c.vmerge == VMerge::Restart {
                let mut k = ri + 1;
                while cont.get(k).is_some_and(|v| v.contains(&c.g)) {
                    span_h += heights.get(k).copied().unwrap_or(0.0);
                    end_row = k;
                    k += 1;
                }
            }
            let final_band = if separated || end_row + 1 == nrows { row_tops[end_row].2 } else { 0.0 };
            let end_trailing = row_spaces[end_row].1;
            let (left_band, right_band) = if separated { (border_band(c.borders.left), border_band(c.borders.right)) } else { (0.0, 0.0) };
            // The grid advance includes both border bands. Shading paints only
            // the interior, while Cell retains its nominal advance for structure.
            let rect =
                Rect::new(c.x + left_band / 2.0, top, (c.w - left_band - right_band).max(0.0), (span_h - top - final_band - end_trailing).max(0.0));
            if c.vmerge != VMerge::Continue
                && let Some(f) = c.fill
            {
                bands.fills.push(items.len());
                items.push(Placed::Fill { rect, color: f });
            }
            let mut dy = match c.valign {
                VAlign::Top => 0.0,
                VAlign::Center => ((span_h - end_trailing - c.h) / 2.0).max(0.0),
                VAlign::Bottom => (span_h - end_trailing - c.h).max(0.0),
            };
            let mut cell_items = c.items;
            if let Some(tn) = c.turned {
                dy = 0.0;
                let m = c.margins;
                // The lines run the cell's whole height (laid out again if the row grew).
                let len = (span_h - (c.h - tn.len)).clamp(4.0, TURNED_LIMIT);
                let mut across = tn.across;
                if (len - tn.len).abs() > 0.01
                    && let Some(cell) = t.rows.get(ri).and_then(|r| r.cells.get(tn.ci))
                {
                    let mut cpath = path.to_vec();
                    cpath.extend([ri as u32, tn.ci as u32]);
                    let text = cell_text.iter().find(|(r, _)| *r == tn.region).map(|(_, c)| c);
                    // Same list numbers as the first layout: lay it out from the same counters.
                    let now = (std::mem::replace(&mut ctx.counters, tn.snap.0), std::mem::replace(&mut ctx.eq_count, tn.snap.1));
                    let b = layout_box(ctx, story, &cell.blocks, &cpath, len, text, depth, None);
                    (cell_items, across) = (b.items, b.height.max(b.floats_bottom));
                    (ctx.counters, ctx.eq_count) = now;
                }
                // Lines stack across the cell from its start edge (right for top-to-bottom text,
                // left for bottom-to-top); the vertical alignment moves them across it.
                let cw = (c.w - m[1] - m[3]).max(0.0);
                let off = match c.valign {
                    VAlign::Top => 0.0,
                    VAlign::Center => ((cw - across) / 2.0).max(0.0),
                    VAlign::Bottom => (cw - across).max(0.0),
                };
                let (ox, oy) = match tn.turn {
                    TextDirection::Up => (c.x + m[1] + off, tn.top + len),
                    _ => (c.x + m[1] + cw - off, tn.top),
                };
                for it in &mut cell_items {
                    it.turn(tn.turn, ox, oy);
                }
            }
            for mut it in cell_items {
                it.translate(0.0, dy);
                items.push(it);
            }
            bands.paragraph_ends.extend(c.paragraph_ends.into_iter().map(|(path, end)| (path, end + dy)));
            let b = c.borders;
            // c.x is the left stroke center; the closing center excludes the
            // two half-bands already present in a separated cell's advance.
            let (x0, x1, y0, y1) = (c.x, c.x + c.w - (left_band + right_band) / 2.0, leading, (rh - trailing).max(leading));
            let top_vis = c.vmerge != VMerge::Continue;
            if top_vis && let Some(e) = c.top_pen.filter(Border::is_visible) {
                // Align the paint to the end of the already reserved top band.
                // Explicit cell/conditional pens reproduce the original center;
                // fallback insideH can have a different width without moving text.
                let half = border_band(Some(e)) / 2.0;
                let y = if separated && ri == 0 { y0 + border_band(b.top) - half } else { y0 + half };
                bands.top_rules.push(items.len());
                items.push(Placed::Rule { x0, y0: y, x1, y1: y, border: e });
            }
            let last_of_merge = c.vmerge == VMerge::None || !cont.get(ri + 1).is_some_and(|v| v.contains(&c.g));
            if last_of_merge && let Some(e) = b.bottom.filter(Border::is_visible) {
                let width = border_band(Some(e));
                bands.bottom = bands.bottom.max(width);
                let y = y1 + if separated || ri + 1 == nrows { -width / 2.0 } else { width / 2.0 };
                bands.bottom_rules.push(items.len());
                items.push(Placed::Rule { x0, y0: y, x1, y1: y, border: e });
            }
            // In this measured first-row family, side ink excludes the reserved
            // bottom band. A nil top also joins the outer start; a visible
            // insideH fallback retains its original start. A nil bottom keeps
            // the measured half-gap extension. Other origins and merged cells
            // retain their old path, as do absent/zero-width bottom pens.
            let measured_sides = (c.nil_top_override || c.fallback_top_sides) && c.vmerge == VMerge::None;
            let nil_top_sides = measured_sides && c.top_pen.is_some_and(|b| b.style == BorderStyle::None);
            let side_top = if nil_top_sides { 0.0 } else { y0 };
            let side_bottom = if measured_sides {
                match b.bottom {
                    Some(e) if e.style == BorderStyle::None => rect.bottom() + gap / 2.0,
                    Some(e) if e.is_visible() => rect.bottom(),
                    _ => y1,
                }
            } else {
                y1
            };
            if let Some(e) = b.left.filter(Border::is_visible) {
                bands.frames.push(items.len());
                items.push(Placed::Rule { x0, y0: side_top, x1: x0, y1: side_bottom, border: e });
            }
            if let Some(e) = b.right.filter(Border::is_visible) {
                bands.frames.push(items.len());
                items.push(Placed::Rule { x0: x1, y0: side_top, x1, y1: side_bottom, border: e });
            }
            bands.frames.push(items.len());
            items.push(Placed::Cell {
                rect: Rect::new(c.x, leading, c.w, ((if c.vmerge == VMerge::Continue { rh } else { span_h }) - leading - end_trailing).max(0.0)),
                covered: c.vmerge == VMerge::Continue,
                table: wordcraft_doc::Path(path.to_vec()),
                row: ri,
                cell: ci,
                story,
            });
        }
        if separated {
            // Cell borders no longer replace the table frame. Keep this frame at
            // the preferred outer width while the individual cells reserve gaps.
            let left_border = if rtl { tborders.right } else { tborders.left };
            let right_border = if rtl { tborders.left } else { tborders.right };
            let frame_left = if ctx.doc.settings.compat_mode >= 15 { border_band(left_border) / 2.0 - start_offset + indent } else { 0.0 };
            let frame_right = frame_left + total;
            bands.table_frame = Some(TableFrame {
                left: frame_left,
                right: frame_right,
                borders: Borders { left: left_border, right: right_border, ..tborders },
                gap,
                first: ri == 0,
                last: ri + 1 == nrows,
                exact: t.rows[ri].props.height_rule == HeightRule::Exact && t.rows[ri].props.height.is_some_and(|h| h > 0.0),
            });
            if ri == 0
                && let Some(border) = tborders.top.filter(Border::is_visible)
            {
                let y = border_band(Some(border)) / 2.0;
                bands.top_rules.push(items.len());
                bands.outer_rules.push(items.len());
                items.push(Placed::Rule { x0: frame_left, y0: y, x1: frame_right, y1: y, border });
            }
            if ri + 1 == nrows
                && let Some(border) = tborders.bottom.filter(Border::is_visible)
            {
                let y = rh - border_band(Some(border)) / 2.0;
                bands.bottom_rules.push(items.len());
                bands.outer_rules.push(items.len());
                items.push(Placed::Rule { x0: frame_left, y0: y, x1: frame_right, y1: y, border });
            }
            for (xx, border) in [(frame_left, left_border), (frame_right, right_border)] {
                if let Some(border) = border.filter(Border::is_visible) {
                    bands.frames.push(items.len());
                    bands.outer_rules.push(items.len());
                    items.push(Placed::Rule { x0: xx, y0: 0.0, x1: xx, y1: rh, border });
                }
            }
        }
        out.push(RowLayout { height: rh, items, bands });
    }
    TableLayout { x, width: total, rows: out }
}

/// Split a row at `cut` (points below the row's top) so the part above fits on this page.
/// Paragraph lines break between lines; fills, vertical rules and cell areas are cut; the
/// rest moves up to the top of the continuation. `None` when no line fits above the cut.
pub fn split_row(row: &RowLayout, cut: f32) -> Option<(RowLayout, RowLayout)> {
    if cut <= 0.0 || cut >= row.height {
        return None;
    }
    // The caller already reserves closing_band() when it lies outside the
    // advance. Reserve only the cell padding and an inside closing band here.
    let inside_bottom = if row.bands.bottom_in_advance { row.bands.bottom } else { 0.0 };
    let reserve = row.bands.margin_bottom + inside_bottom + row.bands.trailing;
    let cut = cut - reserve;
    if cut <= 0.0 {
        return None;
    }
    // Where each paragraph splits, and how far up the continuation moves.
    let mut first_moved = f32::MAX;
    let mut kept_any = false;
    let mut kept_end = 0.0f32;
    for (source, it) in row.items.iter().enumerate() {
        // Turned text moves as a whole, like a picture.
        if let Some(r) = it.turned_bounds() {
            if r.bottom() <= cut + 0.01 {
                kept_any = true;
                kept_end = kept_end.max(r.bottom());
            } else {
                first_moved = first_moved.min(r.y);
            }
            continue;
        }
        match it {
            Placed::Lines { path, para, l0, l1, y, .. } => {
                let base = para.lines.get(*l0).map(|l| l.top).unwrap_or(0.0);
                for k in *l0..*l1 {
                    let Some(l) = para.lines.get(k) else { continue };
                    let (top, bottom) = (y + l.top - base, y + l.top - base + l.height);
                    let bottom = if *l1 == para.lines.len() && k + 1 == *l1 {
                        bottom.max(row.bands.paragraph_ends.get(path).copied().unwrap_or(bottom))
                    } else {
                        bottom
                    };
                    if bottom <= cut + 0.01 {
                        kept_any = true;
                        kept_end = kept_end.max(bottom);
                    } else {
                        first_moved = first_moved.min(top);
                        break;
                    }
                }
            }
            Placed::Image { rect, .. } | Placed::Shape { rect, .. } | Placed::Graphic { rect, .. } | Placed::Clip { rect, .. } => {
                if rect.bottom() <= cut + 0.01 {
                    kept_any = true;
                    kept_end = kept_end.max(rect.bottom());
                } else {
                    first_moved = first_moved.min(rect.y);
                }
            }
            // Nested cells have real structure beyond their last text line.
            // The row's own frame follows the fragment and is not content.
            Placed::Cell { rect, .. } if row.bands.frames.binary_search(&source).is_err() && rect.y < cut => {
                kept_end = kept_end.max(rect.bottom().min(cut));
            }
            _ => {}
        }
    }
    if !kept_any {
        return None;
    }
    // Every line fits and only a nested table's cell runs past the cut (a floating table's
    // fixed-height row, say): split there rather than move the whole row on. A row that is just
    // taller than its text moves on whole, as in Word.
    let own_depth = row.items.iter().filter_map(|it| if let Placed::Cell { table, .. } = it { Some(table.0.len()) } else { None }).min();
    let nested_past_cut =
        row.items.iter().any(|it| matches!(it, Placed::Cell { rect, table, .. } if Some(table.0.len()) > own_depth && rect.bottom() > cut + 0.01));
    if first_moved == f32::MAX && nested_past_cut {
        first_moved = cut;
        kept_end = kept_end.max(cut);
    }
    if first_moved == f32::MAX {
        return None;
    }
    // Close at retained content, not at the page cap. A raw continuation starts
    // at its own top band; place_table supplies the actual physical predecessor.
    let head_height = kept_end + reserve;
    let shift = (first_moved - row.bands.margin_top - row.bands.own_top).max(0.0);
    let tail_height = (row.height - shift).max(4.0);
    let (mut a, mut b) = (Vec::new(), Vec::new());
    let fragment_bands = || RowBands {
        top: row.bands.own_top,
        own_top: row.bands.own_top,
        margin_top: row.bands.margin_top,
        margin_bottom: row.bands.margin_bottom,
        bottom: row.bands.bottom,
        bottom_in_advance: row.bands.bottom_in_advance,
        separated: row.bands.separated,
        leading: row.bands.leading,
        trailing: row.bands.trailing,
        table_frame: row.bands.table_frame.clone(),
        ..Default::default()
    };
    let (mut a_bands, mut b_bands) = (fragment_bands(), fragment_bands());
    a_bands.top = row.bands.top;
    let push = |items: &mut Vec<Placed>, bands: &mut RowBands, source: usize, item: Placed| {
        if row.bands.frames.binary_search(&source).is_ok() {
            bands.frames.push(items.len());
        }
        if row.bands.top_rules.binary_search(&source).is_ok() {
            bands.top_rules.push(items.len());
        }
        if row.bands.bottom_rules.binary_search(&source).is_ok() {
            bands.bottom_rules.push(items.len());
        }
        if row.bands.outer_rules.contains(&source) {
            bands.outer_rules.push(items.len());
        }
        if row.bands.fills.binary_search(&source).is_ok() {
            bands.fills.push(items.len());
        }
        items.push(item);
    };
    for (source, it) in row.items.iter().enumerate() {
        if let Some(r) = it.turned_bounds() {
            let mut it = it.clone();
            if r.bottom() <= cut + 0.01 {
                push(&mut a, &mut a_bands, source, it);
            } else {
                it.translate(0.0, -shift);
                push(&mut b, &mut b_bands, source, it);
            }
            continue;
        }
        match it {
            Placed::Lines { story, path, para, l0, l1, x, y, turn } => {
                let base = para.lines.get(*l0).map(|l| l.top).unwrap_or(0.0);
                let mut split = *l1;
                for k in *l0..*l1 {
                    let Some(l) = para.lines.get(k) else { continue };
                    let bottom = y + l.top - base + l.height;
                    let bottom = if *l1 == para.lines.len() && k + 1 == *l1 {
                        bottom.max(row.bands.paragraph_ends.get(path).copied().unwrap_or(bottom))
                    } else {
                        bottom
                    };
                    if bottom > cut + 0.01 {
                        split = k;
                        break;
                    }
                }
                if split > *l0 {
                    push(
                        &mut a,
                        &mut a_bands,
                        source,
                        Placed::Lines { story: *story, path: path.clone(), para: para.clone(), l0: *l0, l1: split, x: *x, y: *y, turn: *turn },
                    );
                }
                if split < *l1 {
                    let top = para.lines.get(split).map(|l| y + l.top - base).unwrap_or(*y);
                    push(
                        &mut b,
                        &mut b_bands,
                        source,
                        Placed::Lines {
                            story: *story,
                            path: path.clone(),
                            para: para.clone(),
                            l0: split,
                            l1: *l1,
                            x: *x,
                            y: top - shift,
                            turn: *turn,
                        },
                    );
                }
            }
            Placed::Image { rect, .. }
            | Placed::Shape { rect, .. }
            | Placed::Graphic { rect, .. }
            | Placed::Clip { rect, .. }
            | Placed::Object { rect, .. } => {
                if rect.bottom() <= cut + 0.01 {
                    push(&mut a, &mut a_bands, source, it.clone());
                } else {
                    let mut it = it.clone();
                    it.translate(0.0, -shift);
                    push(&mut b, &mut b_bands, source, it);
                }
            }
            // Shading and cell areas: the part above the cut stays, the rest moves up with the
            // continuation (a nested table's cell below the cut moves whole).
            Placed::Fill { rect, color } => {
                let (above, below) = if row.bands.fills.binary_search(&source).is_ok() {
                    (
                        Some(Rect::new(rect.x, rect.y, rect.w, (head_height - inside_bottom - row.bands.trailing - rect.y).max(0.0))),
                        Some(Rect::new(
                            rect.x,
                            row.bands.own_top,
                            rect.w,
                            (tail_height - inside_bottom - row.bands.trailing - row.bands.own_top).max(0.0),
                        )),
                    )
                } else {
                    split_rect(*rect, cut, shift)
                };
                if let Some(rect) = above {
                    push(&mut a, &mut a_bands, source, Placed::Fill { rect, color: *color });
                }
                if let Some(rect) = below {
                    push(&mut b, &mut b_bands, source, Placed::Fill { rect, color: *color });
                }
            }
            Placed::Cell { rect, covered, table, row: r, cell, story } => {
                let (above, below) = if row.bands.frames.binary_search(&source).is_ok() {
                    (
                        Some(Rect::new(rect.x, rect.y, rect.w, (head_height - row.bands.trailing - rect.y).max(0.0))),
                        Some(Rect::new(rect.x, rect.y, rect.w, (tail_height - row.bands.trailing - rect.y).max(0.0))),
                    )
                } else {
                    split_rect(*rect, cut, shift)
                };
                let cell_at = |rect| Placed::Cell { rect, covered: *covered, table: table.clone(), row: *r, cell: *cell, story: *story };
                if let Some(rect) = above {
                    push(&mut a, &mut a_bands, source, cell_at(rect));
                }
                if let Some(rect) = below {
                    push(&mut b, &mut b_bands, source, cell_at(rect));
                }
            }
            Placed::Rule { x0, y0, x1, y1, border } => {
                if (y0 - y1).abs() < 0.01 {
                    // Horizontal: the top edge stays, the bottom edge moves; both fragments close.
                    // Outer strokes may be centered half a band inside the edge.
                    // Classify by their extent and retain that offset when closing.
                    let half_band = border.width.clamp(0.0, 12.0) / 2.0;
                    if row.bands.top_rules.binary_search(&source).is_ok() || *y0 - half_band <= 0.01 {
                        push(&mut a, &mut a_bands, source, it.clone());
                        push(&mut b, &mut b_bands, source, Placed::Rule { x0: *x0, y0: *y0, x1: *x1, y1: *y0, border: *border });
                    } else if row.bands.bottom_rules.binary_search(&source).is_ok() || *y0 + half_band >= row.height - 0.01 {
                        let y = head_height - (row.height - *y0);
                        push(&mut a, &mut a_bands, source, Placed::Rule { x0: *x0, y0: y, x1: *x1, y1: y, border: *border });
                        push(&mut b, &mut b_bands, source, Placed::Rule { x0: *x0, y0: y0 - shift, x1: *x1, y1: y1 - shift, border: *border });
                    } else if *y0 <= cut {
                        push(&mut a, &mut a_bands, source, it.clone());
                    } else {
                        push(&mut b, &mut b_bands, source, Placed::Rule { x0: *x0, y0: y0 - shift, x1: *x1, y1: y1 - shift, border: *border });
                    }
                } else {
                    let (t, bt) = (y0.min(*y1), y0.max(*y1));
                    let own = row.bands.frames.binary_search(&source).is_ok();
                    if own || t < cut {
                        push(
                            &mut a,
                            &mut a_bands,
                            source,
                            Placed::Rule {
                                x0: *x0,
                                y0: t,
                                x1: *x1,
                                y1: if own { head_height - (row.height - bt) } else { bt.min(cut) },
                                border: *border,
                            },
                        );
                    }
                    if bt > cut {
                        push(
                            &mut b,
                            &mut b_bands,
                            source,
                            Placed::Rule {
                                x0: *x0,
                                y0: if own { t } else { (t - shift).max(0.0) },
                                x1: *x1,
                                y1: if own { tail_height - (row.height - bt) } else { bt - shift },
                                border: *border,
                            },
                        );
                    }
                }
            }
        }
    }
    // Only the fragment containing a paragraph's last line owns its retained
    // space-after. Tail coordinates move by the same amount as their content.
    let paragraph_ends = |items: &[Placed], shift: f32| {
        items
            .iter()
            .filter_map(|item| {
                let Placed::Lines { path, para, l1, .. } = item else { return None };
                if *l1 != para.lines.len() {
                    return None;
                }
                row.bands.paragraph_ends.get(path).map(|end| (path.clone(), end - shift))
            })
            .collect()
    };
    a_bands.paragraph_ends = paragraph_ends(&a, 0.0);
    b_bands.paragraph_ends = paragraph_ends(&b, shift);
    Some((RowLayout { height: head_height, items: a, bands: a_bands }, RowLayout { height: tail_height, items: b, bands: b_bands }))
}

/// The parts of `r` above `cut` and below it, the latter moved up by `shift` (onto the
/// continuation of a row split at `cut`).
fn split_rect(r: Rect, cut: f32, shift: f32) -> (Option<Rect>, Option<Rect>) {
    let above = (r.y < cut).then(|| Rect::new(r.x, r.y, r.w, (r.bottom().min(cut) - r.y).max(0.0)));
    let top = (r.y - shift).max(0.0);
    let below = (r.bottom() > cut).then(|| Rect::new(r.x, top, r.w, (r.bottom() - shift - top).max(0.0)));
    (above, below)
}

/// A cell's region for table-style conditional formatting: (header row, total row, first column,
/// last column, banded row, banded column).
type Region = (bool, bool, bool, bool, bool, bool);

/// The region of the cell at grid column `g` spanning `span` of `ncols` columns in row `ri`.
fn region_of(t: &Table, style: Option<&TableStyleProps>, ri: usize, g: usize, span: usize, ncols: usize) -> Region {
    let (look, nrows) = (&t.props.look, t.rows.len());
    let first_col = look.first_column && g == 0;
    let last_col = look.last_column && g.saturating_add(span) >= ncols && ncols > 1;
    let col_band = look.banded_columns && !first_col && g.saturating_sub(usize::from(look.first_column)).is_multiple_of(2);
    (look.header_row && ri == 0, look.total_row && ri + 1 == nrows && nrows > 1, first_col, last_col, banded(t, style, ri), col_band)
}

/// Whether row `ri` is in an odd band of the table style's banded rows (bands of the style's
/// band size, counted below the header row).
fn banded(t: &Table, style: Option<&TableStyleProps>, ri: usize) -> bool {
    let look = &t.props.look;
    let size = style.and_then(|s| s.parts.band_size).unwrap_or(1).clamp(1, 1000) as usize;
    look.banded_rows && !(look.header_row && ri == 0) && (ri.saturating_sub(usize::from(look.header_row)) / size).is_multiple_of(2)
}

/// The table style's text formatting in `region`: the whole table's, then the column band's, the
/// row band's, the first/last column's and the header/total row's (later regions win,
/// ECMA-376 §17.7.6).
fn region_text(st: &TableStyleProps, (header, total, first_col, last_col, band, col_band): Region) -> CellText {
    let mut chr = st.chr.clone();
    if col_band {
        chr.overlay(&st.parts.col_band_chr);
    }
    if band {
        chr.overlay(&st.parts.band_chr);
    }
    if first_col {
        chr.overlay(&st.parts.first_col_chr);
    }
    if last_col {
        chr.overlay(&st.parts.last_col_chr);
    }
    if header {
        chr.overlay(&st.parts.header_chr);
    } else if total {
        chr.overlay(&st.parts.total_chr);
    }
    CellText { para: st.para.clone(), chr }
}

/// Each column's narrowest and widest content, cell margins included: its longest word and its
/// longest line. A cell spanning columns shares what they lack between them; nested tables count
/// with their own. `None` when every cell is empty.
fn column_content(ctx: &mut Ctx, t: &Table, style: Option<&TableStyleProps>, depth: usize) -> Option<Vec<(f32, f32)>> {
    let mut cols = vec![(0.0f32, 0.0f32); t.cols().max(1)];
    let mut spanning = Vec::new();
    let mut any = false;
    for (ri, row) in t.rows.iter().enumerate() {
        let mut g = 0usize;
        for cell in &row.cells {
            let span = cell.span();
            if cell.props.vmerge != VMerge::Continue {
                let text = style.map(|st| region_text(st, region_of(t, style, ri, g, cell.span(), cols.len())));
                let (lo, hi) = blocks_content(ctx, &cell.blocks, text.as_ref(), depth);
                any |= hi > 0.0;
                let (left, right) = horizontal_insets(t, style, ri, g, cell, cols.len());
                let need = (lo + left + right, hi + left + right);
                match cols.get_mut(g) {
                    Some(col) if span == 1 => *col = (col.0.max(need.0), col.1.max(need.1)),
                    _ => spanning.push((g, span, need)),
                }
            }
            g += span;
        }
    }
    for (g, span, need) in spanning {
        let Some(cs) = cols.get_mut(g..(g + span).min(t.cols())) else { continue };
        let n = cs.len().max(1) as f32;
        let (have_lo, have_hi) = cs.iter().fold((0.0, 0.0), |(l, h), c| (l + c.0, h + c.1));
        for c in cs {
            c.0 += (need.0 - have_lo).max(0.0) / n;
            c.1 += (need.1 - have_hi).max(0.0) / n;
        }
    }
    any.then_some(cols)
}

/// The narrowest and widest a block list can be laid out.
fn blocks_content(ctx: &mut Ctx, blocks: &Blocks, text: Option<&CellText>, depth: usize) -> (f32, f32) {
    let (mut lo, mut hi) = (0.0f32, 0.0f32);
    for b in blocks.iter() {
        let (l, h) = match &**b {
            Block::Para(p) => {
                let label = list_level(ctx, p).map(|l| (String::new(), l));
                para_content(&ctx.para_labelled(p, UNBOUNDED, text, &[], label))
            }
            // A nested table: its fixed width, else its columns' but at least its preferred width
            // (its own, else its grid's when it has preferred widths at all).
            Block::Table(nt) if depth < 8 => {
                let grid = if is_automatic(nt) { 0.0 } else { nt.grid.iter().filter(|w| w.is_finite()).map(|w| w.max(0.0)).sum() };
                let width = nt.props.width.filter(|w| w.is_finite()).unwrap_or(grid).clamp(0.0, UNBOUNDED);
                if nt.props.fixed && width > 0.0 {
                    (width, width)
                } else {
                    let style = table_style(ctx, nt);
                    let cols = column_content(ctx, nt, style.as_ref(), depth + 1).unwrap_or_default();
                    let proportional = percentage_content(nt, &cols);
                    let measured = proportional.as_deref().unwrap_or(&cols);
                    let (l, h) = measured.iter().fold((0.0, 0.0), |(l, h), c| (l + c.0, h + c.1));
                    let gaps = effective_spacing(nt, style.as_ref()).unwrap_or(0.0) * 2.0 * (measured.len() + 1) as f32;
                    (width.max(l + gaps), width.max(h + gaps))
                }
            }
            Block::Table(_) => (0.0, 0.0),
        };
        lo = lo.max(l);
        hi = hi.max(h);
    }
    (lo, hi)
}

/// The list level `p` is in, for its indents (without counting it: measuring isn't laying out).
fn list_level(ctx: &Ctx, p: &Paragraph) -> Option<wordcraft_doc::numbering::Level> {
    let n = p.props.numbering.or_else(|| ctx.doc.styles.resolve_para(&p.props).numbering).filter(|n| n.num != 0)?;
    ctx.doc.numbering.level(n.num, n.level).cloned()
}

/// A width to lay a paragraph out in so that only its own line breaks end lines.
const UNBOUNDED: f32 = 100_000.0;

/// A paragraph's longest word and longest line (between line breaks), indents included.
fn para_content(pl: &ParaLayout) -> (f32, f32) {
    let indents = pl.rp.indent_left.max(0.0) + pl.rp.indent_right.max(0.0) + pl.rp.indent_first.max(0.0);
    let (mut word, mut line, mut spaces) = (0.0f32, 0.0f32, 0.0f32);
    let (mut lo, mut hi) = (0.0f32, 0.0f32);
    for c in &pl.clusters {
        let adv = if c.adv.is_finite() { c.adv.clamp(0.0, UNBOUNDED) } else { 0.0 };
        match c.kind {
            ClKind::Space => {
                lo = lo.max(word);
                word = 0.0;
                spaces += adv;
            }
            ClKind::LineBreak | ClKind::PageBreak | ClKind::ColumnBreak => {
                lo = lo.max(word);
                hi = hi.max(line);
                (word, line, spaces) = (0.0, 0.0, 0.0);
            }
            _ => {
                word += adv;
                line += spaces + adv;
                spaces = 0.0;
                if c.break_after {
                    lo = lo.max(word);
                    word = 0.0;
                }
            }
        }
    }
    (lo.max(word) + indents, hi.max(line) + indents)
}

/// Whether nothing in `t` asks for a width (its own or any cell's): its columns then follow
/// their content alone.
fn is_automatic(t: &Table) -> bool {
    t.props.width.is_none()
        && t.props.width_pct.is_none()
        && t.rows.iter().all(|r| r.cells.iter().all(|c| c.props.width.is_none()))
        && (t.rows.iter().all(|r| r.cells.iter().all(|c| c.props.width_pct.is_none())) || percentage_proportions(t).is_some())
}

/// A complete percentage grid describes proportions even when the table's width is automatic.
/// For a spanning cell, retain its existing grid-column proportions. Incomplete/invalid
/// percentage constraints keep the established layout path instead of inventing missing values.
fn percentage_proportions(t: &Table) -> Option<Vec<f32>> {
    let n = t.cols().clamp(1, wordcraft_doc::table::MAX_COLS);
    let mut shares = vec![0.0f32; n];
    for row in &t.rows {
        let mut g = 0usize;
        for cell in &row.cells {
            let end = g.saturating_add(cell.span()).min(n);
            if let Some(percent) = cell.props.width_pct.filter(|p| p.is_finite() && *p > 0.0) {
                let slice = t.grid.get(g..end).unwrap_or(&[]);
                let sum: f32 = slice.iter().filter(|w| w.is_finite()).map(|w| w.max(0.0)).sum();
                for (i, share) in shares.get_mut(g..end).unwrap_or(&mut []).iter_mut().enumerate() {
                    let fraction = if sum > 0.0 { slice.get(i).copied().unwrap_or(0.0).max(0.0) / sum } else { 1.0 / (end - g).max(1) as f32 };
                    *share = share.max(percent.clamp(0.0, 100.0) * fraction);
                }
            }
            g = end;
        }
    }
    if shares.iter().any(|p| *p <= 0.0 || !p.is_finite()) {
        return None;
    }
    let sum: f32 = shares.iter().sum();
    if !sum.is_finite() || sum <= 0.0 {
        return None;
    }
    shares.iter_mut().for_each(|p| *p /= sum);
    Some(shares)
}

/// Natural percentage widths preserve the requested shares, while minimum widths remain
/// the actual per-column minima. If the natural total does not fit, those minima take
/// priority over percentages (Word Arial 90/10 long-word controls).
fn percentage_content(t: &Table, content: &[(f32, f32)]) -> Option<Vec<(f32, f32)>> {
    let shares = percentage_proportions(t)?;
    if shares.len() != content.len() {
        return None;
    }
    let natural = shares.iter().zip(content).fold(0.0f32, |n, (share, c)| n.max(c.1.max(c.0) / share));
    if !natural.is_finite() {
        return None;
    }
    Some(shares.into_iter().zip(content).map(|(share, c)| (c.0, natural * share)).collect())
}

/// Expand deficient columns to their real minima and take that room proportionally from
/// columns with spare width. Used by preferred grids and constrained percentage tables.
fn fit_column_minimums(grid: &mut [f32], content: &[(f32, f32)]) {
    let short: f32 = grid.iter().zip(content).map(|(w, c)| (c.0 - *w).max(0.0)).sum();
    if short < 0.01 {
        return;
    }
    let spare: f32 = grid.iter().zip(content).map(|(w, c)| (*w - c.0).max(0.0)).sum();
    let give = (short / spare.max(0.01)).min(1.0);
    for (w, c) in grid.iter_mut().zip(content) {
        *w = if *w < c.0 { c.0 } else { *w - (*w - c.0) * give };
    }
}

/// Word's autofit for a table whose widths may follow its content (`tblLayout` not fixed).
/// - With no preferred width anywhere (the table's and every cell's automatic) the columns come
///   from their content alone: each its widest line if they all fit in `avail`, else shared out
///   between narrowest and widest in proportion to how much each can give.
/// - Otherwise the grid stands, but no column is narrower than its longest word: the columns with
///   room to spare give up the difference.
fn autofit(grid: &mut [f32], content: &[(f32, f32)], t: &Table, avail: f32) {
    if grid.len() != content.len() || !avail.is_finite() {
        return;
    }
    if is_automatic(t) {
        if let Some(proportional) = percentage_content(t, content) {
            let natural: f32 = proportional.iter().map(|c| c.1).sum();
            let ratio = if natural > avail { avail / natural.max(0.01) } else { 1.0 };
            for (w, c) in grid.iter_mut().zip(&proportional) {
                *w = (c.1 * ratio).max(4.0);
            }
            fit_column_minimums(grid, &proportional);
            return;
        }
        let (lo, hi) = content.iter().fold((0.0f32, 0.0f32), |(l, h), c| (l + c.0, h + c.1.max(c.0)));
        let k = if hi <= avail {
            1.0
        } else if lo >= avail || hi - lo < 0.01 {
            0.0
        } else {
            (avail - lo) / (hi - lo)
        };
        for (w, c) in grid.iter_mut().zip(content) {
            *w = (c.0 + (c.1.max(c.0) - c.0) * k).max(4.0);
        }
        return;
    }
    fit_column_minimums(grid, content);
}

#[cfg(test)]
mod wordneo_autofit_tests {
    use super::*;
    use std::sync::Arc;
    use wordcraft_doc::{Document, para_block};

    /// Matches split-deep's synthetic Word controls: Letter, Calibri12,
    /// single-spaced manual breaks, no paragraph before/after, 0.5pt borders.
    fn split_margin_document(cells: &[(usize, f32, f32)], after: f32, header: bool, shaded: bool) -> Document {
        let mut doc = Document::new();
        doc.last_section.page_h = 792.0;
        doc.last_section.margin_top = 72.0;
        doc.last_section.margin_bottom = 72.0;
        doc.styles.default_chr.font = Some("Calibri".into());
        doc.styles.default_chr.size = Some(12.0);
        doc.styles.default_para.space_before = Some(0.0);
        doc.styles.default_para.space_after = Some(0.0);
        doc.styles.default_para.line_spacing = Some(wordcraft_doc::props::LineSpacing::Multiple(1.0));
        let mut table = Table::new(1 + usize::from(header), cells.len(), 300.0);
        table.props.fixed = true;
        table.props.cell_margins = Some([0.0; 4]);
        table.props.borders = Some(Borders::all(Border::single(0.5)));
        if header {
            let row = &mut table.rows[0];
            row.props.header = true;
            row.props.height = Some(20.0);
            row.props.height_rule = HeightRule::Exact;
            for cell in &mut row.cells {
                cell.props.borders = Some(Borders { bottom: Some(Border::single(1.0)), ..Default::default() });
                cell.blocks = vec![para_block(Paragraph::with_text("HEADER", Default::default()))];
            }
        }
        for (cell, &(lines, top, bottom)) in table.rows[usize::from(header)].cells.iter_mut().zip(cells) {
            cell.props.margins = Some([top, 0.0, bottom, 0.0]);
            if shaded {
                cell.props.shading = Some(Rgb(210, 220, 230));
            }
            let text = (1..=lines).map(|i| format!("LINE {i:03}")).collect::<Vec<_>>().join("\n");
            let mut para = Paragraph::with_text(&text, Default::default());
            para.props.space_after = Some(after);
            cell.blocks = vec![para_block(para)];
        }
        doc.body = vec![Arc::new(Block::Table(table))];
        doc
    }

    fn split_margin_layout(doc: &Document) -> crate::DocLayout {
        crate::layout(doc, &mut crate::LayoutCache::new(), &crate::LayoutOptions::default())
    }

    /// Absolute baseline, line-box bottom and logical line index, in placement order.
    fn split_cell_lines(layout: &crate::DocLayout, page: usize, row: u32, cell: u32) -> Vec<(f32, f32, usize)> {
        layout.pages[page]
            .items
            .iter()
            .filter_map(|item| match item {
                Placed::Lines { path, para, l0, l1, y, .. } if path.0 == [0, row, cell, 0] => Some((*y, para, *l0, *l1)),
                _ => None,
            })
            .flat_map(|(y, para, l0, l1)| {
                let base = para.lines[l0].top;
                (l0..l1).map(move |i| {
                    let line = &para.lines[i];
                    (y + line.baseline - base, y + line.top - base + line.height, i)
                })
            })
            .collect()
    }

    /// Word two-cells-top-0-6: both cells start at the row's maximum top
    /// margin already on page1, before any split or continuation takes place.
    #[test]
    fn split_vertical_top_margin_is_shared_by_the_row() {
        let mixed = split_margin_layout(&split_margin_document(&[(1, 0.0, 0.0), (1, 6.0, 0.0)], 0.0, false, false));
        let both = split_margin_layout(&split_margin_document(&[(1, 6.0, 0.0), (1, 6.0, 0.0)], 0.0, false, false));
        for cell in 0..2 {
            let actual = split_cell_lines(&mixed, 0, 0, cell)[0].0;
            let expected = split_cell_lines(&both, 0, 0, cell)[0].0;
            assert!((actual - expected).abs() < 0.01, "cell{cell}: row maxTop6 must apply to both cells: {actual} vs {expected}");
        }
    }

    /// Word bottom0-long-bottom6-short: the short cell's padding reserves
    /// six points below the long cell as well; its content length is irrelevant.
    #[test]
    fn split_vertical_bottom_margin_includes_a_short_cell() {
        let mixed = split_margin_layout(&split_margin_document(&[(5, 0.0, 0.0), (1, 0.0, 6.0)], 0.0, false, false));
        let both = split_margin_layout(&split_margin_document(&[(5, 0.0, 6.0), (1, 0.0, 6.0)], 0.0, false, false));
        let bottom = |layout: &crate::DocLayout| {
            layout.pages[0]
                .items
                .iter()
                .filter_map(|item| match item {
                    Placed::Cell { rect, .. } => Some(rect.bottom()),
                    _ => None,
                })
                .fold(0.0f32, f32::max)
        };
        assert!((bottom(&mixed) - bottom(&both)).abs() < 0.01, "row maxBottom6 must survive a short cell: {} vs {}", bottom(&mixed), bottom(&both));
    }

    /// Word's bottom6 controls fit43 lines, whereas paragraph-after6 keeps44
    /// on page1 and applies the six points only to the final paragraph fragment.
    #[test]
    fn split_reserves_cell_bottom_padding_without_repeating_paragraph_after() {
        for cells in [vec![(75, 0.0, 6.0)], vec![(75, 0.0, 0.0), (1, 0.0, 6.0)]] {
            let layout = split_margin_layout(&split_margin_document(&cells, 0.0, false, false));
            assert_eq!(split_cell_lines(&layout, 0, 0, 0).len(), 43, "Word reserves maxBottom6 even when only the short cell supplies it");
            assert_eq!(split_cell_lines(&layout, 1, 0, 0).len(), 32);
        }
        let plain = split_margin_layout(&split_margin_document(&[(75, 0.0, 0.0)], 0.0, false, false));
        let after = split_margin_layout(&split_margin_document(&[(75, 0.0, 0.0)], 6.0, false, false));
        assert_eq!(split_cell_lines(&after, 0, 0, 0).len(), 44, "unfinished paragraph-after is not a per-fragment bottom margin");
        let final_bottom = |layout: &crate::DocLayout| {
            layout
                .pages
                .last()
                .unwrap()
                .items
                .iter()
                .filter_map(|item| match item {
                    Placed::Cell { rect, .. } => Some(rect.bottom()),
                    _ => None,
                })
                .fold(0.0f32, f32::max)
        };
        assert!((final_bottom(&after) - final_bottom(&plain) - 6.0).abs() < 0.01, "preserve paragraph-after6 on the final fragment");
    }

    /// Word's base/top6 controls repeat their first-line baseline on page2.
    #[test]
    fn split_fragments_repeat_the_real_top_margin() {
        for top in [0.0, 6.0] {
            let layout = split_margin_layout(&split_margin_document(&[(75, top, 0.0)], 0.0, false, false));
            assert_eq!(layout.pages.len(), 2);
            let first = split_cell_lines(&layout, 0, 0, 0)[0].0;
            let tail = split_cell_lines(&layout, 1, 0, 0)[0].0;
            assert!((first - tail).abs() < 0.01, "top{top}: repeat the real leading inset: {first} vs {tail}");
        }
    }

    /// Word closes a partial row after its retained lines and padding, not at
    /// the page cap. Real cell shading must end before the closing border band.
    #[test]
    fn split_fragments_close_at_natural_content_end_with_owned_shading() {
        for (top, bottom) in [(0.0, 0.0), (6.0, 0.0), (0.0, 6.0)] {
            let layout = split_margin_layout(&split_margin_document(&[(75, top, bottom)], 0.0, false, true));
            assert_eq!(layout.pages.len(), 2);
            let first = split_cell_lines(&layout, 0, 0, 0);
            let last = split_cell_lines(&layout, 1, 0, 0);
            let mut gaps = Vec::new();
            let mut fill_deltas = Vec::new();
            for (pi, lines) in [first, last].iter().enumerate() {
                let border = layout.pages[pi]
                    .items
                    .iter()
                    .filter_map(|item| match item {
                        Placed::Rule { y0, y1, border, .. } if (y0 - y1).abs() < 0.01 => Some((*y0, border.width)),
                        _ => None,
                    })
                    .max_by(|a, b| a.0.total_cmp(&b.0))
                    .unwrap();
                gaps.push(border.0 - border.1 / 2.0 - lines.last().unwrap().0);
                let fill_bottom = layout.pages[pi]
                    .items
                    .iter()
                    .find_map(|item| match item {
                        Placed::Fill { rect, color } if *color == Rgb(210, 220, 230) => Some(rect.bottom()),
                        _ => None,
                    })
                    .unwrap();
                fill_deltas.push(fill_bottom - (border.0 - border.1 / 2.0));
            }
            assert!(
                (gaps[0] - gaps[1]).abs() < 0.01 && fill_deltas.iter().all(|d| d.abs() < 0.01),
                "top{top}/bottom{bottom}: natural baseline-to-border gaps must agree and owned shading must stop before that band: gaps={gaps:?}, fill excess={fill_deltas:?}"
            );
        }
    }

    /// Word header20/bottom1 + body75/ownTop0.5: every body fragment begins
    /// below the physical header's one-point band, including a partial tail.
    #[test]
    fn split_continuation_uses_the_repeated_headers_physical_border() {
        let layout = split_margin_layout(&split_margin_document(&[(75, 0.0, 0.0)], 0.0, true, false));
        assert_eq!(layout.pages.len(), 2);
        let first = split_cell_lines(&layout, 0, 1, 0);
        let tail = split_cell_lines(&layout, 1, 1, 0);
        assert_eq!((first.len(), tail.len()), (42, 33));
        assert!(
            (first[0].0 - tail[0].0).abs() < 0.01,
            "a split tail must use header-bottom1, not an invented 2pt inset: {} vs {}",
            first[0].0,
            tail[0].0
        );
        assert_eq!(first.iter().chain(&tail).map(|line| line.2).collect::<Vec<_>>(), (0..75).collect::<Vec<_>>());
    }

    /// Word's75-complete-paragraphs/after6 control retains the completed
    /// paragraph's six-point space before each fragment's closing border.
    #[test]
    fn split_keeps_space_after_a_completed_paragraph_at_the_page_edge() {
        let mut doc = split_margin_document(&[(75, 0.0, 0.0)], 0.0, false, false);
        let Block::Table(table) = Arc::make_mut(&mut doc.body[0]) else { panic!("table fixture") };
        table.rows[0].cells[0].blocks = (1..=75)
            .map(|i| {
                let mut para = Paragraph::with_text(&format!("LINE {i:03}"), Default::default());
                para.props.space_after = Some(6.0);
                para_block(para)
            })
            .collect();
        let layout = split_margin_layout(&doc);
        assert_eq!(layout.pages.len(), 3, "Word's measured control has three pages");
        let mut counts = Vec::new();
        let mut gaps = Vec::new();
        for page in &layout.pages {
            let mut last_baseline = 0.0f32;
            let mut count = 0;
            for item in &page.items {
                if let Placed::Lines { para, l0, l1, y, .. } = item {
                    let base = para.lines[*l0].top;
                    for line in &para.lines[*l0..*l1] {
                        last_baseline = last_baseline.max(y + line.baseline - base);
                        count += 1;
                    }
                }
            }
            let border_top = page
                .items
                .iter()
                .filter_map(|item| match item {
                    Placed::Rule { y0, y1, border, .. } if (y0 - y1).abs() < 0.01 => Some(y0 - border.width / 2.0),
                    _ => None,
                })
                .fold(0.0f32, f32::max);
            counts.push(count);
            gaps.push(border_top - last_baseline);
        }
        assert_eq!(counts, [31, 31, 13], "preserve Word's ordered paragraph distribution");
        let final_gap = *gaps.last().unwrap();
        assert!(gaps.iter().all(|gap| (gap - final_gap).abs() < 0.01), "completed paragraph-after6 must remain before each closing border: {gaps:?}");
    }

    /// The same Word 75-paragraph/after6 control with an 84pt bottom margin
    /// fits 30/30/15: a completed paragraph's after must also fit below its line.
    #[test]
    fn split_fits_a_completed_paragraph_after_before_the_page_cap() {
        let mut doc = split_margin_document(&[(75, 0.0, 0.0)], 0.0, false, false);
        doc.last_section.margin_bottom = 84.0;
        let Block::Table(table) = Arc::make_mut(&mut doc.body[0]) else { panic!("table fixture") };
        table.rows[0].cells[0].blocks = (1..=75)
            .map(|i| {
                let mut para = Paragraph::with_text(&format!("LINE {i:03}"), Default::default());
                para.props.space_after = Some(6.0);
                para_block(para)
            })
            .collect();
        let layout = split_margin_layout(&doc);
        let counts: Vec<usize> = layout
            .pages
            .iter()
            .map(|page| {
                page.items
                    .iter()
                    .filter_map(|item| match item {
                        Placed::Lines { l0, l1, .. } => Some(l1 - l0),
                        _ => None,
                    })
                    .sum()
            })
            .collect();
        assert_eq!(counts, [30, 30, 15], "Word moves a completed paragraph when its after would cross the 708pt cap");
        for (pi, page) in layout.pages.iter().enumerate() {
            let bottom = page
                .items
                .iter()
                .find_map(|item| match item {
                    Placed::Cell { rect, table, row: 0, cell: 0, .. } if table.0 == [0] => Some(rect.bottom()),
                    _ => None,
                })
                .expect("one outer cell per fragment");
            assert!(
                bottom <= page.body.bottom() + 0.01,
                "page{pi}: retained paragraph-after and the closing frame must fit: {bottom} vs {}",
                page.body.bottom()
            );
        }
    }

    fn split_spacing_fragment_bottoms(doc: &Document) -> Vec<f32> {
        split_margin_layout(doc)
            .pages
            .iter()
            .map(|page| {
                page.items
                    .iter()
                    .find_map(|item| match item {
                        Placed::Cell { rect, table, row: 0, cell: 0, .. } if table.0 == [0] => Some(rect.bottom()),
                        _ => None,
                    })
                    .expect("one outer cell per fragment")
            })
            .collect()
    }

    /// Structural guard: spacing suppressed between same-style paragraphs must
    /// close every fragment like an explicit zero-after control.
    #[test]
    fn split_does_not_restore_contextual_paragraph_after() {
        let fixture = |after: f32| {
            let mut doc = split_margin_document(&[(75, 0.0, 0.0)], 0.0, false, false);
            let Block::Table(table) = Arc::make_mut(&mut doc.body[0]) else { panic!("table fixture") };
            table.rows[0].cells[0].blocks = (1..=75)
                .map(|i| {
                    let mut para = Paragraph::with_text(&format!("LINE {i:03}"), Default::default());
                    para.props.space_after = Some(if i == 75 { 6.0 } else { after });
                    para.props.contextual_spacing = Some(true);
                    para_block(para)
                })
                .collect();
            doc
        };
        let expected = split_spacing_fragment_bottoms(&fixture(0.0));
        let actual = split_spacing_fragment_bottoms(&fixture(6.0));
        assert_eq!(expected.len(), 2, "the control crosses a page boundary");
        assert!(
            actual.len() == expected.len() && actual.iter().zip(&expected).all(|(a, b)| (a - b).abs() < 0.01),
            "contextual after removed from the flow must stay removed at each fragment edge: {actual:?} vs {expected:?}"
        );
    }

    /// Structural guard: automatic spacing removed between list items, and at
    /// the cell's end, must not reappear when a row is split.
    #[test]
    fn split_does_not_restore_automatic_list_after() {
        let fixture = |automatic: bool| {
            let mut doc = split_margin_document(&[(75, 0.0, 0.0)], 0.0, false, false);
            let num = doc.numbering.add_list(wordcraft_doc::ListKind::Numbered);
            let Block::Table(table) = Arc::make_mut(&mut doc.body[0]) else { panic!("table fixture") };
            table.rows[0].cells[0].blocks = (1..=75)
                .map(|i| {
                    let mut para = Paragraph::with_text(&format!("LINE {i:03}"), Default::default());
                    para.props.space_after = Some(0.0);
                    para.props.space_after_auto = Some(automatic);
                    para.props.numbering = Some(wordcraft_doc::props::NumRef { num, level: 0 });
                    para_block(para)
                })
                .collect();
            doc
        };
        let expected = split_spacing_fragment_bottoms(&fixture(false));
        let actual = split_spacing_fragment_bottoms(&fixture(true));
        assert_eq!(expected.len(), 2, "the list control crosses a page boundary");
        assert!(
            actual.len() == expected.len() && actual.iter().zip(&expected).all(|(a, b)| (a - b).abs() < 0.01),
            "automatic list after removed from the flow must stay removed at each fragment edge: {actual:?} vs {expected:?}"
        );
    }

    /// A third fragment must retain the same real margins and border ownership
    /// as the second. This is the repeated application of the measured two-page
    /// contract, not an independently measured Word150-lines pagination count.
    #[test]
    fn split_margin_metadata_survives_three_fragments() {
        let layout = split_margin_layout(&split_margin_document(&[(150, 6.0, 6.0)], 0.0, false, true));
        assert!(layout.pages.len() >= 3);
        let mut indices = Vec::new();
        let first = split_cell_lines(&layout, 0, 0, 0)[0].0;
        for pi in 0..layout.pages.len() {
            let lines = split_cell_lines(&layout, pi, 0, 0);
            assert!(!lines.is_empty());
            assert!((lines[0].0 - first).abs() < 0.01, "page{pi}: maxTop6 must survive repeated splitting: {} vs {first}", lines[0].0);
            indices.extend(lines.into_iter().map(|line| line.2));
        }
        assert_eq!(indices, (0..150).collect::<Vec<_>>(), "each logical line is placed once and in order");
    }

    fn row_band_baselines(top: bool, height: Option<(f32, HeightRule)>, widths: [f32; 2]) -> [f32; 3] {
        let mut doc = Document::new();
        doc.styles.default_chr.font = Some("Calibri".into());
        doc.styles.default_chr.size = Some(11.0);
        doc.styles.default_para.space_after = Some(0.0);
        let mut table = Table::new(1, 2, 300.0);
        table.props.fixed = true;
        table.props.cell_margins = Some([0.0; 4]);
        table.props.borders = Some(Borders::all(Border::single(0.0)));
        if let Some((h, rule)) = height {
            table.rows[0].props.height = Some(h);
            table.rows[0].props.height_rule = rule;
        }
        for (cell, (size, width, text)) in table.rows[0].cells.iter_mut().zip([(12.0, widths[0], "Tall text"), (11.0, widths[1], "Small text")]) {
            let mut borders = Borders::all(Border::single(0.0));
            if top {
                borders.top = Some(Border::single(width));
            } else {
                borders.bottom = Some(Border::single(width));
            }
            cell.props.borders = Some(borders);
            cell.blocks = vec![para_block(Paragraph::with_text(text, wordcraft_doc::props::CharProps { size: Some(size), ..Default::default() }))];
        }
        doc.body = vec![Arc::new(Block::Table(table)), para_block(Paragraph::with_text("AFTER_TABLE", Default::default()))];
        let layout = crate::layout(&doc, &mut crate::LayoutCache::new(), &crate::LayoutOptions::default());
        let mut bases = [None; 3];
        for item in &layout.pages[0].items {
            if let Placed::Lines { path, para, l0, y, .. } = item {
                let index = if path.0 == [0, 0, 0, 0] {
                    Some(0)
                } else if path.0 == [0, 0, 1, 0] {
                    Some(1)
                } else if path.0 == [1] {
                    Some(2)
                } else {
                    None
                };
                if let Some(i) = index {
                    let line = &para.lines[*l0];
                    bases[i] = Some(*y + line.baseline - line.top);
                }
            }
        }
        bases.map(Option::unwrap)
    }

    /// Word's 18-case Calibri control: a top border in the smaller-font cell moves
    /// both cell baselines by its width. The band is shared by the row, not per cell.
    #[test]
    fn row_top_border_band_is_shared_across_cells_with_different_font_sizes() {
        for (name, height, after_delta) in
            [("auto", None, 0.5), ("exact", Some((20.0, HeightRule::Exact)), 0.0), ("atleast", Some((20.0, HeightRule::AtLeast)), 0.5)]
        {
            let none = row_band_baselines(true, height, [0.0, 0.0]);
            let mixed = row_band_baselines(true, height, [0.0, 0.5]);
            let both = row_band_baselines(true, height, [0.5, 0.5]);
            for (i, expected) in [0.5, 0.5, after_delta].into_iter().enumerate() {
                assert!((mixed[i] - none[i] - expected).abs() < 0.01, "{name} marker{i}: none={none:?} mixed={mixed:?}; expected delta{expected}");
                assert!((both[i] - mixed[i]).abs() < 0.01, "{name} marker{i}: the same shared band must not be counted twice: {mixed:?} vs {both:?}");
            }
        }
    }

    /// The last bottom border advances the following paragraph without moving cell
    /// baselines, including exact-height rows. Mixed and fully-bordered rows agree.
    #[test]
    fn row_final_bottom_border_band_is_shared_and_survives_exact_height() {
        for (name, height) in [("auto", None), ("exact", Some((20.0, HeightRule::Exact))), ("atleast", Some((20.0, HeightRule::AtLeast)))] {
            let none = row_band_baselines(false, height, [0.0, 0.0]);
            let mixed = row_band_baselines(false, height, [0.0, 0.5]);
            let both = row_band_baselines(false, height, [0.5, 0.5]);
            for (i, expected) in [0.0, 0.0, 0.5].into_iter().enumerate() {
                assert!((mixed[i] - none[i] - expected).abs() < 0.01, "{name} marker{i}: none={none:?} mixed={mixed:?}; expected delta{expected}");
                assert!((both[i] - mixed[i]).abs() < 0.01, "{name} marker{i}: the same shared band must not be counted twice: {mixed:?} vs {both:?}");
            }
        }
    }

    fn interior_band_baselines(previous_bottom: f32, next_top: f32) -> [f32; 3] {
        let mut doc = Document::new();
        doc.styles.default_chr.font = Some("Calibri".into());
        doc.styles.default_chr.size = Some(12.0);
        doc.styles.default_para.space_after = Some(0.0);
        let mut table = Table::new(2, 1, 300.0);
        table.props.fixed = true;
        table.props.cell_margins = Some([0.0; 4]);
        table.props.borders = Some(Borders::all(Border::single(0.0)));
        for (ri, row) in table.rows.iter_mut().enumerate() {
            let cell = &mut row.cells[0];
            let mut borders = Borders::all(Border::single(0.0));
            if ri == 0 {
                borders.bottom = Some(Border::single(previous_bottom));
            } else {
                borders.top = Some(Border::single(next_top));
            }
            cell.props.borders = Some(borders);
            let chr = wordcraft_doc::props::CharProps {
                font: Some(if ri == 0 { "Cambria" } else { "Calibri" }.into()),
                size: Some(12.0),
                bold: Some(ri == 0),
                ..Default::default()
            };
            cell.blocks = vec![para_block(Paragraph::with_text("Hg text", chr))];
        }
        doc.body = vec![Arc::new(Block::Table(table)), para_block(Paragraph::with_text("AFTER_TABLE", Default::default()))];
        let layout = crate::layout(&doc, &mut crate::LayoutCache::new(), &crate::LayoutOptions::default());
        let mut bases = [None; 3];
        for item in &layout.pages[0].items {
            if let Placed::Lines { path, para, l0, y, .. } = item {
                let index = if path.0 == [0, 0, 0, 0] {
                    Some(0)
                } else if path.0 == [0, 1, 0, 0] {
                    Some(1)
                } else if path.0 == [1] {
                    Some(2)
                } else {
                    None
                };
                if let Some(i) = index {
                    let line = &para.lines[*l0];
                    bases[i] = Some(*y + line.baseline - line.top);
                }
            }
        }
        bases.map(Option::unwrap)
    }

    /// Word's Cambria/Calibri first-row control: a 1pt bottom border reserves
    /// the same next-row band as an explicit 1pt top border, without doubling it.
    #[test]
    fn interior_row_band_includes_previous_bottom_border_once() {
        let none = interior_band_baselines(0.0, 0.0);
        let previous = interior_band_baselines(1.0, 0.0);
        let next = interior_band_baselines(0.0, 1.0);
        let both = interior_band_baselines(1.0, 1.0);
        for (i, expected) in [0.0, 1.0, 1.0].into_iter().enumerate() {
            assert!((previous[i] - none[i] - expected).abs() < 0.01, "marker{i}: previous-bottom band missing: none={none:?}, previous={previous:?}");
            assert!((next[i] - previous[i]).abs() < 0.01, "marker{i}: top/bottom must reserve the same band: {next:?} vs {previous:?}");
            assert!((both[i] - previous[i]).abs() < 0.01, "marker{i}: shared interior band counted twice: {both:?} vs {previous:?}");
        }
    }

    /// The border center is half a band inside the outer row edge. Splitting a
    /// tall row must recognize those strokes as edges and close both fragments.
    #[test]
    fn split_row_closes_fragments_when_border_centers_are_inside_edge_bands() {
        let doc = Document::from_text("Line");
        let layout = crate::layout(&doc, &mut crate::LayoutCache::new(), &crate::LayoutOptions::default());
        let line = layout.pages[0].items.iter().find(|item| matches!(item, Placed::Lines { .. })).unwrap().clone();
        let placed_line = |y| {
            let mut item = line.clone();
            if let Placed::Lines { x, y: py, .. } = &mut item {
                *x = 0.0;
                *py = y;
            }
            item
        };
        for width in [0.5, 2.0, 12.0] {
            let border = Border::single(width);
            let top = width / 2.0;
            let bottom = 200.0 - top;
            let row = RowLayout {
                height: 200.0,
                items: vec![
                    placed_line(20.0),
                    placed_line(150.0),
                    Placed::Rule { x0: 0.0, y0: top, x1: 100.0, y1: top, border },
                    Placed::Rule { x0: 0.0, y0: bottom, x1: 100.0, y1: bottom, border },
                ],
                bands: Default::default(),
            };
            let (head, tail) = split_row(&row, 100.0).expect("the two lines straddle the cut");
            for (name, fragment) in [("head", head), ("tail", tail)] {
                let ys: Vec<_> = fragment
                    .items
                    .iter()
                    .filter_map(|item| match item {
                        Placed::Rule { y0, y1, .. } if (y0 - y1).abs() < 0.01 => Some(*y0),
                        _ => None,
                    })
                    .collect();
                assert_eq!(ys.len(), 2, "{name}/{width}pt: top and bottom must both close: {ys:?}");
                assert!(ys.iter().any(|y| (y - width / 2.0).abs() < 0.01), "{name}: keep the top stroke inside its band: {ys:?}");
                assert!(
                    ys.iter().any(|y| (y + width / 2.0 - fragment.height).abs() < 0.01),
                    "{name}: keep the bottom stroke inside its band: {ys:?}"
                );
            }
        }
    }

    /// Word Exact30 controls: previous-bottom2 contributes between rows on one
    /// page, but is absent when the next row moves whole to the following page.
    #[test]
    fn inherited_row_border_band_is_not_repeated_after_page_break() {
        let measure = |page_height, border_width, valign| {
            let mut doc = Document::new();
            doc.last_section.page_h = page_height;
            doc.last_section.margin_top = 72.0;
            doc.last_section.margin_bottom = 72.0;
            doc.styles.default_chr.font = Some("Calibri".into());
            doc.styles.default_chr.size = Some(12.0);
            doc.styles.default_para.space_after = Some(0.0);
            let mut table = Table::new(2, 1, 300.0);
            table.props.fixed = true;
            table.props.cell_margins = Some([0.0; 4]);
            table.props.borders = Some(Borders::all(Border::single(0.0)));
            for (ri, row) in table.rows.iter_mut().enumerate() {
                row.props.height = Some(30.0);
                row.props.height_rule = HeightRule::Exact;
                row.cells[0].blocks = vec![para_block(Paragraph::with_text("Hjg", Default::default()))];
                let mut borders = Borders::all(Border::single(0.0));
                if ri == 0 {
                    borders.bottom = Some(Border::single(border_width));
                }
                row.cells[0].props.borders = Some(borders);
                if ri == 1 {
                    row.cells[0].props.valign = valign;
                }
            }
            doc.body = vec![
                Arc::new(Block::Table(table)),
                para_block(Paragraph::with_text("AFTER_TABLE", wordcraft_doc::props::CharProps { size: Some(1.0), ..Default::default() })),
            ];
            let layout = crate::layout(&doc, &mut crate::LayoutCache::new(), &crate::LayoutOptions::default());
            let mut bases = [None; 3];
            let mut lower_height = None;
            for (pi, page) in layout.pages.iter().enumerate() {
                for item in &page.items {
                    if let Placed::Lines { path, para, l0, y, .. } = item {
                        let index = if path.0 == [0, 0, 0, 0] {
                            Some(0)
                        } else if path.0 == [0, 1, 0, 0] {
                            Some(1)
                        } else if path.0 == [1] {
                            Some(2)
                        } else {
                            None
                        };
                        if let Some(i) = index {
                            let line = &para.lines[*l0];
                            bases[i] = Some((pi, *y + line.baseline - line.top));
                        }
                    }
                    if let Placed::Cell { row: 1, rect, .. } = item {
                        lower_height = Some(rect.h);
                    }
                }
            }
            (bases.map(Option::unwrap), lower_height.unwrap())
        };
        let (same_page, _) = measure(240.0, 2.0, VAlign::Top);
        assert_eq!((same_page[0].0, same_page[1].0), (0, 0));
        assert!((same_page[1].1 - same_page[0].1 - 32.0).abs() < 0.01, "retain the 2pt band between adjacent rows: {same_page:?}");
        for valign in [VAlign::Top, VAlign::Center, VAlign::Bottom] {
            let (same_page, _) = measure(240.0, 2.0, valign);
            let (none, plain_height) = measure(200.0, 0.0, valign);
            let (split, split_height) = measure(200.0, 2.0, valign);
            assert_eq!((split[0].0, split[1].0), (0, 1));
            assert!(
                (split[1].1 - (same_page[1].1 - 30.0) + 2.0).abs() < 0.01,
                "{valign:?}: remove only the inherited band while preserving alignment slack: {same_page:?} vs {split:?}"
            );
            assert!((split[2].1 - none[2].1 + 2.0).abs() < 0.01, "the following paragraph must lose the cropped 2pt advance: {none:?} vs {split:?}");
            assert!(
                (split_height - plain_height + 2.0).abs() < 0.01,
                "Word crops the inherited band from the effective Exact advance: {split_height} vs {plain_height}"
            );
        }
    }

    /// The new physical predecessor is the repeated header, not the data row
    /// on the previous page (Word header20/bottom1, data30/bottom2 controls).
    #[test]
    fn repeated_header_replaces_the_previous_data_rows_border_band() {
        for previous_bottom in [0.0, 2.0] {
            let mut doc = Document::new();
            doc.last_section.page_h = 210.0;
            doc.last_section.margin_top = 72.0;
            doc.last_section.margin_bottom = 72.0;
            doc.styles.default_chr.font = Some("Calibri".into());
            doc.styles.default_chr.size = Some(12.0);
            doc.styles.default_para.space_after = Some(0.0);
            let mut table = Table::new(3, 1, 300.0);
            table.props.fixed = true;
            table.props.cell_margins = Some([0.0; 4]);
            table.props.borders = Some(Borders::all(Border::single(0.0)));
            for (ri, row) in table.rows.iter_mut().enumerate() {
                row.props.height = Some(if ri == 0 { 20.0 } else { 30.0 });
                row.props.height_rule = HeightRule::Exact;
                row.props.header = ri == 0;
                let mut borders = Borders::all(Border::single(0.0));
                borders.bottom = Some(Border::single([1.0, previous_bottom, 0.0][ri]));
                row.cells[0].props.borders = Some(borders);
                row.cells[0].blocks = vec![para_block(Paragraph::with_text("Hjg", Default::default()))];
            }
            doc.body = vec![Arc::new(Block::Table(table))];
            let layout = crate::layout(&doc, &mut crate::LayoutCache::new(), &crate::LayoutOptions::default());
            assert_eq!(layout.pages.len(), 2);
            let base = |pi: usize, row: u32| {
                layout.pages[pi]
                    .items
                    .iter()
                    .find_map(|item| match item {
                        Placed::Lines { path, para, l0, y, .. } if path.0 == [0, row, 0, 0] => {
                            let line = &para.lines[*l0];
                            Some(*y + line.baseline - line.top)
                        }
                        _ => None,
                    })
                    .unwrap()
            };
            assert!((base(0, 0) - base(1, 0)).abs() < 0.01, "the same header is repeated at the same baseline");
            assert!((base(0, 1) - base(1, 2)).abs() < 0.01, "both data rows must use header-bottom1 at the physical page boundary");
        }
    }

    /// Word's firstCol controls: nil/none insideH in the conditional region
    /// leave the global grid; visible insideH overrides it. Direct top/bottom
    /// cancellation and conditional outer-edge cancellation retain their meaning.
    #[test]
    fn conditional_first_column_inside_nil_keeps_global_horizontal_grid() {
        use wordcraft_doc::props::BorderStyle;
        use wordcraft_doc::styles::{Style, StyleKind};
        for (name, inside) in [
            ("nil", Border { style: BorderStyle::None, ..Border::single(0.5) }),
            ("none", Border { style: BorderStyle::None, ..Border::single(0.5) }),
            ("single2", Border::single(2.0)),
        ] {
            for direct_cancel in [false, true] {
                let mut doc = Document::new();
                let nil = Border { style: BorderStyle::None, ..Border::single(0.5) };
                doc.styles.styles.push(Style {
                    id: "BorderProbe".into(),
                    kind: StyleKind::Table,
                    table: Some(TableStyleParts {
                        borders: Some(Borders::all(Border::single(0.5))),
                        first_col_borders: Some(Borders { top: Some(nil), bottom: Some(nil), between: Some(inside), ..Default::default() }),
                        ..Default::default()
                    }),
                    ..Default::default()
                });
                let mut table = Table::new(3, 2, 300.0);
                table.props.style = Some("BorderProbe".into());
                table.props.look.first_column = true;
                table.props.fixed = true;
                table.props.cell_margins = Some([0.0; 4]);
                for row in &mut table.rows {
                    for cell in &mut row.cells {
                        cell.blocks = vec![para_block(Paragraph::with_text("Hjg", Default::default()))];
                    }
                    if direct_cancel {
                        row.cells[0].props.borders = Some(Borders { top: Some(nil), bottom: Some(nil), ..Default::default() });
                    }
                }
                doc.body = vec![Arc::new(Block::Table(table))];
                let layout = crate::layout(&doc, &mut crate::LayoutCache::new(), &crate::LayoutOptions::default());
                let rects: Vec<_> = layout.pages[0]
                    .items
                    .iter()
                    .filter_map(|item| match item {
                        Placed::Cell { cell: 0, rect, .. } => Some(*rect),
                        _ => None,
                    })
                    .collect();
                let first = rects[0];
                let bottom = rects.last().unwrap().bottom();
                let mut rules: Vec<_> = layout.pages[0]
                    .items
                    .iter()
                    .filter_map(|item| match item {
                        Placed::Rule { x0, x1, y0, y1, border }
                            if (x0 - first.x).abs() < 0.01 && (x1 - first.right()).abs() < 0.01 && (y0 - y1).abs() < 0.01 =>
                        {
                            Some((*y0, border.width))
                        }
                        _ => None,
                    })
                    .collect();
                rules.sort_by(|a, b| a.0.total_cmp(&b.0));
                rules.dedup_by(|a, b| (a.0 - b.0).abs() < 0.01);
                if direct_cancel {
                    assert!(rules.is_empty(), "direct nil must cancel every first-column horizontal border: {rules:?}");
                } else {
                    assert_eq!(rules.len(), 2, "{name}: both internal grid borders must survive: {rules:?}");
                    let expected = if inside.is_visible() { inside.width } else { 0.5 };
                    for (y, width) in rules {
                        assert!((width - expected).abs() < 0.01, "{name}: expected width{expected}, got{width}");
                        assert!(y > first.y + 0.5 && y < bottom - 0.5, "conditional top/bottom nil must still cancel outer edges: y{y}");
                    }
                }
            }
        }
    }

    /// Word paints cell shading after the incoming horizontal band. Otherwise
    /// the next row's fill hides the previous row's border inside that band.
    #[test]
    fn cell_shading_does_not_cover_the_incoming_horizontal_border_band() {
        let mut doc = Document::new();
        let mut table = Table::new(2, 1, 300.0);
        table.props.fixed = true;
        table.props.cell_margins = Some([0.0; 4]);
        table.props.borders = Some(Borders::all(Border::single(0.0)));
        for row in &mut table.rows {
            row.cells[0].blocks = vec![para_block(Paragraph::with_text("Hjg", Default::default()))];
        }
        table.rows[0].cells[0].props.borders = Some(Borders { bottom: Some(Border::single(2.0)), ..Default::default() });
        table.rows[1].cells[0].props.shading = Some(Rgb(210, 220, 230));
        doc.body = vec![Arc::new(Block::Table(table))];
        let layout = crate::layout(&doc, &mut crate::LayoutCache::new(), &crate::LayoutOptions::default());
        let items = &layout.pages[0].items;
        let border_bottom = items
            .iter()
            .find_map(|item| match item {
                Placed::Rule { y0, y1, border, .. } if (y0 - y1).abs() < 0.01 && (border.width - 2.0).abs() < 0.01 => Some(*y0 + 1.0),
                _ => None,
            })
            .unwrap();
        let fill_top = items
            .iter()
            .find_map(|item| match item {
                Placed::Fill { rect, color } if *color == Rgb(210, 220, 230) => Some(rect.y),
                _ => None,
            })
            .unwrap();
        assert!(
            (fill_top - border_bottom).abs() < 0.01,
            "the fill must begin after the full border band: fill{fill_top}, border ends{border_bottom}"
        );
    }

    /// Word compat14 controls: a 59.92% table in a 523.3pt text area measures 320.05pt
    /// with default 5.4pt side margins, 313.55pt with zero margins and 314.75pt with 1pt.
    /// Compat15 uses the text area alone. Empty fixed cells isolate the width calculation.
    #[test]
    fn legacy_percentage_table_width_includes_default_cell_side_margins() {
        for mode in [14, 15] {
            for (margin, first_left, last_right) in
                [(0.0, None, None), (1.0, None, None), (5.4, None, None), (0.0, Some(10.0), None), (0.0, None, Some(10.0))]
            {
                let mut doc = Document::new();
                doc.settings.compat_mode = mode;
                doc.last_section.page_w = 595.3;
                doc.last_section.margin_left = 36.0;
                doc.last_section.margin_right = 36.0;
                let mut table = Table::new(1, 5, 320.05);
                table.props.fixed = true;
                table.props.width_pct = Some(59.92);
                table.props.cell_margins = Some([0.0, margin, 0.0, margin]);
                if let Some(left) = first_left {
                    table.rows[0].cells[0].props.margins = Some([0.0, left, 0.0, margin]);
                }
                if let Some(right) = last_right {
                    table.rows[0].cells[4].props.margins = Some([0.0, margin, 0.0, right]);
                }
                doc.body = vec![Arc::new(Block::Table(table))];
                let layout = crate::layout(&doc, &mut crate::LayoutCache::new(), &crate::LayoutOptions::default());
                let width: f32 = layout.pages[0]
                    .items
                    .iter()
                    .filter_map(|item| match item {
                        Placed::Cell { row: 0, rect, .. } => Some(rect.w),
                        _ => None,
                    })
                    .sum();
                let extra = first_left.unwrap_or(margin) + last_right.unwrap_or(margin);
                let expected = (523.3 + if mode < 15 { extra } else { 0.0 }) * 0.5992;
                assert!((width - expected).abs() < 0.05, "compat{mode}, margin{margin}: {width} must use legacy percentage frame {expected}");
            }
        }
    }

    fn column_widths(percent: Option<f32>, border: f32) -> Vec<f32> {
        let mut doc = Document::new();
        doc.styles.default_chr.font = Some("Arial".into());
        let mut table = Table::new(2, 3, 468.0);
        table.props.cell_margins = Some([0.0; 4]);
        table.props.borders = Some(Borders::all(Border::single(border)));
        for (r, row) in table.rows.iter_mut().enumerate() {
            for (c, cell) in row.cells.iter_mut().enumerate() {
                cell.props.width = None;
                cell.props.width_pct = percent;
                let text = format!("{}{}", ['A', 'B', 'C'][c], r + 1);
                cell.blocks = vec![para_block(Paragraph::with_text(&text, Default::default()))];
            }
        }
        doc.body = vec![Arc::new(Block::Table(table))];
        let layout = crate::layout(&doc, &mut crate::LayoutCache::new(), &crate::LayoutOptions::default());
        layout.pages[0]
            .items
            .iter()
            .filter_map(|item| match item {
                Placed::Cell { row: 0, rect, .. } => Some(rect.w),
                _ => None,
            })
            .collect()
    }

    /// Word's computed grid grows by each cell's two half-border bands: raising a border
    /// from 0.5 to 2 pt adds 1.5 pt to every automatically-sized column (Arial-only probes).
    #[test]
    fn automatic_table_width_includes_horizontal_border_bands() {
        let plain = column_widths(None, 0.0);
        let thin = column_widths(None, 0.5);
        let thick = column_widths(None, 2.0);
        assert_eq!(plain.len(), 3);
        for ((plain, thin), thick) in plain.iter().zip(&thin).zip(&thick) {
            assert!((thin - plain - 0.5).abs() < 0.05, "thin: {plain} -> {thin}");
            assert!((thick - thin - 1.5).abs() < 0.05, "thick: {thin} -> {thick}");
        }
    }

    /// Percentage-only cells without a preferred table width size to their content while
    /// keeping the requested proportions. They do not force the old full-page grid width.
    #[test]
    fn automatic_table_percentage_cells_size_to_content_and_keep_proportions() {
        let plain = column_widths(None, 0.5);
        let percent = column_widths(Some(100.0 / 3.0), 0.5);
        assert_eq!(percent.len(), 3);
        assert!(percent.iter().sum::<f32>() < 100.0, "stale page-width grid: {percent:?}");
        let widest = plain.iter().copied().fold(0.0f32, f32::max);
        for width in percent {
            assert!((width - widest).abs() < 0.05, "equal percentage cells use widest content: {width} vs {widest}");
        }
    }

    /// Measuring a nested automatic table must include the width required by percentage
    /// proportions. Otherwise the parent shrinks to the raw sum of text widths while the
    /// child uses a much larger width to keep its 90/10 columns and overflows its parent.
    #[test]
    fn nested_percentage_table_measurement_contains_its_rendered_columns() {
        let mut doc = Document::new();
        doc.styles.default_chr.font = Some("Arial".into());
        let mut inner = Table::new(1, 2, 468.0);
        inner.props.cell_margins = Some([0.0; 4]);
        inner.props.borders = Some(Borders::all(Border::single(0.0)));
        for (cell, (percent, text)) in inner.rows[0].cells.iter_mut().zip([(90.0, "S"), (10.0, "LONGWORDLONGWORD")]) {
            cell.props.width = None;
            cell.props.width_pct = Some(percent);
            cell.blocks = vec![para_block(Paragraph::with_text(text, Default::default()))];
        }
        let mut outer = Table::new(1, 1, 468.0);
        outer.props.cell_margins = Some([0.0; 4]);
        outer.props.borders = Some(Borders::all(Border::single(0.0)));
        outer.rows[0].cells[0].props.width = None;
        outer.rows[0].cells[0].blocks = vec![Arc::new(Block::Table(inner)), para_block(Paragraph::new())];
        doc.body = vec![Arc::new(Block::Table(outer))];
        let layout = crate::layout(&doc, &mut crate::LayoutCache::new(), &crate::LayoutOptions::default());
        let parent = layout.pages[0]
            .items
            .iter()
            .find_map(|item| match item {
                Placed::Cell { rect, table, .. } if table.0.len() == 1 => Some(*rect),
                _ => None,
            })
            .unwrap();
        let children: Vec<_> = layout.pages[0]
            .items
            .iter()
            .filter_map(|item| match item {
                Placed::Cell { rect, table, .. } if table.0.len() > 1 => Some(*rect),
                _ => None,
            })
            .collect();
        assert_eq!(children.len(), 2);
        for child in children {
            assert!(
                child.x >= parent.x - 0.1 && child.right() <= parent.right() + 0.1,
                "nested percentage cell {child:?} overflows its measured parent {parent:?}"
            );
        }
    }

    /// Word Arial controls keep LONGWORDLONGWORD on one line by relaxing 90/10 shares
    /// when their natural total does not fit. A uniform cap after forcing proportions
    /// makes that second cell narrower than its minimum and wraps the same word.
    #[test]
    fn percentage_minimums_are_preserved_when_natural_proportions_do_not_fit() {
        let mut doc = Document::new();
        doc.styles.default_chr.font = Some("Arial".into());
        let mut table = Table::new(1, 2, 468.0);
        table.props.cell_margins = Some([0.0; 4]);
        table.props.borders = Some(Borders::all(Border::single(0.5)));
        for (cell, (percent, text)) in table.rows[0].cells.iter_mut().zip([(90.0, "S"), (10.0, "LONGWORDLONGWORD")]) {
            cell.props.width = None;
            cell.props.width_pct = Some(percent);
            cell.blocks = vec![para_block(Paragraph::with_text(text, Default::default()))];
        }
        doc.body = vec![Arc::new(Block::Table(table))];
        let layout = crate::layout(&doc, &mut crate::LayoutCache::new(), &crate::LayoutOptions::default());
        let widths: Vec<_> = layout.pages[0]
            .items
            .iter()
            .filter_map(|item| match item {
                Placed::Cell { rect, .. } => Some(rect.w),
                _ => None,
            })
            .collect();
        assert_eq!(widths.len(), 2);
        let long_lines = layout.pages[0]
            .items
            .iter()
            .find_map(|item| match item {
                Placed::Lines { path, l0, l1, .. } if path.0 == [0, 0, 1, 0] => Some(l1 - l0),
                _ => None,
            })
            .unwrap();
        assert_eq!(long_lines, 1, "the Word control keeps the long second-cell word on one line");
        assert!(widths[1] > widths.iter().sum::<f32>() * 0.1 + 1.0, "the 10% preference must relax to preserve that column's minimum: {widths:?}");
    }
}
