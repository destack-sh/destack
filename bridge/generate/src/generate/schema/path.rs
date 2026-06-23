use std::collections::BTreeMap;

use anyhow::{Result, bail};

use crate::generate::core::upper_camel;

/// Generated schema output root.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SchemaRoot {
    /// Public bridge DTO root.
    Bridge,
    /// Workspace protocol DTO root.
    Protocol,
}

impl SchemaRoot {
    /// Return generated module segments for this schema root.
    fn module_segments(self, segments: &[String]) -> Vec<String> {
        match self {
            Self::Bridge => Self::bridge_segments(segments),
            Self::Protocol => Self::protocol_segments(segments),
        }
    }

    /// Return whether unknown references should keep their module qualifier.
    fn requires_qualified_fallback(self) -> bool {
        matches!(self, Self::Protocol)
    }

    /// Return public bridge generated module segments.
    fn bridge_segments(segments: &[String]) -> Vec<String> {
        let mut segments = segments.to_vec();
        if segments
            .first()
            .is_some_and(|segment| segment == "destack_bridge_language")
        {
            segments.remove(0);
        }

        segments
    }

    /// Return workspace protocol generated module segments.
    fn protocol_segments(segments: &[String]) -> Vec<String> {
        if starts_with_segments(segments, &["destack_workspace", "protocol", "message"]) {
            let mut path = vec!["protocol".to_string()];
            path.extend_from_slice(&segments[3..]);

            return path;
        }

        if starts_with_segments(segments, &["destack_workspace", "protocol"]) {
            let mut path = vec!["protocol".to_string()];
            path.extend_from_slice(&segments[2..]);

            return path;
        }

        if let Some(first) = segments
            .first()
            .and_then(|segment| segment.strip_prefix("destack_"))
        {
            return Self::external_protocol_segments(first, &segments[1..]);
        }

        let mut path = vec!["protocol".to_string()];
        path.extend_from_slice(segments);
        path
    }

    /// Return protocol segments for types pulled from another Destack crate.
    fn external_protocol_segments(crate_name: &str, segments: &[String]) -> Vec<String> {
        let mut path = vec!["protocol".to_string(), crate_name.to_string()];
        if segments
            .first()
            .is_some_and(|segment| segment == crate_name)
        {
            path.extend_from_slice(&segments[1..]);
        } else {
            path.extend_from_slice(segments);
        }

        path
    }
}

/// One bridge source module path.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ModulePath {
    /// Path segments under `bridge/language/src`.
    segments: Vec<String>,
}

impl ModulePath {
    /// Return one bridge source module path from schema module segments.
    pub(super) fn from_segments(root: SchemaRoot, segments: &[String]) -> Result<Self> {
        let segments = root
            .module_segments(segments)
            .into_iter()
            .map(|segment| clean_rust_segment(&segment))
            .collect::<Vec<_>>();
        if segments.is_empty() {
            bail!("bridge schema module path is empty");
        }

        Ok(Self { segments })
    }

    /// Return path segments.
    pub(crate) fn segments(&self) -> &[String] {
        &self.segments
    }

    /// Return this module path joined by `/`.
    pub(crate) fn slash_path(&self) -> String {
        self.segments.join("/")
    }
}

/// Return one Rust module segment without raw identifier syntax.
fn clean_rust_segment(segment: &str) -> String {
    segment.strip_prefix("r#").unwrap_or(segment).to_string()
}

/// Return whether module segments start with one exact prefix.
fn starts_with_segments(segments: &[String], prefix: &[&str]) -> bool {
    segments.len() >= prefix.len()
        && segments
            .iter()
            .zip(prefix.iter())
            .all(|(segment, prefix)| segment == prefix)
}

/// Return generated type names for one registry.
pub(super) fn schema_type_names(
    registry: &destack_serde::SchemaRegistry,
) -> BTreeMap<destack_serde::SchemaName, String> {
    let mut counts = BTreeMap::<String, usize>::new();
    for name in registry.items.keys() {
        *counts.entry(name.name.clone()).or_default() += 1;
    }

    registry
        .items
        .keys()
        .map(|name| {
            let generated = if counts[&name.name] > 1 {
                qualified_schema_type_name(name)
            } else {
                name.name.clone()
            };

            (name.clone(), generated)
        })
        .collect()
}

/// Return the generated type name for one schema item.
pub(super) fn schema_type_name(
    root: SchemaRoot,
    name: &destack_serde::SchemaName,
    names: &BTreeMap<destack_serde::SchemaName, String>,
) -> String {
    names.get(name).cloned().unwrap_or_else(|| {
        if root.requires_qualified_fallback() {
            qualified_schema_type_name(name)
        } else {
            name.name.clone()
        }
    })
}

/// Return a stable qualified name for colliding protocol types.
fn qualified_schema_type_name(name: &destack_serde::SchemaName) -> String {
    let mut segments = name
        .module
        .iter()
        .map(|segment| clean_rust_segment(segment))
        .map(|segment| {
            segment
                .strip_prefix("destack_")
                .unwrap_or(&segment)
                .to_string()
        })
        .map(|segment| upper_camel(&segment))
        .filter(|segment| {
            segment
                .chars()
                .next()
                .is_some_and(|character| character == '_' || character.is_ascii_alphabetic())
        })
        .collect::<Vec<_>>();

    if segments.last().is_some_and(|segment| segment == &name.name) {
        segments.pop();
    }

    let qualifier = segments.join("");
    if qualifier.is_empty() {
        return name.name.clone();
    }

    if name.name.starts_with(&qualifier) {
        name.name.clone()
    } else {
        format!("{qualifier}{}", name.name)
    }
}
