//! Widget registry backed by the generated manifest.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, LazyLock, RwLock},
};

use anyhow::{bail, Context, Result};
use semver::Version;
use serde_json::Value as JsonValue;
use time::{format_description::well_known::Iso8601, OffsetDateTime};
use tracing::{debug, error, info, warn};

use crate::widgets_manifest::{
    read_manifest, WidgetManifest, WidgetManifestEntry, SUPPORTED_SCHEMA_MAJOR,
};

/// Represents a widget with all metadata required for MCP integration.
#[derive(Debug, Clone)]
pub struct Widget {
    pub id: String,
    pub title: String,
    pub template_uri: String,
    pub invoking: String,
    pub invoked: String,
    pub html: String,
    pub response_text: String,
    pub assets: WidgetAssets,
}

impl Widget {
    /// Generates OpenAI-specific metadata for widget integration.
    pub fn meta(&self) -> JsonValue {
        serde_json::json!({
            "openai/outputTemplate": self.template_uri,
            "openai/toolInvocation/invoking": self.invoking,
            "openai/toolInvocation/invoked": self.invoked,
            "openai/widgetAccessible": true,
            "openai/resultCanProduceWidget": true,
        })
    }
}

/// Optional asset metadata associated with a widget.
#[derive(Debug, Clone, Default)]
pub struct WidgetAssets {
    pub html: Option<String>,
    pub css: Option<String>,
    pub js: Option<String>,
}

/// Registry metadata useful for diagnostics and health checks.
#[derive(Debug, Clone)]
pub struct RegistryMetadata {
    pub schema_version: Option<String>,
    pub manifest_path: PathBuf,
    pub manifest_exists: bool,
    pub manifest_generated_at: Option<OffsetDateTime>,
    pub last_successful_load: Option<OffsetDateTime>,
    pub registry_initialized: bool,
}

impl RegistryMetadata {
    fn empty(manifest_path: PathBuf) -> Self {
        Self {
            schema_version: None,
            manifest_path,
            manifest_exists: false,
            manifest_generated_at: None,
            last_successful_load: None,
            registry_initialized: false,
        }
    }
}

/// In-memory widget registry with fast lookups by ID or template URI.
#[derive(Debug)]
pub struct WidgetsRegistry {
    widgets: Vec<Arc<Widget>>,
    widgets_by_id: HashMap<String, Arc<Widget>>,
    widgets_by_uri: HashMap<String, Arc<Widget>>,
    metadata: RegistryMetadata,
}

impl WidgetsRegistry {
    fn empty(manifest_path: PathBuf) -> Self {
        Self {
            widgets: Vec::new(),
            widgets_by_id: HashMap::new(),
            widgets_by_uri: HashMap::new(),
            metadata: RegistryMetadata::empty(manifest_path),
        }
    }

