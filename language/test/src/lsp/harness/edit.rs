use destack_lsp_types as lsp;

use crate::lsp::{LspTestState, OpenDocumentState, Range};

/// One editor-style text mutation operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextEditOperation {
    /// Insert text at the current caret.
    Insert(String),
    /// Replace the current selection with text.
    Replace(String),
    /// Delete the codepoint to the left of the caret.
    DeleteLeft,
    /// Delete the codepoint to the right of the caret.
    DeleteRight,
}

impl LspTestState {
    /// Return the current caret byte offset.
    pub fn caret_offset(&self) -> usize {
        self.editor.caret_offset
    }

    /// Open one fixture-relative file and track its overlay state.
    pub fn open_file(&mut self, file_path: &str) -> Result<(), String> {
        // load the workspace text before publishing the editor overlay
        let text = self.driver.read_file_text(file_path)?;
        self.driver.open_file_with_text(file_path, &text, 1)?;

        self.editor.open_documents.insert(
            file_path.to_string(),
            OpenDocumentState {
                file_path: file_path.to_string(),
                text,
                version: 1,
            },
        );

        // focus the newly opened file when nothing else is active
        if self.editor.active_file_path.is_none() {
            self.editor.active_file_path = Some(file_path.to_string());
            self.editor.caret_offset = 0;
            self.editor.selection_start = None;
            self.editor.selection_end = None;
        }

        Ok(())
    }

    /// Close one open file and drop its overlay state.
    pub fn close_file(&mut self, file_path: &str) -> Result<(), String> {
        // require an open overlay so version tracking stays consistent
        if !self.editor.open_documents.contains_key(file_path) {
            return Err(format!("cannot close unopened file {file_path}"));
        }

        self.driver.close_file(file_path);
        self.editor.open_documents.remove(file_path);

        // clear editor focus when the active file was closed
        if self.editor.active_file_path.as_deref() == Some(file_path) {
            self.editor.active_file_path = None;
            self.editor.caret_offset = 0;
            self.editor.selection_start = None;
            self.editor.selection_end = None;
        }

        Ok(())
    }

    /// Send didSave for one open file.
    pub fn save_file(&mut self, file_path: &str) -> Result<(), String> {
        let document = self
            .editor
            .open_documents
            .get(file_path)
            .cloned()
            .ok_or_else(|| format!("cannot save unopened file {file_path}"))?;

        self.driver.write_file_text(file_path, &document.text)?;
        self.driver.save_file(file_path, Some(document.text));

        Ok(())
    }

    /// Replace the full overlay text for one open file.
    pub fn replace_document_text(&mut self, file_path: &str, text: &str) -> Result<(), String> {
        let document = self
            .editor
            .open_documents
            .get_mut(file_path)
            .ok_or_else(|| format!("cannot change unopened file {file_path}"))?;

        // advance the client version before publishing the full-text change
        document.version += 1;
        document.text = text.to_string();
        self.driver
            .change_file(file_path, &document.text, document.version);

        // clamp the caret into the new text if the active document changed
        if self.editor.active_file_path.as_deref() == Some(file_path) {
            self.editor.caret_offset = self.editor.caret_offset.min(document.text.len());
            self.editor.selection_start = None;
            self.editor.selection_end = None;
        }

        Ok(())
    }

    /// Create one closed workspace file and notify the server.
    pub fn create_file_text(&mut self, file_path: &str, text: &str) -> Result<(), String> {
        self.driver.create_file_text(file_path, text)
    }

    /// Update one closed workspace file and notify the server.
    pub fn replace_closed_file_text(&mut self, file_path: &str, text: &str) -> Result<(), String> {
        self.driver.change_closed_file_text(file_path, text)
    }

    /// Delete one closed workspace file and notify the server.
    pub fn delete_file(&mut self, file_path: &str) -> Result<(), String> {
        // require a closed file so deletion semantics stay explicit
        if self.editor.open_documents.contains_key(file_path) {
            return Err(format!("cannot delete open file {file_path}"));
        }

        self.driver.delete_file_text(file_path)
    }

