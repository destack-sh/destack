use anyhow::{Result, bail};

use crate::generate::core::lower_camel;

/// One generated schema module path.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ModulePath {
    /// Generated path segments.
    segments: Vec<String>,
}

impl ModulePath {
    /// Return one generated module path from schema module segments.
    pub(super) fn from_segments(segments: &[String]) -> Result<Self> {
        let segments = Self::module_segments(segments)
            .into_iter()
            .map(|segment| Self::clean_segment(&segment))
            .collect::<Vec<_>>();
        if segments.is_empty() {
            bail!("client schema module path is empty");
        }

        Ok(Self { segments })
    }

    /// Return one generated module path from a canonical service name.
    pub(crate) fn from_service(name: &str) -> Result<Self> {
        let mut segments = name.split('.');
        if segments.next() != Some("destack") {
            bail!("client service name must begin with destack: {name}");
        }
        let segments = segments.collect::<Vec<_>>();
        if segments.is_empty() || segments.iter().any(|segment| segment.is_empty()) {
            bail!("client service name has no service: {name}");
        }
        let segments = segments.into_iter().map(lower_camel).collect();

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

    /// Return generated module segments for one reflected Rust module.
    fn module_segments(segments: &[String]) -> Vec<String> {
        if let Some(first) = segments
            .first()
            .and_then(|segment| segment.strip_prefix("destack_"))
        {
            return Self::crate_segments(first, &segments[1..]);
        }

        segments.to_vec()
    }

    /// Return one Rust module segment without raw identifier syntax.
    pub(super) fn clean_segment(segment: &str) -> String {
        segment.strip_prefix("r#").unwrap_or(segment).to_string()
    }

    /// Return generated segments for one Destack crate item.
    pub(super) fn crate_segments(crate_name: &str, segments: &[String]) -> Vec<String> {
        let mut path = vec![crate_name.to_string()];
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