    fn from_manifest(
        manifest: WidgetManifest,
        manifest_path: PathBuf,
        load_timestamp: OffsetDateTime,
    ) -> Result<Self> {
        validate_schema_version(&manifest.schema_version)?;

        let mut widgets: Vec<Arc<Widget>> = Vec::with_capacity(manifest.widgets.len());
        let mut by_id = HashMap::with_capacity(manifest.widgets.len());
        let mut by_uri = HashMap::with_capacity(manifest.widgets.len());
        let manifest_dir = manifest_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));

        for entry in manifest.widgets {
            let widget = Arc::new(widget_from_entry(&entry, &manifest_dir)?);

            if by_id.contains_key(&widget.id) {
                bail!("Duplicate widget id detected in manifest: {}", widget.id);
            }
            if by_uri.contains_key(&widget.template_uri) {
                bail!(
                    "Duplicate widget templateUri detected in manifest: {}",
                    widget.template_uri
                );
            }

            by_id.insert(widget.id.clone(), Arc::clone(&widget));
            by_uri.insert(widget.template_uri.clone(), Arc::clone(&widget));
            widgets.push(widget);
        }

        widgets.sort_by(|a, b| a.id.cmp(&b.id));

        let generated_at = manifest
            .generated_at
            .as_deref()
            .and_then(parse_timestamp)
            .or_else(|| file_timestamp(&manifest_path));

        let metadata = RegistryMetadata {
            schema_version: Some(manifest.schema_version),
            manifest_path,
            manifest_exists: true,
            manifest_generated_at: generated_at,
            last_successful_load: Some(load_timestamp),
            registry_initialized: true,
        };

        Ok(Self {
            widgets,
            widgets_by_id: by_id,
            widgets_by_uri: by_uri,
            metadata,
        })
    }

    /// Returns the registry metadata.
    pub fn metadata(&self) -> &RegistryMetadata {
        &self.metadata
    }

    /// Returns all widgets as shared references.
    pub fn widgets(&self) -> Vec<Arc<Widget>> {
        self.widgets.clone()
    }

    fn widget_by_id(&self, id: &str) -> Option<Arc<Widget>> {
        self.widgets_by_id.get(id).cloned()
    }

    fn widget_by_uri(&self, uri: &str) -> Option<Arc<Widget>> {
        self.widgets_by_uri.get(uri).cloned()
    }
}

fn log_registry_success(registry: &WidgetsRegistry) {
    let widget_count = registry.widgets.len();
    let schema = registry
        .metadata
        .schema_version
        .as_deref()
        .unwrap_or("unknown");

    info!(
        widget_count,
        schema,
        manifest = %registry.metadata.manifest_path.display(),
        "Loaded widgets manifest"
    );

    if let Some(timestamp) = registry.metadata.manifest_generated_at {
        if let Ok(formatted) = timestamp.format(&Iso8601::DEFAULT) {
            debug!(
                manifest_timestamp = formatted,
                "Widget manifest generated at {}", formatted
            );
        }
    }
}

fn widget_from_entry(entry: &WidgetManifestEntry, manifest_dir: &Path) -> Result<Widget> {
    if entry.id.trim().is_empty() {
        bail!("Widget entry missing id");
    }
    if entry.template_uri.trim().is_empty() {
        bail!("Widget entry missing templateUri for {}", entry.id);
    }
    if entry.html.trim().is_empty() {
        bail!("Widget entry missing html for {}", entry.id);
    }

    let assets = WidgetAssets {
        html: validate_asset_path(
            entry.assets.as_ref().and_then(|a| a.html.as_deref()),
            manifest_dir,
        )
        .context("validating html asset")?,
        css: validate_asset_path(
            entry.assets.as_ref().and_then(|a| a.css.as_deref()),
            manifest_dir,
        )
        .context("validating css asset")?,
        js: validate_asset_path(
            entry.assets.as_ref().and_then(|a| a.js.as_deref()),
            manifest_dir,
        )
        .context("validating js asset")?,
    };

    Ok(Widget {
        id: entry.id.trim().to_string(),
        title: entry.title.trim().to_string(),
        template_uri: entry.template_uri.trim().to_string(),
        invoking: entry.invoking.trim().to_string(),
        invoked: entry.invoked.trim().to_string(),
        html: entry.html.clone(),
        response_text: entry.response_text.trim().to_string(),
        assets,
    })
}

fn validate_schema_version(schema: &str) -> Result<()> {
    let version = Version::parse(schema)
        .with_context(|| format!("Invalid schemaVersion in widget manifest: {schema}"))?;
    if version.major != SUPPORTED_SCHEMA_MAJOR {
        bail!(
            "Unsupported schemaVersion: {} (expected major {})",
            schema,
            SUPPORTED_SCHEMA_MAJOR
        );
    }
    Ok(())
}

fn parse_timestamp(value: &str) -> Option<OffsetDateTime> {
    OffsetDateTime::parse(value, &Iso8601::DEFAULT).ok()
}

