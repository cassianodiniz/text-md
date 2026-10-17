//! Public Settings/serde contract for the noExtraLineSpacing compatibility flag.

use serde_json::{Value, json};
use wordcraft_doc::{Document, Settings};

#[test]
fn no_extra_line_spacing_defaults_to_absent_without_changing_old_json() {
    assert_eq!(Settings::default().no_extra_line_spacing, None);
    let from_old_json: Settings = serde_json::from_value(json!({"compatMode": 14})).unwrap();
    assert_eq!(from_old_json.no_extra_line_spacing, None);
    assert_eq!(from_old_json.compat_mode, 14);
    let serialized = serde_json::to_value(&from_old_json).unwrap();
    assert!(serialized.get("noExtraLineSpacing").is_none());
    assert!(serialized.get("no_extra_line_spacing").is_none());
    let document_json = serde_json::to_value(Document::new()).unwrap();
    assert!(document_json["settings"].get("noExtraLineSpacing").is_none());
}

#[test]
fn no_extra_line_spacing_serde_keeps_explicit_true_and_false() {
    for enabled in [true, false] {
        let settings = Settings { no_extra_line_spacing: Some(enabled), ..Settings::default() };
        let serialized = serde_json::to_value(&settings).unwrap();
        assert_eq!(serialized.get("noExtraLineSpacing"), Some(&Value::Bool(enabled)));
        assert!(serialized.get("no_extra_line_spacing").is_none());
        let reopened: Settings = serde_json::from_value(serialized).unwrap();
        assert_eq!(reopened, settings);

        let imported: Settings = serde_json::from_value(json!({"noExtraLineSpacing": enabled})).unwrap();
        assert_eq!(imported.no_extra_line_spacing, Some(enabled));
    }
}

#[test]
fn no_extra_line_spacing_null_uses_the_absent_state() {
    let settings: Settings = serde_json::from_value(json!({"noExtraLineSpacing": null})).unwrap();
    assert_eq!(settings.no_extra_line_spacing, None);
    assert!(serde_json::to_value(settings).unwrap().get("noExtraLineSpacing").is_none());
}