    /// Insert text at the current caret.
    pub fn insert_text(&mut self, text: &str) -> Result<(), String> {
        let file_path = self
            .editor
            .active_file_path
            .clone()
            .ok_or_else(|| "no active file is focused".to_string())?;
        let document = self
            .editor
            .open_documents
            .get(&file_path)
            .ok_or_else(|| format!("active file {file_path} is not open"))?;
        let offset = self.editor.caret_offset.min(document.text.len());
        let mut updated_text = document.text.clone();

        // insert at the current caret and advance the caret past the inserted text
        updated_text.insert_str(offset, text);
        self.replace_document_text(&file_path, &updated_text)?;
        self.editor.active_file_path = Some(file_path);
        self.editor.caret_offset = offset + text.len();
        self.editor.selection_start = None;
        self.editor.selection_end = None;

        Ok(())
    }

    /// Insert text at the current caret.
    pub fn insert(&mut self, text: &str) -> Result<(), String> {
        self.insert_text(text)
    }

    /// Paste text at the current caret.
    pub fn paste(&mut self, text: &str) -> Result<(), String> {
        self.insert_text(text)
    }

    /// Insert one line of text at the current caret.
    pub fn insert_line(&mut self, text: &str) -> Result<(), String> {
        let line = format!("{text}\n");

        self.insert_text(&line)
    }

    /// Insert multiple lines of text at the current caret.
    pub fn insert_lines<'a, I>(&mut self, lines: I) -> Result<(), String>
    where
        I: IntoIterator<Item = &'a str>,
    {
        let joined_text = lines.into_iter().collect::<Vec<_>>().join("\n");

        self.insert_text(&joined_text)
    }

    /// Replace the current selection with text.
    pub fn replace_selection(&mut self, text: &str) -> Result<(), String> {
        let file_path = self
            .editor
            .active_file_path
            .clone()
            .ok_or_else(|| "no active file is focused".to_string())?;
        let document = self
            .editor
            .open_documents
            .get(&file_path)
            .ok_or_else(|| format!("active file {file_path} is not open"))?;
        let (start_offset, end_offset) = self.selection_offsets()?;
        let mut updated_text = document.text.clone();

        // replace the selected byte range and collapse the caret to the inserted text
        updated_text.replace_range(start_offset..end_offset, text);
        self.replace_document_text(&file_path, &updated_text)?;
        self.editor.active_file_path = Some(file_path);
        self.editor.caret_offset = start_offset + text.len();
        self.editor.selection_start = None;
        self.editor.selection_end = None;

        Ok(())
    }

    /// Replace a byte range in the active document with text.
    pub fn replace(
        &mut self,
        start_offset: usize,
        length: usize,
        text: &str,
    ) -> Result<(), String> {
        let file_path = self
            .editor
            .active_file_path
            .clone()
            .ok_or_else(|| "no active file is focused".to_string())?;
        let document = self
            .editor
            .open_documents
            .get(&file_path)
            .ok_or_else(|| format!("active file {file_path} is not open"))?;
        let end_offset = start_offset
            .checked_add(length)
            .ok_or_else(|| "replace length overflowed the active document".to_string())?;
        if end_offset > document.text.len() {
            return Err(format!(
                "replace range {start_offset}..{end_offset} exceeds document length {}",
                document.text.len()
            ));
        }

        let mut updated_text = document.text.clone();

        // replace the requested range and collapse the caret to the inserted text
        updated_text.replace_range(start_offset..end_offset, text);
        self.replace_document_text(&file_path, &updated_text)?;
        self.editor.active_file_path = Some(file_path);
        self.editor.caret_offset = start_offset + text.len();
        self.editor.selection_start = None;
        self.editor.selection_end = None;

        Ok(())
    }

    /// Delete one zero-based line from the active document.
    pub fn delete_line(&mut self, index: usize) -> Result<(), String> {
        let file_path = self
            .editor
            .active_file_path
            .clone()
            .ok_or_else(|| "no active file is focused".to_string())?;
        let document = self
            .editor
            .open_documents
            .get(&file_path)
            .ok_or_else(|| format!("active file {file_path} is not open"))?;
        let (start_offset, end_offset) = line_range_for_index(&document.text, index)?;
        let mut updated_text = document.text.clone();

        // remove the requested line, including its trailing newline when present
        updated_text.replace_range(start_offset..end_offset, "");
        self.replace_document_text(&file_path, &updated_text)?;
        self.editor.active_file_path = Some(file_path);
        self.editor.caret_offset = start_offset.min(updated_text.len());
        self.editor.selection_start = None;
        self.editor.selection_end = None;

        Ok(())
    }