fn file_timestamp(path: &Path) -> Option<OffsetDateTime> {
    std::fs::metadata(path)
        .and_then(|meta| meta.modified())
        .ok()
        .map(OffsetDateTime::from)
}

fn validate_asset_path(asset: Option<&str>, manifest_dir: &Path) -> Result<Option<String>> {
    let Some(raw) = asset else {
        return Ok(None);
    };

    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    if is_remote_path(trimmed) {
        return Ok(Some(trimmed.to_string()));
    }

    let candidate = if Path::new(trimmed).is_absolute() {
        PathBuf::from(trimmed)
    } else {
        manifest_dir.join(trimmed)
    };

    if !candidate.exists() {
        bail!("Asset path does not exist: {}", candidate.display());
    }
    if !candidate.is_file() {
        bail!("Asset path is not a file: {}", candidate.display());
    }

    Ok(Some(trimmed.to_string()))
}

fn is_remote_path(value: &str) -> bool {
    value.starts_with("http://") || value.starts_with("https://") || value.starts_with("//")
}

fn now_utc() -> OffsetDateTime {
    OffsetDateTime::now_utc()
}

static MANIFEST_PATH: LazyLock<PathBuf> = LazyLock::new(resolve_manifest_path);

fn resolve_manifest_path() -> PathBuf {
    std::env::var("WIDGETS_MANIFEST_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("assets/widgets.json"))
}

static REGISTRY: LazyLock<RwLock<Arc<WidgetsRegistry>>> = LazyLock::new(|| {
    let manifest_path = manifest_path();
    RwLock::new(Arc::new(WidgetsRegistry::empty(manifest_path)))
});

/// Returns the configured manifest path.
pub fn manifest_path() -> PathBuf {
    MANIFEST_PATH.clone()
}

/// Returns a clone of the current registry (cheap due to Arc).
pub fn registry() -> Arc<WidgetsRegistry> {
    REGISTRY.read().expect("registry lock poisoned").clone()
}

fn swap_registry(new_registry: Arc<WidgetsRegistry>) {
    let mut lock = REGISTRY.write().expect("registry lock poisoned");
    *lock = new_registry;
}

/// Attempts to bootstrap the registry from disk during startup.
pub fn bootstrap_registry() {
    let path = manifest_path();
    match load_registry_from_path(&path) {
        Ok(registry) => {
            log_registry_success(&registry);
            swap_registry(Arc::new(registry));
        }
        Err(LoadError::NotFound { path }) => {
            warn!(
                manifest = %path.display(),
                "No widgets available - manifest not found at {}",
                path.display()
            );
            swap_registry(Arc::new(WidgetsRegistry::empty(path)));
        }
        Err(LoadError::Validation { path, error }) => {
            error!(
                manifest = %path.display(),
                error = %error,
                "Failed to load widget manifest; keeping existing registry"
            );
        }
    }
}

/// Attempts to load a registry from the given path.
pub fn load_registry_from_path(path: &Path) -> Result<WidgetsRegistry, LoadError> {
    if !path.exists() {
        return Err(LoadError::NotFound {
            path: path.to_path_buf(),
        });
    }

    let manifest = read_manifest(path).map_err(|error| LoadError::Validation {
        path: path.to_path_buf(),
        error,
    })?;

    let registry = WidgetsRegistry::from_manifest(manifest, path.to_path_buf(), now_utc())
        .map_err(|error| LoadError::Validation {
            path: path.to_path_buf(),
            error,
        })?;

    Ok(registry)
}

/// Outcome of a successful registry reload.
#[derive(Debug, Clone)]
pub struct RegistryReloadOutcome {
    pub widget_count: usize,
    pub schema_version: Option<String>,
    pub manifest_timestamp: Option<OffsetDateTime>,
}

/// Reloads the registry from disk and swaps it into place.
pub fn reload_registry() -> Result<RegistryReloadOutcome, LoadError> {
    let path = manifest_path();
    let registry = load_registry_from_path(&path)?;

    let outcome = RegistryReloadOutcome {
        widget_count: registry.widgets.len(),
        schema_version: registry.metadata.schema_version.clone(),
        manifest_timestamp: registry.metadata.manifest_generated_at,
    };

    log_registry_success(&registry);
    swap_registry(Arc::new(registry));

    Ok(outcome)
}

/// Returns all available widgets.
pub fn get_all_widgets() -> Vec<Arc<Widget>> {
    registry().widgets()
}

/// Looks up a widget by its ID (tool name).
pub fn get_widget_by_id(id: &str) -> Option<Arc<Widget>> {
    registry().widget_by_id(id)
}

/// Looks up a widget by its template URI.
pub fn get_widget_by_uri(uri: &str) -> Option<Arc<Widget>> {
    registry().widget_by_uri(uri)
}

/// Returns registry metadata for diagnostics.
pub fn registry_metadata() -> RegistryMetadata {
    registry().metadata.clone()
}

/// Errors that can occur while loading the manifest.
#[derive(Debug)]
pub enum LoadError {
    NotFound { path: PathBuf },
    Validation { path: PathBuf, error: anyhow::Error },
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadError::NotFound { path } => {
                write!(f, "manifest not found at {}", path.display())
            }
            LoadError::Validation { path, error } => {
                write!(
                    f,
                    "failed to load manifest at {}: {}",
                    path.display(),
                    error
                )
            }
        }
    }
}

