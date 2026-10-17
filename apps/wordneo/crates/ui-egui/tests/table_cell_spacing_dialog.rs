//! Staged outside SOURCE, not compiled or executed.
//! UI distance follows measured Word COM: 1.5pt visible <-> 0.75 file points.
//! The Word widget itself was not observed; do not infer pixel/rounding parity.
//! Uses existing public APIs only: no prospective TableForm field or enum.

use std::sync::Arc;

use egui_kittest::Harness;
use egui_kittest::kittest::{By, Queryable};
use serde_json::{Value, json};
use wordcraft_doc::styles::{Style, StyleKind, TableStyleParts};
use wordcraft_doc::{Block, Document, Paragraph, Path, Pos, StoryRef, Table, para_block};
use wordcraft_engine::{Selection, Session};
use wordcraft_geom::Unit;
use wordcraft_ui_egui::dialogs::Dialog;
use wordcraft_ui_egui::{Services, WordApp};

type UiHarness = Harness<'static, WordApp>;
const VALUE_LABEL: &str = "Cell spacing:";
const INHERIT_LABEL: &str = "Use table style";
const ZERO_LABEL: &str = "No spacing";
const CUSTOM_LABEL: &str = "Custom spacing";

fn app(direct: Option<f32>, unit: Unit) -> WordApp {
    let mut doc = Document::new();
    doc.styles.styles.push(Style {
        id: "UiSpacingBase".into(),
        kind: StyleKind::Table,
        table: Some(TableStyleParts { cell_spacing: Some(1.5), ..Default::default() }),
        ..Default::default()
    });
    doc.styles.styles.push(Style {
        id: "UiSpacingDerived".into(),
        kind: StyleKind::Table,
        based_on: Some("UiSpacingBase".into()),
        table: Some(TableStyleParts::default()),
        ..Default::default()
    });
    // Keep unrelated existing length widgets inside their current unit range.
    let mut table = Table::new(1, 2, 40.0);
    table.props.style = Some("UiSpacingDerived".into());
    table.props.cell_spacing = direct;
    for (cell, text) in table.rows[0].cells.iter_mut().zip(["A", "B"]) {
        cell.blocks = vec![para_block(Paragraph::with_text(text, Default::default()))];
    }
    doc.body = vec![Arc::new(Block::Table(table)), para_block(Paragraph::with_text("After the table.", Default::default()))];
    app_from_doc(doc, unit)
}

fn app_from_doc(doc: Document, unit: Unit) -> WordApp {
    let mut session = Session::new(doc);
    session.prefs.units = Some(unit);
    session.sel = Selection::caret(Pos { story: StoryRef::Body, path: Path(vec![0, 0, 0, 0]), off: 0 });
    let mut app = WordApp::new(session, Services::default());
    app.ui.language = "en".into();
    app
}

fn harness(app: WordApp) -> UiHarness {
    let mut h = Harness::builder().with_size(egui::vec2(1440.0, 1000.0)).build_ui_state(
        |ui, app: &mut WordApp| {
            let ctx = ui.ctx().clone();
            app.logic(&ctx);
            app.ui(ui);
        },
        app,
    );
    h.run_steps(4);
    h
}

fn table(app: &WordApp) -> &Table {
    app.session.doc.table(StoryRef::Body, &Path::top(0)).unwrap()
}

fn inherited(app: &WordApp) -> Option<f32> {
    app.session.doc.styles.table_style("UiSpacingDerived").unwrap().parts.cell_spacing
}

fn open(h: &mut UiHarness) {
    h.state_mut().run("table.properties", json!({})).unwrap();
    h.run_steps(4);
    assert!(matches!(h.state().dialog, Some(Dialog::TableProperties { .. })));
}

fn changes(h: &UiHarness) -> Value {
    let Some(Dialog::TableProperties { form, basis }) = &h.state().dialog else { panic!("table properties must be open") };
    form.changes(basis)
}

fn assert_controls(h: &UiHarness) {
    for label in [INHERIT_LABEL, ZERO_LABEL, CUSTOM_LABEL] {
        assert!(h.query_by_label(label).is_some(), "missing visible, operable spacing choice: {label}");
    }
    assert!(
        h.query(By::new().role(egui::accesskit::Role::SpinButton).label(VALUE_LABEL)).is_some(),
        "the spacing value must be labelled and keyboard-accessible"
    );
}

fn choose(h: &mut UiHarness, label: &str) {
    h.get_by_label(label).click();
    h.run_steps(3);
}

fn type_spacing(h: &mut UiHarness, value: &str) {
    h.get(By::new().role(egui::accesskit::Role::SpinButton).label(VALUE_LABEL)).focus();
    h.run_steps(2);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.run_steps(1);
    h.get(By::new().role(egui::accesskit::Role::SpinButton).label(VALUE_LABEL)).type_text(value);
    h.run_steps(2);
    // Tab commits the numeric edit without using the dialog's Enter-to-OK.
    h.key_press(egui::Key::Tab);
    h.run_steps(3);
}

fn close(h: &mut UiHarness, label: &str) {
    h.get_by_label(label).click();
    h.run_steps(3);
    assert!(h.state().dialog.is_none());
}

fn reopened(app: &WordApp, unit: Unit) -> WordApp {
    let bytes = wordcraft_engine::io::save_bytes("own-ui-spacing.docx", &app.session.doc).unwrap();
    let doc = wordcraft_engine::io::open_bytes("own-ui-spacing.docx", &bytes).unwrap();
    app_from_doc(doc, unit)
}

