use crate::lsp::{GoTo, LspTestState, Marker, Range};

impl<'a> GoTo<'a> {
    /// Move the caret to one named marker.
    pub fn marker(&mut self, name: &str) -> Result<(), String> {
        self.state.go_to_marker(name)
    }

    /// Visit all markers in source order.
    pub fn each_marker<F>(&mut self, visit: F) -> Result<(), String>
    where
        F: FnMut(&mut LspTestState, &Marker, usize) -> Result<(), String>,
    {
        self.state.go_to_each_marker(visit)
    }

    /// Visit the requested marker subset in the provided order.
    pub fn each_named_marker<F>(&mut self, marker_names: &[&str], visit: F) -> Result<(), String>
    where
        F: FnMut(&mut LspTestState, &Marker, usize) -> Result<(), String>,
    {
        self.state.go_to_each_named_marker(marker_names, visit)
    }

    /// Move the caret to the start of one range by stripped text.
    pub fn range_start(&mut self, text: &str) -> Result<(), String> {
        self.state.go_to_range_start(text)
    }

    /// Visit all ranges in source order.
    pub fn each_range<F>(&mut self, visit: F) -> Result<(), String>
    where
        F: FnMut(&mut LspTestState, &Range, usize) -> Result<(), String>,
    {
        self.state.go_to_each_range(visit)
    }

    /// Move the caret to the beginning of the active file.
    pub fn bof(&mut self) -> Result<(), String> {
        self.state.go_to_bof()
    }

    /// Move the caret to the end of the active file.
    pub fn eof(&mut self) -> Result<(), String> {
        self.state.go_to_eof()
    }

    /// Move the caret to one byte position, optionally focusing one file first.
    pub fn position(&mut self, offset: usize, file_path: Option<&str>) -> Result<(), String> {
        self.state.go_to_position(offset, file_path)
    }

    /// Focus one file.
    pub fn file(&mut self, file_path: &str) -> Result<(), String> {
        self.state.go_to_file(file_path)
    }

    /// Select the text between two markers.
    pub fn select(&mut self, start_marker_name: &str, end_marker_name: &str) -> Result<(), String> {
        self.state.select(start_marker_name, end_marker_name)
    }

    /// Select the full contents of one file.
    pub fn select_all_in_file(&mut self, file_path: &str) -> Result<(), String> {
        self.state.select_all_in_file(file_path)
    }

    /// Select one range.
    pub fn select_range(&mut self, range: &Range) -> Result<(), String> {
        self.state.select_range(range)
    }
}