impl std::error::Error for LoadError {}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn sample_manifest_json() -> serde_json::Value {
        serde_json::json!({
            "schemaVersion": "1.0.0",
            "generatedAt": "2024-10-15T10:30:00Z",
            "widgets": [
                {
                    "id": "pizza-map",
                    "title": "Pizza Map",
                    "templateUri": "ui://widget/pizza-map.html",
                    "invoking": "Invoking",
                    "invoked": "Invoked",
                    "html": "<div></div>",
                    "responseText": "Rendered!",
                    "assets": {
                        "html": "assets/pizzaz-aaaa.html"
                    }
                }
            ]
        })
    }

    #[test]
    fn validate_schema_version_accepts_major_one() {
        assert!(validate_schema_version("1.0.0").is_ok());
        assert!(validate_schema_version("1.2.3").is_ok());
    }

    #[test]
    fn validate_schema_version_rejects_other_major() {
        assert!(validate_schema_version("2.0.0").is_err());
        assert!(validate_schema_version("0.9.0").is_err());
    }

    #[test]
    fn load_registry_from_valid_manifest() {
        let manifest_path = NamedTempFile::new().expect("tmp manifest");
        let manifest_dir = manifest_path
            .path()
            .parent()
            .expect("manifest dir")
            .to_path_buf();

        let html_path = manifest_dir.join("assets").join("pizzaz-aaaa.html");
        std::fs::create_dir_all(html_path.parent().unwrap()).unwrap();
        std::fs::write(&html_path, "<div></div>").unwrap();

        let manifest = sample_manifest_json();
        serde_json::to_writer(&manifest_path, &manifest).unwrap();

        let registry = load_registry_from_path(manifest_path.path()).unwrap();
        assert_eq!(registry.widgets.len(), 1);
        assert_eq!(registry.widgets[0].id, "pizza-map");
        assert!(registry.metadata.registry_initialized);
    }

    #[test]
    fn load_registry_missing_manifest() {
        let missing = PathBuf::from("does-not-exist.json");
        let result = load_registry_from_path(&missing);
        assert!(matches!(result, Err(LoadError::NotFound { .. })));
    }

    #[test]
    fn asset_validation_allows_remote() {
        let result = validate_asset_path(Some("https://example.com/test.js"), Path::new("."));
        assert!(result.is_ok());
    }

    #[test]
    fn asset_validation_rejects_missing_file() {
        let result = validate_asset_path(Some("missing.css"), Path::new("."));
        assert!(result.is_err());
    }
}
