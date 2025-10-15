//! Test utilities and helpers

use std::{path::PathBuf, sync::Once};

use crate::widgets;

static INIT: Once = Once::new();

/// Ensures the widget registry is populated from the test fixture manifest.
pub fn initialize_widgets_for_tests() {
    INIT.call_once(|| {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/widgets.json");
        std::env::set_var("WIDGETS_MANIFEST_PATH", &path);
        std::env::set_var("WIDGETS_REFRESH_TOKEN", "test-refresh-token");
        widgets::bootstrap_registry();
    });
}
