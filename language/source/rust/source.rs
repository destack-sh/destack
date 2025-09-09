#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceId(u32);

impl SourceId {
    pub fn new(id: u32) -> Self {
        Self(id)
    }
}

/// A "source" inside the `SourceMap` (Like a file).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Source {
    /// The id of the SourceFile.
    pub id: SourceId,
    /// The full name or path of the SourceFile.
    pub name: String,
    /// The content of the source file.
    pub content: String,
    /// The length of the SourceFile in bytes.
    pub len: u32,
}

impl Source {
    pub fn new(id: SourceId, name: String, content: String) -> Self {
        let len = content.len() as u32;
        Self {
            id,
            name,
            content,
            len,
        }
    }

    /// Get the line number and column for a given byte index.
    /// Returns (line_index, column_index), both 0-based.
    /// If the byte index is at EOF, returns the last line and its length as column.
    pub fn get_position(&self, byte_index: u32) -> Option<(u32, u32)> {
        let target = byte_index as usize;
        if target > self.content.len() {
            return None;
        }

        let mut line_index: u32 = 0;
        let mut line_start_byte: usize = 0;

        for (i, ch) in self.content.char_indices() {
            if i == target {
                return Some((line_index, (i - line_start_byte) as u32));
            }
            if ch == '\n' {
                // move to next line
                line_index += 1;
                line_start_byte = i + 1;
            }
        }

        // if we didn't return inside the loop, the index may be at EOF
        if target == self.content.len() {
            return Some((line_index, (target - line_start_byte) as u32));
        }
        None
    }

    /// Compute the number of lines in the source (at least 1 for empty content).
    pub fn line_count(&self) -> u32 {
        // count '\n' and add 1
        let mut count: u32 = 1;
        for ch in self.content.chars() {
            if ch == '\n' {
                count += 1;
            }
        }
        count
    }

    /// Get the byte bounds (start, end-exclusive) for a line by 0-based index.
    pub fn get_line_bounds(&self, line_index: u32) -> Option<(usize, usize)> {
        let mut current_line: u32 = 0;
        let mut line_start: usize = 0;
        for (i, ch) in self.content.char_indices() {
            if ch == '\n' {
                if current_line == line_index {
                    return Some((line_start, i));
                }
                current_line += 1;
                line_start = i + 1;
            }
        }
        // handle last line (no trailing newline)
        if current_line == line_index {
            return Some((line_start, self.content.len()));
        }
        None
    }

    /// Get a line as a string slice by 0-based index.
    pub fn get_line(&self, line_index: u32) -> Option<&str> {
        self.get_line_bounds(line_index)
            .and_then(|(s, e)| self.content.get(s..e))
    }
}