    /// Delete one inclusive zero-based line range from the active document.
    pub fn delete_line_range(
        &mut self,
        start_index: usize,
        end_index_inclusive: usize,
    ) -> Result<(), String> {
        if end_index_inclusive < start_index {
            return Err(format!(
                "delete line range {start_index}..{end_index_inclusive} is inverted"
            ));
        }

        let file_path = self
            .editor
            .active_file_path
            .clone()
            .ok_or_else(|| "no active file is focused".to_string())?;
        let document = self
            .editor
            .open_documents
            .get(&file_path)
            .ok_or_else(|| format!("active file {file_path} is not open"))?;
        let (start_offset, _) = line_range_for_index(&document.text, start_index)?;
        let (_, end_offset) = line_range_for_index(&document.text, end_index_inclusive)?;
        let mut updated_text = document.text.clone();

        // remove the requested inclusive line span in one rewrite
        updated_text.replace_range(start_offset..end_offset, "");
        self.replace_document_text(&file_path, &updated_text)?;
        self.editor.active_file_path = Some(file_path);
        self.editor.caret_offset = start_offset.min(updated_text.len());
        self.editor.selection_start = None;
        self.editor.selection_end = None;

        Ok(())
    }

    /// Replace one zero-based line in the active document.
    pub fn replace_line(&mut self, index: usize, text: &str) -> Result<(), String> {
        let file_path = self
            .editor
            .active_file_path
            .clone()
            .ok_or_else(|| "no active file is focused".to_string())?;
        let document = self
            .editor
            .open_documents
            .get(&file_path)
            .ok_or_else(|| format!("active file {file_path} is not open"))?;
        let (start_offset, end_offset) = line_range_for_index(&document.text, index)?;
        let mut replacement = text.to_string();
        let has_trailing_newline = document
            .text
            .get(end_offset.saturating_sub(1)..end_offset)
            .is_some_and(|slice| slice == "\n");

        // keep the original newline shape for the replaced line
        if has_trailing_newline {
            replacement.push('\n');
        }

        let mut updated_text = document.text.clone();
        updated_text.replace_range(start_offset..end_offset, &replacement);
        self.replace_document_text(&file_path, &updated_text)?;
        self.editor.active_file_path = Some(file_path);
        self.editor.caret_offset = start_offset + text.len();
        self.editor.selection_start = None;
        self.editor.selection_end = None;

        Ok(())
    }

    /// Delete the codepoint to the left of the caret.
    pub fn delete_left(&mut self, count: usize) -> Result<(), String> {
        let file_path = self
            .editor
            .active_file_path
            .clone()
            .ok_or_else(|| "no active file is focused".to_string())?;
        let document = self
            .editor
            .open_documents
            .get(&file_path)
            .ok_or_else(|| format!("active file {file_path} is not open"))?;
        let start_offset = previous_char_boundary(&document.text, self.editor.caret_offset, count)?;
        let end_offset = self.editor.caret_offset.min(document.text.len());
        let mut updated_text = document.text.clone();

        // remove codepoints behind the caret and collapse the caret backward
        updated_text.replace_range(start_offset..end_offset, "");
        self.replace_document_text(&file_path, &updated_text)?;
        self.editor.active_file_path = Some(file_path);
        self.editor.caret_offset = start_offset;
        self.editor.selection_start = None;
        self.editor.selection_end = None;

        Ok(())
    }

    /// Delete the codepoint to the left of the caret.
    pub fn backspace(&mut self, count: usize) -> Result<(), String> {
        self.delete_left(count)
    }

