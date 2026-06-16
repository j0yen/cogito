/// Spec loading and validation for cogito.toml
///
/// The spec format is a single TOML file (spec/cogito.toml) or a directory
/// containing ontology.toml + domain files.

use std::path::Path;

use anyhow::{bail, Context, Result};
use serde::Deserialize;

/// IRI base for the cogito ontology
pub const COGITO_NS: &str = "https://wintermute.local/cogito#";
pub const COGITO_ONT: &str = "https://wintermute.local/cogito";
pub const BFO_IMPORT: &str = "http://purl.obolibrary.org/obo/bfo/2020/bfo.owl";

/// The built-in spec path (relative to the binary's directory or embedded).
/// Falls back to the embedded string if not found on disk.
pub const BUILTIN_SPEC: &str = include_str!("../spec/cogito.toml");

#[derive(Debug, Deserialize)]
pub struct CogitoSpec {
    pub iri: Option<String>,
    pub version_iri: Option<String>,
    pub imports: Option<Vec<String>>,
    #[serde(default)]
    pub object_properties: Vec<ObjectPropertyDef>,
    #[serde(default)]
    pub classes: Vec<ClassDef>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ObjectPropertyDef {
    pub iri: String,
    pub label: String,
    #[serde(default)]
    pub transitive: bool,
    pub inverse_of: Option<String>,
    pub domain: Option<String>,
    pub range: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ClassDef {
    pub iri: String,
    pub label: String,
    pub parent: Option<String>,
    /// Alternative parent field for full-IRI parents not matching the "BFO:" prefix scheme
    pub parent_full: Option<String>,
    pub definition: Option<String>,
    pub equivalent_to: Option<String>,
    #[serde(default)]
    pub subclass_of: Vec<String>,
}

impl ClassDef {
    /// Return the effective parent IRI (prefer `parent_full` over `parent`)
    pub fn effective_parent(&self) -> Option<&str> {
        self.parent_full
            .as_deref()
            .or(self.parent.as_deref())
    }
}

/// Load and validate the spec from the given path (file or dir).
/// If `path` is None, use the embedded spec.
pub fn load(path: Option<&Path>) -> Result<CogitoSpec> {
    let content = match path {
        None => BUILTIN_SPEC.to_string(),
        Some(p) => {
            if p.is_dir() {
                // Look for cogito.toml inside
                let candidate = p.join("cogito.toml");
                std::fs::read_to_string(&candidate)
                    .with_context(|| format!("reading {}", candidate.display()))?
            } else {
                std::fs::read_to_string(p)
                    .with_context(|| format!("reading {}", p.display()))?
            }
        }
    };

    let spec: CogitoSpec =
        toml::from_str(&content).context("parsing cogito spec TOML")?;
    validate(&spec)?;
    Ok(spec)
}

/// Validate the spec — check for duplicate IRIs and required fields.
pub fn validate(spec: &CogitoSpec) -> Result<()> {
    let mut iris = std::collections::HashSet::new();

    for op in &spec.object_properties {
        if op.iri.is_empty() {
            bail!("object property has empty IRI");
        }
        if op.label.is_empty() {
            bail!("object property '{}' has empty label", op.iri);
        }
        if !iris.insert(op.iri.clone()) {
            bail!("duplicate IRI in spec: {}", op.iri);
        }
    }

    for cls in &spec.classes {
        if cls.iri.is_empty() {
            bail!("class has empty IRI");
        }
        if cls.label.is_empty() {
            bail!("class '{}' has empty label", cls.iri);
        }
        if !iris.insert(cls.iri.clone()) {
            bail!("duplicate IRI in spec: {}", cls.iri);
        }
    }

    Ok(())
}