#[test]
fn table_spacing_dialog_reads_inherited_zero_and_custom_and_cancels_without_a_command() {
    for (direct, distance) in [(None, 3.0), (Some(0.0), 0.0), (Some(0.75), 1.5)] {
        let mut h = harness(app(direct, Unit::Points));
        let before = serde_json::to_value(&h.state().session.doc).unwrap();
        let depth = h.state().session.undo_depth();
        let last = h.state().session.last_command.clone();
        open(&mut h);
        assert_controls(&h); // Current UI fails here at runtime, without a new API.
        assert_eq!(changes(&h), json!({}), "reading must not materialize inherited spacing");
        let shown = h.get(By::new().role(egui::accesskit::Role::SpinButton).label(VALUE_LABEL)).value().unwrap();
        let shown: String = shown.chars().filter(|c| !c.is_whitespace()).collect();
        assert_eq!(wordcraft_geom::parse_measure(&shown, Unit::Points), Some(distance));
        choose(&mut h, if direct == Some(0.0) { INHERIT_LABEL } else { ZERO_LABEL });
        close(&mut h, "Cancel");
        assert_eq!(serde_json::to_value(&h.state().session.doc).unwrap(), before);
        assert_eq!(h.state().session.undo_depth(), depth);
        assert_eq!(h.state().session.last_command, last, "Cancel must not send a formatting command");
    }
}

#[test]
fn table_spacing_dialog_untouched_ok_and_cancel_preserve_none_zero_custom_and_history() {
    for direct in [None, Some(0.0), Some(0.75)] {
        let mut h = harness(app(direct, Unit::Points));
        let before = serde_json::to_value(&h.state().session.doc).unwrap();
        let depth = h.state().session.undo_depth();
        let last = h.state().session.last_command.clone();
        for button in ["OK", "Cancel"] {
            open(&mut h);
            assert_eq!(changes(&h), json!({}));
            close(&mut h, button);
            assert_eq!(serde_json::to_value(&h.state().session.doc).unwrap(), before);
            assert_eq!(h.state().session.undo_depth(), depth);
            assert_eq!(h.state().session.last_command, last, "untouched dialog must not send a command");
        }
    }
}

#[test]
fn table_spacing_dialog_custom_zero_and_style_use_the_shared_command_undo_and_docx() {
    let mut h = harness(app(None, Unit::Points));
    let depth = h.state().session.undo_depth();
    open(&mut h);
    assert_controls(&h);
    choose(&mut h, CUSTOM_LABEL);
    type_spacing(&mut h, "1.5 pt");
    assert_eq!(changes(&h), json!({"cellSpacing": 0.75}), "convert the visible COM-distance value only in L6");
    assert_eq!(table(h.state()).props.cell_spacing, None, "editing the form must not edit the document before OK");
    close(&mut h, "OK");
    assert_eq!(table(h.state()).props.cell_spacing, Some(0.75));
    assert_eq!(h.state().session.last_command.as_ref().map(|c| c.0.as_str()), Some("table.properties"));
    assert_eq!(h.state().session.undo_depth(), depth + 1);
    h.state_mut().run("edit.undo", json!({})).unwrap();
    assert_eq!(table(h.state()).props.cell_spacing, None);
    h.state_mut().run("edit.redo", json!({})).unwrap();
    assert_eq!(table(h.state()).props.cell_spacing, Some(0.75));
    h = harness(reopened(h.state(), Unit::Points));
    assert_eq!(table(h.state()).props.cell_spacing, Some(0.75));
    open(&mut h);
    assert_controls(&h);
    assert_eq!(changes(&h), json!({}));
    choose(&mut h, ZERO_LABEL);
    assert_eq!(changes(&h), json!({"cellSpacing": 0.0}));
    close(&mut h, "OK");
    assert_eq!(table(h.state()).props.cell_spacing, Some(0.0));
    assert_eq!(inherited(h.state()), Some(1.5));
    h = harness(reopened(h.state(), Unit::Points));
    assert_eq!(table(h.state()).props.cell_spacing, Some(0.0));
    open(&mut h);
    assert_controls(&h);
    choose(&mut h, INHERIT_LABEL);
    assert_eq!(changes(&h), json!({"cellSpacing": null}));
    close(&mut h, "OK");
    assert_eq!(table(h.state()).props.cell_spacing, None);
    h = harness(reopened(h.state(), Unit::Points));
    assert_eq!(table(h.state()).props.cell_spacing, None);
    assert_eq!(inherited(h.state()), Some(1.5));
}

#[test]
fn table_spacing_dialog_parses_explicit_point_input_in_each_interface_unit() {
    for unit in [Unit::Points, Unit::Centimeters, Unit::Inches] {
        let mut h = harness(app(None, unit));
        let depth = h.state().session.undo_depth();
        open(&mut h);
        assert_controls(&h);
        choose(&mut h, CUSTOM_LABEL);
        type_spacing(&mut h, "1.5 pt");
        let value = changes(&h)["cellSpacing"].as_f64().unwrap();
        assert!((value - 0.75).abs() < 1e-6, "UI units must not become file-point units: {unit:?}, {value}");
        close(&mut h, "Cancel");
        assert_eq!(table(h.state()).props.cell_spacing, None);
        assert_eq!(h.state().session.undo_depth(), depth);
    }
}