    /// Delete the codepoint to the right of the caret.
    pub fn delete_right(&mut self, count: usize) -> Result<(), String> {
        let file_path = self
            .editor
            .active_file_path
            .clone()
            .ok_or_else(|| "no active file is focused".to_string())?;
        let document = self
            .editor
            .open_documents
            .get(&file_path)
            .ok_or_else(|| format!("active file {file_path} is not open"))?;
        let start_offset = self.editor.caret_offset.min(document.text.len());
        let end_offset = next_char_boundary(&document.text, start_offset, count)?;
        let mut updated_text = document.text.clone();

        // remove codepoints ahead of the caret and keep the caret stable
        updated_text.replace_range(start_offset..end_offset, "");
        self.replace_document_text(&file_path, &updated_text)?;
        self.editor.active_file_path = Some(file_path);
        self.editor.caret_offset = start_offset;
        self.editor.selection_start = None;
        self.editor.selection_end = None;

        Ok(())
    }

    /// Delete the codepoint at the current caret.
    pub fn delete_at_caret(&mut self, count: usize) -> Result<(), String> {
        self.delete_right(count)
    }

    /// Move the caret right by a number of codepoints.
    pub fn move_caret_right(&mut self, count: usize) -> Result<(), String> {
        let file_path = self
            .editor
            .active_file_path
            .clone()
            .ok_or_else(|| "no active file is focused".to_string())?;
        let document = self
            .editor
            .open_documents
            .get(&file_path)
            .ok_or_else(|| format!("active file {file_path} is not open"))?;
        let next_offset = next_char_boundary(&document.text, self.editor.caret_offset, count)?;

        self.editor.active_file_path = Some(file_path);
        self.editor.caret_offset = next_offset;
        self.editor.selection_start = None;
        self.editor.selection_end = None;

        Ok(())
    }

    /// Move the caret right by a number of codepoints.
    pub fn move_right(&mut self, count: usize) -> Result<(), String> {
        self.move_caret_right(count)
    }

    /// Move the caret left by a number of codepoints.
    pub fn move_caret_left(&mut self, count: usize) -> Result<(), String> {
        let file_path = self
            .editor
            .active_file_path
            .clone()
            .ok_or_else(|| "no active file is focused".to_string())?;
        let document = self
            .editor
            .open_documents
            .get(&file_path)
            .ok_or_else(|| format!("active file {file_path} is not open"))?;
        let previous_offset =
            previous_char_boundary(&document.text, self.editor.caret_offset, count)?;

        self.editor.active_file_path = Some(file_path);
        self.editor.caret_offset = previous_offset;
        self.editor.selection_start = None;
        self.editor.selection_end = None;

        Ok(())
    }

    /// Move the caret left by a number of codepoints.
    pub fn move_left(&mut self, count: usize) -> Result<(), String> {
        self.move_caret_left(count)
    }

    /// Apply one ordered LSP text edit list to an open document overlay and sync the server view.
    pub fn apply_text_edits(
        &mut self,
        file_path: &str,
        edits: &[lsp::TextEdit],
    ) -> Result<(), String> {
        let (updated_text, next_version) = {
            let document = self
                .editor
                .open_documents
                .get_mut(file_path)
                .ok_or_else(|| format!("cannot apply edits to unopened file {file_path}"))?;
            let updated_text = apply_text_edits(&document.text, edits)?;

            // advance the overlay version before publishing the synced full-text change
            document.version += 1;
            document.text = updated_text.clone();
            (updated_text, document.version)
        };

        self.driver
            .change_file(file_path, &updated_text, next_version);

        Ok(())
    }

    /// Return the current overlay text for one open document.
    pub fn current_document_text(&self, file_path: &str) -> Result<&str, String> {
        let document = self
            .editor
            .open_documents
            .get(file_path)
            .ok_or_else(|| format!("cannot read unopened file {file_path}"))?;

        Ok(&document.text)
    }

    /// Return the current overlay version for one open document.
    pub fn current_document_version(&self, file_path: &str) -> Result<i32, String> {
        let document = self
            .editor
            .open_documents
            .get(file_path)
            .ok_or_else(|| format!("cannot read unopened file {file_path}"))?;

        Ok(document.version)
    }

