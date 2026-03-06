use std::collections::HashMap;

use crate::lsp::{Marker, NormalizedDiagnostic, Range, Test, TextSpan};

impl<'a> Test<'a> {
    /// Return all markers in source order.
    pub fn markers(&mut self) -> Vec<Marker> {
        self.state.markers().into_iter().cloned().collect()
    }

    /// Return all marker names in source order.
    pub fn marker_names(&mut self) -> Vec<String> {
        self.state.marker_names()
    }

    /// Return one marker by name.
    pub fn marker(&mut self, name: &str) -> Result<Marker, String> {
        self.state
            .marker(name)
            .cloned()
            .ok_or_else(|| format!("fixture is missing /*{name}*/ marker"))
    }

    /// Return one marker name for one marker.
    pub fn marker_name(&mut self, marker: &Marker) -> Result<String, String> {
        let marker_name = self
            .state
            .fixture
            .markers
            .iter()
            .find_map(|(name, candidate)| (candidate == marker).then(|| name.clone()))
            .ok_or_else(|| "marker does not belong to the current fixture".to_string())?;

        Ok(marker_name)
    }

    /// Return all ranges in source order.
    pub fn ranges(&mut self) -> Vec<Range> {
        self.state.ranges().to_vec()
    }

    /// Return all ranges for one file, or every file when none is provided.
    pub fn ranges_in_file(&mut self, file_path: Option<&str>) -> Vec<Range> {
        self.state
            .ranges_in_file(file_path)
            .into_iter()
            .cloned()
            .collect()
    }

    /// Return all spans for one file, or every file when none is provided.
    pub fn spans(&mut self, file_path: Option<&str>) -> Vec<TextSpan> {
        self.state.spans(file_path)
    }

    /// Return all ranges grouped by stripped text.
    pub fn ranges_by_text(&mut self) -> HashMap<String, Vec<Range>> {
        self.state
            .ranges_by_text()
            .into_iter()
            .map(|(text, ranges)| (text, ranges.into_iter().cloned().collect()))
            .collect()
    }

    /// Return one marker by name.
    pub fn marker_by_name(&mut self, name: &str) -> Result<Marker, String> {
        self.marker(name)
    }

    /// Return normalized semantic diagnostics for one file, or the active file when none is provided.
    pub fn semantic_diagnostics(
        &mut self,
        file_path: Option<&str>,
    ) -> Result<Vec<NormalizedDiagnostic>, String> {
        self.state.semantic_diagnostics(file_path)
    }
}
