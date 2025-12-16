use std::collections::HashMap;

use destack_source::{FileId, Span};

/// A cursor marker ($0, $1, etc.) in test source.
#[derive(Debug, Clone)]
pub struct CursorMarker {
    /// The cursor index (0, 1, etc.).
    pub index: usize,
    /// The offset in the clean source.
    pub offset: u32,
}

/// A range marker (^^^ name) in test source.
#[derive(Debug, Clone)]
pub struct RangeMarker {
    /// The marker name (e.g., "def:foo", "use:foo").
    pub name: String,
    /// The span in the clean source.
    pub span: Span,
    /// Optional target for reference markers (e.g., "def:foo" in "use:foo -> def:foo").
    pub target: Option<String>,
}

/// Expected completion item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedCompletion {
    pub label: String,
    pub kind: String,
    pub detail: Option<String>,
}

/// Expected symbol in document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedSymbol {
    pub name: String,
    pub kind: String,
    pub children: Vec<ExpectedSymbol>,
}

/// Expected folding range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedFold {
    pub start_line: u32,
    pub end_line: u32,
    pub kind: Option<String>,
}

/// Expected inlay hint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedInlayHint {
    pub offset: u32,
    pub label: String,
}

/// Expected highlight range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedHighlight {
    pub start: u32,
    pub end: u32,
    pub kind: Option<String>,
}

/// Test expectations parsed from @expect directives.
#[derive(Debug, Clone, Default)]
pub struct TestExpectations {
    /// Expected completions at cursor: @completion $0: x(field), y(field)
    pub completions: HashMap<usize, Vec<ExpectedCompletion>>,
    /// Expected hover text at cursor: @hover $0: "text"
    pub hover: HashMap<usize, String>,
    /// Expected document symbols: @symbols: Foo(struct), bar(function)
    pub symbols: Vec<ExpectedSymbol>,
    /// Expected folding ranges: @fold: 2-5(function), 7-10
    pub folds: Vec<ExpectedFold>,
    /// Expected inlay hints: @inlay_hint 5: ": Type"
    pub inlay_hints: Vec<ExpectedInlayHint>,
    /// Expected highlights at cursor: @highlight $0: 5-10, 15-20
    pub highlights: HashMap<usize, Vec<ExpectedHighlight>>,
    /// Expected signature at cursor: @signature $0: "sig" [active_param]
    pub signature: HashMap<usize, (String, Option<usize>)>,
    /// Expected reference count: @references def:foo: 3
    pub reference_count: HashMap<String, usize>,
    /// Expected semantic tokens: @semantic 5-10: keyword, 12-15: function
    pub semantic_tokens: Vec<(u32, u32, String)>,
    /// Expected code actions at cursor: @code_action $0: "title1", "title2"
    pub code_actions: HashMap<usize, Vec<String>>,
    /// Expected code lenses: @code_lens 5: "title"
    pub code_lenses: Vec<(u32, String)>,
}

/// Parsed markers from a test source file.
#[derive(Debug, Clone, Default)]
pub struct TestMarkers {
    /// Cursor markers ($0, $1, etc.).
    pub cursors: Vec<CursorMarker>,
    /// Range markers (^^^ name).
    pub ranges: Vec<RangeMarker>,
    /// Test type from header comment.
    pub test_type: Option<String>,
    /// Description from header comment.
    pub description: Option<String>,
    /// Test expectations from @expect directives.
    pub expectations: TestExpectations,
}

impl TestMarkers {
    /// Get a cursor by index.
    pub fn cursor(&self, index: usize) -> Option<&CursorMarker> {
        self.cursors.iter().find(|c| c.index == index)
    }

    /// Get the first cursor (most common case).
    pub fn cursor0(&self) -> Option<&CursorMarker> {
        self.cursor(0)
    }

    /// Get a range marker by name.
    pub fn range(&self, name: &str) -> Option<&RangeMarker> {
        self.ranges.iter().find(|r| r.name == name)
    }

    /// Get all markers with a given prefix (e.g., "def:").
    pub fn with_prefix(&self, prefix: &str) -> Vec<&RangeMarker> {
        self.ranges
            .iter()
            .filter(|r| r.name.starts_with(prefix))
            .collect()
    }

    /// Get all definition markers.
    pub fn definitions(&self) -> Vec<&RangeMarker> {
        self.with_prefix("def:")
    }

    /// Get all use/reference markers.
    pub fn references(&self) -> Vec<&RangeMarker> {
        self.with_prefix("use:")
    }
}