    /// Return the current overlay text for the active document.
    pub fn current_file_content(&self) -> Result<&str, String> {
        let file_path = self
            .editor
            .active_file_path
            .as_deref()
            .ok_or_else(|| "no active file is focused".to_string())?;

        self.current_document_text(file_path)
    }

    /// Return the current line content for the active document.
    pub fn current_line_content(&self) -> Result<&str, String> {
        let file_path = self
            .editor
            .active_file_path
            .as_deref()
            .ok_or_else(|| "no active file is focused".to_string())?;
        let document = self
            .editor
            .open_documents
            .get(file_path)
            .ok_or_else(|| format!("active file {file_path} is not open"))?;
        let line_range = line_bounds_for_offset(&document.text, self.editor.caret_offset)?;

        document
            .text
            .get(line_range.0..line_range.1)
            .ok_or_else(|| "current line bounds do not align to char boundaries".to_string())
    }

    /// Return the active text slice starting at the caret with the expected text length.
    pub fn text_at_caret(&self, expected_text: &str) -> Result<&str, String> {
        let file_path = self
            .editor
            .active_file_path
            .as_deref()
            .ok_or_else(|| "no active file is focused".to_string())?;
        let document = self
            .editor
            .open_documents
            .get(file_path)
            .ok_or_else(|| format!("active file {file_path} is not open"))?;
        let end_offset = self
            .editor
            .caret_offset
            .checked_add(expected_text.len())
            .ok_or_else(|| "caret slice overflowed the active document".to_string())?;

        document
            .text
            .get(self.editor.caret_offset..end_offset)
            .ok_or_else(|| "caret slice does not align to char boundaries".to_string())
    }

    /// Return the selected byte offsets in ascending order.
    fn selection_offsets(&self) -> Result<(usize, usize), String> {
        let start_offset = self
            .editor
            .selection_start
            .ok_or_else(|| "no active selection".to_string())?;
        let end_offset = self
            .editor
            .selection_end
            .ok_or_else(|| "no active selection".to_string())?;

        Ok((start_offset.min(end_offset), start_offset.max(end_offset)))
    }

    /// Return the current selection as one live range in the active document.
    pub fn current_selection_range(&self) -> Result<Range, String> {
        let file_path = self
            .editor
            .active_file_path
            .clone()
            .ok_or_else(|| "no active file is focused".to_string())?;
        let document = self
            .editor
            .open_documents
            .get(&file_path)
            .ok_or_else(|| format!("active file {file_path} is not open"))?;
        let (start_offset, end_offset) = self.selection_offsets()?;
        let (start_line, start_character) =
            line_and_character_for_offset(&document.text, start_offset)?;
        let (end_line, end_character) = line_and_character_for_offset(&document.text, end_offset)?;
        let text = document
            .text
            .get(start_offset..end_offset)
            .ok_or_else(|| "selection range does not align to char boundaries".to_string())?;

        Ok(Range {
            file_path,
            start_offset,
            end_offset,
            start_line,
            start_character,
            end_line,
            end_character,
            text: text.to_string(),
        })
    }
}

/// Apply one ordered LSP text edit list to source text.
fn apply_text_edits(text: &str, edits: &[lsp::TextEdit]) -> Result<String, String> {
    let mut ranged_edits = edits
        .iter()
        .map(|edit| {
            let start_offset = offset_for_position(text, edit.range.start)?;
            let end_offset = offset_for_position(text, edit.range.end)?;

            Ok((start_offset, end_offset, edit.new_text.as_str()))
        })
        .collect::<Result<Vec<_>, String>>()?;

    // apply from the end so earlier ranges remain stable
    ranged_edits.sort_by(|left, right| right.0.cmp(&left.0));
    let mut updated_text = text.to_string();

    // rewrite each edit range against the current text
    for (start_offset, end_offset, new_text) in ranged_edits {
        updated_text.replace_range(start_offset..end_offset, new_text);
    }

    Ok(updated_text)
}

