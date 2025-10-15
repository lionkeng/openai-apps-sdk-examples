//! Manifest types and parsing helpers for the widget registry.

use std::{fs, path::Path};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Canonical schema version supported by the server.
pub const SUPPORTED_SCHEMA_MAJOR: u64 = 1;

/// Top-level manifest structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WidgetManifest {
    pub schema_version: String,
    #[serde(default)]
    pub generated_at: Option<String>,
    #[serde(default)]
    pub widgets: Vec<WidgetManifestEntry>,
}

/// Per widget manifest entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WidgetManifestEntry {
    pub id: String,
    pub title: String,
    pub template_uri: String,
    pub invoking: String,
    pub invoked: String,
    pub html: String,
    pub response_text: String,
    #[serde(default)]
    pub assets: Option<WidgetManifestAssets>,
}

/// Optional asset paths associated with a widget manifest entry.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WidgetManifestAssets {
    pub html: Option<String>,
    pub css: Option<String>,
    pub js: Option<String>,
}

/// Reads and deserializes a manifest from disk.
pub fn read_manifest(path: &Path) -> Result<WidgetManifest> {
    let data = fs::read_to_string(path)
        .with_context(|| format!("Failed to read widget manifest at {}", path.display()))?;
    let manifest: WidgetManifest = serde_json::from_str(&data)
        .with_context(|| format!("Failed to parse widget manifest JSON at {}", path.display()))?;
    Ok(manifest)
}