/// Parse test source and extract markers.
///
/// Returns the clean source (without markers) and the extracted markers.
pub fn parse_markers(file_id: FileId, source: &str) -> (String, TestMarkers) {
    let mut markers = TestMarkers::default();
    let mut clean_lines: Vec<String> = Vec::new();

    // track previous line for ^^^ markers
    let mut prev_line_start = 0u32;

    // track multiline directive state
    let mut multiline_state: Option<MultilineDirective> = None;

    for line in source.lines() {
        let trimmed = line.trim();

        // check for header comments (// test: ..., // description: ...)
        if let Some(rest) = trimmed.strip_prefix("// test:") {
            markers.test_type = Some(rest.trim().to_string());
            multiline_state = None;
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("// description:") {
            markers.description = Some(rest.trim().to_string());
            multiline_state = None;
            continue;
        }

        // check for @expect directives (// @completion, // @hover, etc.)
        if let Some(rest) = trimmed.strip_prefix("// @") {
            parse_expect_directive_with_state(
                rest,
                &mut markers.expectations,
                &mut multiline_state,
            );
            continue;
        }

        // check for multiline continuation: "// - item"
        if let Some(rest) = trimmed.strip_prefix("// - ") {
            parse_expect_directive_with_state(
                &format!("- {rest}"),
                &mut markers.expectations,
                &mut multiline_state,
            );
            continue;
        }

        // skip other comment-only lines that look like old expectations
        if trimmed.starts_with("// expected") {
            continue;
        }

        // non-directive line clears multiline state
        if !trimmed.starts_with("//") {
            multiline_state = None;
        }

        // check for marker comment (// ^^^ name or // ^^^ name -> target)
        if let Some(marker_match) = parse_caret_marker(line) {
            let (caret_col, caret_len, name, target) = marker_match;

            // calculate span based on previous line
            let start = prev_line_start + caret_col as u32;
            let end = start + caret_len as u32;

            markers.ranges.push(RangeMarker {
                name,
                span: Span::new(file_id, start, end),
                target,
            });
            // don't add marker line to clean source
            continue;
        }

        // check for cursor markers ($0, $1, etc.) and remove them
        let (clean_line, line_cursors) = extract_cursors(line);

        // calculate line start in clean source
        let line_start = clean_lines
            .iter()
            .map(|l| l.len() as u32 + 1) // +1 for newline
            .sum::<u32>();

        // add cursor markers with adjusted offsets
        for (index, col) in line_cursors {
            markers.cursors.push(CursorMarker {
                index,
                offset: line_start + col as u32,
            });
        }

        // update tracking for ^^^ markers
        prev_line_start = line_start;

        clean_lines.push(clean_line);
    }

    (clean_lines.join("\n"), markers)
}

/// Parse a caret marker line (// ^^^ name or //    ^^^ name -> target).
///
/// Returns (column, length, name, target).
fn parse_caret_marker(line: &str) -> Option<(usize, usize, String, Option<String>)> {
    // find the comment start
    let comment_start = line.find("//")?;
    let after_comment = &line[comment_start + 2..];

    // find the carets
    let caret_start = after_comment.find('^')?;
    let carets = after_comment[caret_start..]
        .chars()
        .take_while(|&c| c == '^')
        .count();

    if carets == 0 {
        return None;
    }

    // the column is based on position in original line (before //)
    let caret_col = comment_start + 2 + caret_start;

    // get the name after carets
    let after_carets = after_comment[caret_start + carets..].trim();

    // check for -> target
    let (name, target) = if let Some(arrow_pos) = after_carets.find("->") {
        let name = after_carets[..arrow_pos].trim().to_string();
        let target = after_carets[arrow_pos + 2..].trim().to_string();
        (name, Some(target))
    } else {
        (after_carets.to_string(), None)
    };

    Some((caret_col, carets, name, target))
}

/// Extract cursor markers ($0, $1, etc.) from a line.
///
/// Returns the clean line and a list of (index, column) pairs.
fn extract_cursors(line: &str) -> (String, Vec<(usize, usize)>) {
    let mut clean = String::new();
    let mut cursors = Vec::new();
    let mut chars = line.chars().peekable();
    let mut col = 0;

    while let Some(c) = chars.next() {
        if c == '$' {
            // check for digit
            if let Some(&digit) = chars.peek()
                && digit.is_ascii_digit()
            {
                let index = digit.to_digit(10).unwrap() as usize;
                chars.next(); // consume digit
                cursors.push((index, col));
                continue;
            }
        }
        clean.push(c);
        col += 1;
    }

    (clean, cursors)
}

/// Current multiline directive being parsed.
#[derive(Debug, Clone)]
enum MultilineDirective {
    Completion(usize),
}

/// Parse an @expect directive and add to expectations:
/// - @completion $0: x(field), y(field)  OR multiline with "- item" lines
/// - @hover $0: "hover text"
/// - @symbols: Foo(struct), bar(function)
/// - @fold: 2-5(function), 7-10
/// - @inlay_hint 5: ": Type"
/// - @highlight $0: 5-10(read), 15-20(write)
/// - @signature $0: "fn(x: i32)" [0]
/// - @references def:foo: 3
/// - @code_action $0: "Extract variable", "Inline"
/// - @code_lens 5: "Run test"
/// - @semantic 5-10: keyword
fn parse_expect_directive_with_state(
    directive: &str,
    expectations: &mut TestExpectations,
    multiline_state: &mut Option<MultilineDirective>,
) {
    let directive = directive.trim();

    // handle multiline continuation: "- item"
    if let Some(item) = directive.strip_prefix("- ") {
        let item = item.trim();
        if let Some(MultilineDirective::Completion(cursor)) = multiline_state
            && let Some(completion) = parse_single_completion(item)
        {
            expectations
                .completions
                .entry(*cursor)
                .or_default()
                .push(completion);
        }
        return;
    }

    // new directive, clear multiline state
    *multiline_state = None;

    // @completion $N: items... OR @completion $N: (start multiline)
    if let Some(rest) = directive.strip_prefix("completion ") {
        if let Some((cursor, items)) = parse_cursor_directive(rest) {
            if items.is_empty() {
                // Start multiline mode
                *multiline_state = Some(MultilineDirective::Completion(cursor));
                expectations.completions.entry(cursor).or_default();
            } else {
                // Inline mode
                let completions = parse_completion_items(items);
                expectations.completions.insert(cursor, completions);
            }
        }
        return;
    }

    // @hover $N: "text"
    if let Some(rest) = directive.strip_prefix("hover ") {
        if let Some((cursor, text)) = parse_cursor_directive(rest) {
            let text = text.trim_matches('"').to_string();
            expectations.hover.insert(cursor, text);
        }
        return;
    }

    // @symbols: Name(kind), Name(kind)
    if let Some(rest) = directive.strip_prefix("symbols:") {
        expectations.symbols = parse_symbol_items(rest.trim());
        return;
    }

    // @fold: 2-5(function), 7-10
    if let Some(rest) = directive.strip_prefix("fold:") {
        expectations.folds = parse_fold_items(rest.trim());
        return;
    }

    // @inlay_hint offset: "label"
    if let Some(rest) = directive.strip_prefix("inlay_hint ") {
        if let Some((offset, label)) = parse_offset_directive(rest) {
            let label = label.trim_matches('"').to_string();
            expectations
                .inlay_hints
                .push(ExpectedInlayHint { offset, label });
        }
        return;
    }

    // @highlight $N: 5-10(read), 15-20
    if let Some(rest) = directive.strip_prefix("highlight ") {
        if let Some((cursor, items)) = parse_cursor_directive(rest) {
            let highlights = parse_highlight_items(items);
            expectations.highlights.insert(cursor, highlights);
        }
        return;
    }

    // @signature $N: "sig" [active]
    if let Some(rest) = directive.strip_prefix("signature ") {
        if let Some((cursor, sig_str)) = parse_cursor_directive(rest) {
            let (sig, active) = parse_signature(sig_str);
            expectations.signature.insert(cursor, (sig, active));
        }
        return;
    }

    // @references marker: count
    if let Some(rest) = directive.strip_prefix("references ") {
        if let Some((marker, count_str)) = rest.split_once(':')
            && let Ok(count) = count_str.trim().parse::<usize>()
        {
            expectations
                .reference_count
                .insert(marker.trim().to_string(), count);
        }
        return;
    }

    // @code_action $N: "title1", "title2"
    if let Some(rest) = directive.strip_prefix("code_action ") {
        if let Some((cursor, items)) = parse_cursor_directive(rest) {
            let actions = parse_quoted_list(items);
            expectations.code_actions.insert(cursor, actions);
        }
        return;
    }

    // @code_lens line: "title"
    if let Some(rest) = directive.strip_prefix("code_lens ") {
        if let Some((line, title)) = parse_offset_directive(rest) {
            let title = title.trim_matches('"').to_string();
            expectations.code_lenses.push((line, title));
        }
        return;
    }

    // @semantic start-end: type
    if let Some(rest) = directive.strip_prefix("semantic ") {
        for token in parse_semantic_tokens(rest) {
            expectations.semantic_tokens.push(token);
        }
    }
}

/// Parse "$N: rest" returning (cursor_index, rest).
fn parse_cursor_directive(s: &str) -> Option<(usize, &str)> {
    let s = s.trim();
    if !s.starts_with('$') {
        return None;
    }
    let rest = &s[1..];
    let digit_end = rest.find(|c: char| !c.is_ascii_digit())?;
    let cursor: usize = rest[..digit_end].parse().ok()?;
    let after = rest[digit_end..].trim();
    let after = after.strip_prefix(':')?;
    Some((cursor, after.trim()))
}

/// Parse "offset: rest" returning (offset, rest).
fn parse_offset_directive(s: &str) -> Option<(u32, &str)> {
    let (offset_str, rest) = s.split_once(':')?;
    let offset: u32 = offset_str.trim().parse().ok()?;
    Some((offset, rest.trim()))
}

/// Parse a single completion item: "x: field" or "x: field: detail"
fn parse_single_completion(item: &str) -> Option<ExpectedCompletion> {
    let item = item.trim();
    if item.is_empty() {
        return None;
    }

    // format: label: kind or label: kind: detail
    let parts: Vec<&str> = item.splitn(3, ':').collect();
    let label = parts.first()?.trim().to_string();
    let kind = parts
        .get(1)
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    let detail = parts.get(2).map(|s| s.trim().to_string());

    if label.is_empty() || kind.is_empty() {
        return None;
    }

    Some(ExpectedCompletion {
        label,
        kind,
        detail,
    })
}

/// Parse inline completion items: "x: field, y: field"
fn parse_completion_items(s: &str) -> Vec<ExpectedCompletion> {
    s.split(',').filter_map(parse_single_completion).collect()
}

/// Parse a single symbol item: "Foo: struct"
fn parse_single_symbol(item: &str) -> Option<ExpectedSymbol> {
    let item = item.trim();
    if item.is_empty() {
        return None;
    }

    let (name, kind) = item.split_once(':')?;
    Some(ExpectedSymbol {
        name: name.trim().to_string(),
        kind: kind.trim().to_string(),
        children: vec![],
    })
}

/// Parse symbol items: "Foo: struct, bar: function"
fn parse_symbol_items(s: &str) -> Vec<ExpectedSymbol> {
    s.split(',').filter_map(parse_single_symbol).collect()
}

/// Parse fold items: "2-5(function), 7-10"
fn parse_fold_items(s: &str) -> Vec<ExpectedFold> {
    s.split(',')
        .filter_map(|item| {
            let item = item.trim();
            if item.is_empty() {
                return None;
            }
            // format: start-end or start-end(kind)
            let (range, kind) = if let Some(paren) = item.find('(') {
                let range = &item[..paren];
                let kind = Some(item[paren + 1..].trim_end_matches(')').trim().to_string());
                (range, kind)
            } else {
                (item, None)
            };

            let (start, end) = range.split_once('-')?;
            let start_line: u32 = start.trim().parse().ok()?;
            let end_line: u32 = end.trim().parse().ok()?;

            Some(ExpectedFold {
                start_line,
                end_line,
                kind,
            })
        })
        .collect()
}

/// Parse highlight items: "5-10(read), 15-20"
fn parse_highlight_items(s: &str) -> Vec<ExpectedHighlight> {
    s.split(',')
        .filter_map(|item| {
            let item = item.trim();
            if item.is_empty() {
                return None;
            }
            let (range, kind) = if let Some(paren) = item.find('(') {
                let range = &item[..paren];
                let kind = Some(item[paren + 1..].trim_end_matches(')').trim().to_string());
                (range, kind)
            } else {
                (item, None)
            };

            let (start, end) = range.split_once('-')?;
            let start: u32 = start.trim().parse().ok()?;
            let end: u32 = end.trim().parse().ok()?;

            Some(ExpectedHighlight { start, end, kind })
        })
        .collect()
}

/// Parse signature: "fn(x: i32)" [0] -> (sig, Some(0))
fn parse_signature(s: &str) -> (String, Option<usize>) {
    let s = s.trim();
    if let Some(bracket) = s.rfind('[') {
        let sig = s[..bracket].trim().trim_matches('"').to_string();
        let active_str = s[bracket + 1..].trim_end_matches(']');
        let active = active_str.trim().parse().ok();
        (sig, active)
    } else {
        (s.trim_matches('"').to_string(), None)
    }
}

/// Parse quoted list: "title1", "title2"
fn parse_quoted_list(s: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;

    for c in s.chars() {
        match c {
            '"' => {
                if in_quotes {
                    result.push(current.clone());
                    current.clear();
                }
                in_quotes = !in_quotes;
            }
            _ if in_quotes => {
                current.push(c);
            }
            _ => {}
        }
    }

    result
}

/// Parse semantic tokens: "5-10: keyword, 12-15: function"
fn parse_semantic_tokens(s: &str) -> Vec<(u32, u32, String)> {
    s.split(',')
        .filter_map(|item| {
            let item = item.trim();
            let (range, kind) = item.split_once(':')?;
            let (start, end) = range.trim().split_once('-')?;
            let start: u32 = start.trim().parse().ok()?;
            let end: u32 = end.trim().parse().ok()?;
            let kind = kind.trim().to_string();
            Some((start, end, kind))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_markers() {
        let source = r#"
const foo = 1;
//    ^^^ def:foo
const bar = foo;
//          ^^^ use:foo -> def:foo
"#;
        let file_id = FileId(0);
        let (clean, markers) = parse_markers(file_id, source);

        assert!(!clean.contains("^^^"));
        assert_eq!(markers.ranges.len(), 2);
        assert_eq!(markers.ranges[0].name, "def:foo");
        assert_eq!(markers.ranges[1].name, "use:foo");
        assert_eq!(markers.ranges[1].target, Some("def:foo".to_string()));
    }

    #[test]
    fn test_parse_cursor_markers() {
        let source = "p.$0";
        let file_id = FileId(0);
        let (clean, markers) = parse_markers(file_id, source);

        assert_eq!(clean, "p.");
        assert_eq!(markers.cursors.len(), 1);
        assert_eq!(markers.cursors[0].index, 0);
        assert_eq!(markers.cursors[0].offset, 2);
    }

    #[test]
    fn test_parse_completion_expectation() {
        let source = r#"
// test: completion
p.$0
// @completion $0: x: field, y: field, magnitude: method
"#;
        let file_id = FileId(0);
        let (_, markers) = parse_markers(file_id, source);

        assert_eq!(markers.test_type, Some("completion".to_string()));
        let completions = markers.expectations.completions.get(&0).unwrap();
        assert_eq!(completions.len(), 3);
        assert_eq!(completions[0].label, "x");
        assert_eq!(completions[0].kind, "field");
        assert_eq!(completions[1].label, "y");
        assert_eq!(completions[2].label, "magnitude");
        assert_eq!(completions[2].kind, "method");
    }

    #[test]
    fn test_parse_hover_expectation() {
        let source = r#"
// test: hover
foo$0
// @hover $0: "Point.x: float32"
"#;
        let file_id = FileId(0);
        let (_, markers) = parse_markers(file_id, source);

        let hover = markers.expectations.hover.get(&0).unwrap();
        assert_eq!(hover, "Point.x: float32");
    }

    #[test]
    fn test_parse_symbols_expectation() {
        let source = r#"
// test: document_symbols
// @symbols: Point: struct, main: function
"#;
        let file_id = FileId(0);
        let (_, markers) = parse_markers(file_id, source);

        let symbols = &markers.expectations.symbols;
        assert_eq!(symbols.len(), 2);
        assert_eq!(symbols[0].name, "Point");
        assert_eq!(symbols[0].kind, "struct");
        assert_eq!(symbols[1].name, "main");
        assert_eq!(symbols[1].kind, "function");
    }

    #[test]
    fn test_parse_fold_expectation() {
        let source = r#"
// test: folding_ranges
// @fold: 2-5(function), 7-10
"#;
        let file_id = FileId(0);
        let (_, markers) = parse_markers(file_id, source);

        let folds = &markers.expectations.folds;
        assert_eq!(folds.len(), 2);
        assert_eq!(folds[0].start_line, 2);
        assert_eq!(folds[0].end_line, 5);
        assert_eq!(folds[0].kind, Some("function".to_string()));
        assert_eq!(folds[1].start_line, 7);
        assert_eq!(folds[1].end_line, 10);
        assert_eq!(folds[1].kind, None);
    }

    #[test]
    fn test_parse_multiline_completion() {
        let source = r#"
// test: completion
p.$0
// @completion $0:
// - x: field
// - y: field
// - magnitude: method
"#;
        let file_id = FileId(0);
        let (_, markers) = parse_markers(file_id, source);

        let completions = markers.expectations.completions.get(&0).unwrap();
        assert_eq!(completions.len(), 3);
        assert_eq!(completions[0].label, "x");
        assert_eq!(completions[0].kind, "field");
        assert_eq!(completions[1].label, "y");
        assert_eq!(completions[2].label, "magnitude");
        assert_eq!(completions[2].kind, "method");
    }
}