/// Convert one LSP position into a byte offset in UTF-8 text.
fn offset_for_position(text: &str, position: lsp::Position) -> Result<usize, String> {
    let target_line = position.line as usize;
    let target_character = position.character as usize;
    let mut line = 0usize;
    let mut character = 0usize;

    // walk character boundaries until the requested line and column are reached
    for (offset, ch) in text.char_indices() {
        if line == target_line && character == target_character {
            return Ok(offset);
        }

        if ch == '\n' {
            line += 1;
            character = 0;
            continue;
        }

        character += 1;
    }

    // allow positions at end of document or end of line
    if line == target_line && character == target_character {
        return Ok(text.len());
    }

    Err(format!(
        "position {}:{} is outside document bounds",
        position.line, position.character
    ))
}

/// Return the previous char boundary by count codepoints.
fn previous_char_boundary(text: &str, offset: usize, count: usize) -> Result<usize, String> {
    if offset > text.len() {
        return Err(format!(
            "offset {offset} exceeds document length {}",
            text.len()
        ));
    }

    let mut boundary = offset;
    for _ in 0..count {
        let prefix = text
            .get(..boundary)
            .ok_or_else(|| format!("offset {boundary} does not align to a char boundary"))?;
        let Some((previous_offset, _)) = prefix.char_indices().last() else {
            return Ok(0);
        };
        boundary = previous_offset;
    }

    Ok(boundary)
}

/// Return the next char boundary by count codepoints.
fn next_char_boundary(text: &str, offset: usize, count: usize) -> Result<usize, String> {
    if offset > text.len() {
        return Err(format!(
            "offset {offset} exceeds document length {}",
            text.len()
        ));
    }

    let mut boundary = offset;
    for _ in 0..count {
        let suffix = text
            .get(boundary..)
            .ok_or_else(|| format!("offset {boundary} does not align to a char boundary"))?;
        let Some(character) = suffix.chars().next() else {
            return Ok(text.len());
        };
        boundary += character.len_utf8();
    }

    Ok(boundary)
}

/// Return the current line bounds for one byte offset.
fn line_bounds_for_offset(text: &str, offset: usize) -> Result<(usize, usize), String> {
    if offset > text.len() {
        return Err(format!(
            "offset {offset} exceeds document length {}",
            text.len()
        ));
    }

    let prefix = text
        .get(..offset)
        .ok_or_else(|| format!("offset {offset} does not align to a char boundary"))?;
    let line_start = prefix.rfind('\n').map(|index| index + 1).unwrap_or(0);
    let suffix = text
        .get(offset..)
        .ok_or_else(|| format!("offset {offset} does not align to a char boundary"))?;
    let line_end = suffix
        .find('\n')
        .map(|index| offset + index)
        .unwrap_or(text.len());

    Ok((line_start, line_end))
}

/// Convert one byte offset into a zero-based line and character pair.
fn line_and_character_for_offset(text: &str, offset: usize) -> Result<(usize, usize), String> {
    if offset > text.len() {
        return Err(format!(
            "offset {offset} exceeds document length {}",
            text.len()
        ));
    }

    let prefix = text
        .get(..offset)
        .ok_or_else(|| format!("offset {offset} does not align to a char boundary"))?;
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count();
    let line_start = prefix.rfind('\n').map(|index| index + 1).unwrap_or(0);
    let character = prefix
        .get(line_start..)
        .ok_or_else(|| "line start does not align to char boundaries".to_string())?
        .chars()
        .count();

    Ok((line, character))
}

/// Return the byte range for one zero-based line, including a trailing newline when present.
fn line_range_for_index(text: &str, target_index: usize) -> Result<(usize, usize), String> {
    let mut line_index = 0usize;
    let mut line_start = 0usize;

    // walk line starts until the requested line is reached
    while line_index < target_index {
        let suffix = text
            .get(line_start..)
            .ok_or_else(|| format!("line start {line_start} is not a char boundary"))?;
        let Some(relative_newline) = suffix.find('\n') else {
            return Err(format!(
                "line index {target_index} exceeds document line count"
            ));
        };

        line_start += relative_newline + 1;
        line_index += 1;
    }

    let suffix = text
        .get(line_start..)
        .ok_or_else(|| format!("line start {line_start} is not a char boundary"))?;
    let line_end = if let Some(relative_newline) = suffix.find('\n') {
        line_start + relative_newline + 1
    } else {
        text.len()
    };

    Ok((line_start, line_end))
}
