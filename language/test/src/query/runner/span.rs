use destack_source::{FileId, Span};

use crate::query::{QueryTestSession, TestFile};

/// Get the test file for a file id.
pub fn file_for(session: &QueryTestSession, file_id: FileId) -> Option<&TestFile> {
    session.files.values().find(|file| file.file_id == file_id)
}

/// Get the source text for a file id.
pub fn source_for_file(session: &QueryTestSession, file_id: FileId) -> &str {
    file_for(session, file_id)
        .map(|file| file.source.as_str())
        .unwrap_or(session.source.as_str())
}

/// Format a span as a file plus 1 based line and column range.
pub fn format_span_for_session(session: &QueryTestSession, span: Span) -> String {
    // resolve the file name
    let file_name = file_for(session, span.file)
        .map(|file| file.name.as_str())
        .unwrap_or("<unknown>");

    // format the line and column range
    let source = source_for_file(session, span.file);
    let range = format_span_line_col(source, span);

    format!("{file_name}:{range}")
}

/// Format a span as a 1 based line and column range.
pub fn format_span_line_col(source: &str, span: Span) -> String {
    // compute line starts once for this source
    let line_starts = compute_line_starts(source);

    // resolve start and end positions
    let start = offset_to_line_col(&line_starts, span.start);
    let end = offset_to_line_col(&line_starts, span.end);

    format!("{}:{}-{}:{}", start.0, start.1, end.0, end.1)
}

/// Convert an offset to a 1 based line and column.
pub fn offset_to_line_col(line_starts: &[u32], offset: u32) -> (u32, u32) {
    // find the last line start at or before the offset
    let line_index = line_starts.partition_point(|&start| start <= offset);
    let line_index = line_index.saturating_sub(1);
    let line_start = line_starts.get(line_index).copied().unwrap_or(0);

    // convert to 1 based line and column
    let line = u32::try_from(line_index + 1).unwrap_or(u32::MAX);
    let col = offset.saturating_sub(line_start).saturating_add(1);
    (line, col)
}

/// Compute the start offset of each line.
pub fn compute_line_starts(source: &str) -> Vec<u32> {
    let mut starts = vec![0u32];
    for (index, byte) in source.as_bytes().iter().enumerate() {
        if *byte == b'\n' {
            let next = u32::try_from(index + 1).unwrap_or(u32::MAX);
            starts.push(next);
        }
    }
    starts
}

/// Get the number of lines in a source file.
pub fn line_count(source: &str) -> u32 {
    // compute line starts for the source
    let starts = compute_line_starts(source);

    // convert the line count to u32
    u32::try_from(starts.len()).unwrap_or(u32::MAX)
}

/// Get the maximum 0 based line index for a source file.
pub fn max_line_index(source: &str) -> u32 {
    // compute the maximum valid line index
    line_count(source).saturating_sub(1)
}
