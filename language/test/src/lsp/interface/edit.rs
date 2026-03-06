use crate::lsp::{Edit, Marker};

impl<'a> Edit<'a> {
    /// Return the current caret position as a synthetic marker snapshot.
    pub fn caret_position(&mut self) -> Result<Marker, String> {
        let (file_path, position) = self.state.current_position()?;

        Ok(Marker {
            name: String::new(),
            file_path,
            offset: self.state.caret_offset(),
            line: position.line as usize,
            character: position.character as usize,
        })
    }

    /// Delete codepoints behind the caret.
    pub fn backspace(&mut self, count: usize) -> Result<(), String> {
        self.state.backspace(count)
    }

    /// Delete codepoints at the caret.
    pub fn delete_at_caret(&mut self, count: usize) -> Result<(), String> {
        self.state.delete_at_caret(count)
    }

    /// Replace one byte range in the active document.
    pub fn replace(
        &mut self,
        start_offset: usize,
        length: usize,
        text: &str,
    ) -> Result<(), String> {
        self.state.replace(start_offset, length, text)
    }

    /// Replace the current selection with text.
    pub fn replace_selection(&mut self, text: &str) -> Result<(), String> {
        self.state.replace_selection(text)
    }

    /// Paste text at the current caret.
    pub fn paste(&mut self, text: &str) -> Result<(), String> {
        self.state.paste(text)
    }

    /// Insert text at the current caret.
    pub fn insert(&mut self, text: &str) -> Result<(), String> {
        self.state.insert(text)
    }

    /// Insert one line at the current caret.
    pub fn insert_line(&mut self, text: &str) -> Result<(), String> {
        self.state.insert_line(text)
    }

    /// Insert multiple lines at the current caret.
    pub fn insert_lines<'b, I>(&mut self, lines: I) -> Result<(), String>
    where
        I: IntoIterator<Item = &'b str>,
    {
        self.state.insert_lines(lines)
    }

    /// Delete one zero-based line.
    pub fn delete_line(&mut self, index: usize) -> Result<(), String> {
        self.state.delete_line(index)
    }

    /// Delete one inclusive zero-based line range.
    pub fn delete_line_range(
        &mut self,
        start_index: usize,
        end_index_inclusive: usize,
    ) -> Result<(), String> {
        self.state
            .delete_line_range(start_index, end_index_inclusive)
    }

    /// Replace one zero-based line.
    pub fn replace_line(&mut self, index: usize, text: &str) -> Result<(), String> {
        self.state.replace_line(index, text)
    }

    /// Move the caret right by a number of codepoints.
    pub fn move_right(&mut self, count: usize) -> Result<(), String> {
        self.state.move_right(count)
    }

    /// Move the caret left by a number of codepoints.
    pub fn move_left(&mut self, count: usize) -> Result<(), String> {
        self.state.move_left(count)
    }

    /// Enable application of formatting edits.
    pub fn enable_formatting(&mut self) {
        self.state.formatting.is_enabled = true;
    }

    /// Disable application of formatting edits.
    pub fn disable_formatting(&mut self) {
        self.state.formatting.is_enabled = false;
    }
}
