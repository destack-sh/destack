/// Find the bounds of the import statement that contains the cursor.
pub(crate) fn find_import_statement(source: &str, offset: usize) -> Option<(usize, usize)> {
    // find the nearest import keyword before the cursor
    let statement_start = source[..offset].rfind("import")?;
    if source[statement_start..offset].contains(';') {
        return None;
    }

    // find the end of the import statement
    let statement_end = source[statement_start..]
        .find(';')
        .map(|idx| statement_start + idx)
        .unwrap_or(source.len());

    // return the statement bounds
    Some((statement_start, statement_end))
}

/// Parse the target module string from an import statement.
pub(crate) fn parse_import_target_by_text(statement: &str) -> Option<String> {
    // locate the from keyword
    let from_idx = statement.find("from")?;
    let after_from = &statement[from_idx + 4..];

    // locate the opening quote
    let mut quote_idx = None;
    let mut quote_char = '"';
    for (idx, ch) in after_from.char_indices() {
        if ch == '"' || ch == '\'' {
            quote_idx = Some(idx);
            quote_char = ch;
            break;
        }
    }
    let quote_idx = quote_idx?;
    let rest = &after_from[quote_idx + 1..];

    // locate the closing quote
    let end_idx = rest.find(quote_char)?;

    // return the parsed target path
    Some(rest[..end_idx].to_string())
}

/// Parse the partially typed import path from a statement and cursor offset.
pub(crate) fn parse_import_path_by_text(statement: &str, cursor_in_stmt: usize) -> Option<String> {
    // locate the from keyword before the cursor
    let from_idx = statement[..cursor_in_stmt.min(statement.len())].rfind("from")?;
    let after_from = &statement[from_idx + 4..];
    let cursor_after_from = cursor_in_stmt.saturating_sub(from_idx + 4);
    let before_cursor = &after_from[..cursor_after_from.min(after_from.len())];

    // locate the nearest opening quote before the cursor
    let mut opening_idx = None;
    let mut quote_char = '"';
    if let Some(idx) = before_cursor.rfind('"') {
        opening_idx = Some(idx);
        quote_char = '"';
    }
    if let Some(idx) = before_cursor.rfind('\'') {
        let current = opening_idx.unwrap_or(0);
        if opening_idx.is_none() || idx > current {
            opening_idx = Some(idx);
            quote_char = '\'';
        }
    }
    let opening_idx = opening_idx?;
    let opening_in_stmt = from_idx + 4 + opening_idx;
    let path_start = opening_in_stmt + 1;

    // return an empty prefix when cursor is before the path
    if cursor_in_stmt <= path_start {
        return Some(String::new());
    }

    // ignore when cursor is past the closing quote
    let rest = &statement[path_start..];
    if let Some(end_idx) = rest.find(quote_char) {
        let closing_in_stmt = path_start + end_idx;
        if cursor_in_stmt > closing_in_stmt {
            return None;
        }
    }

    // slice the prefix up to the cursor
    let end = cursor_in_stmt.min(statement.len());

    // return the partial path
    statement.get(path_start..end).map(str::to_string)
}

/// Parse import clause names and cursor context from raw text.
pub(crate) fn parse_import_clause_by_text(
    statement: &str,
    cursor_in_stmt: usize,
) -> (Vec<String>, bool, bool) {
    // locate the import clause braces
    let Some(open_idx) = statement.find('{') else {
        return (Vec::new(), false, false);
    };
    let Some(close_idx) = statement.rfind('}') else {
        return (Vec::new(), false, false);
    };
    if close_idx <= open_idx + 1 {
        return (Vec::new(), false, false);
    }

    // ensure the cursor is inside the clause
    let cursor_in_clause = cursor_in_stmt > open_idx && cursor_in_stmt <= close_idx;
    if !cursor_in_clause {
        return (Vec::new(), false, false);
    }

    // seed names from the prefix and prepare state
    let mut existing_names = parse_import_prefix_names(&statement[..open_idx]);
    let mut cursor_is_type = false;
    let mut segment_start = open_idx + 1;

    // walk comma separated segments
    for (idx, ch) in statement[open_idx + 1..close_idx].char_indices() {
        if ch != ',' {
            continue;
        }

        let segment_end = open_idx + 1 + idx;
        let is_cursor_segment = cursor_in_stmt >= segment_start && cursor_in_stmt <= segment_end;
        let segment = statement[segment_start..segment_end].trim();

        if !segment.is_empty() {
            let parsed = parse_import_item_names(segment);
            if is_cursor_segment {
                cursor_is_type = parsed.is_type;
            } else {
                existing_names.extend(parsed.names);
            }
        }

        segment_start = segment_end + 1;
    }

    // handle the final segment after the last comma
    let last_segment = statement[segment_start..close_idx].trim();
    let cursor_in_last = cursor_in_stmt >= segment_start && cursor_in_stmt <= close_idx;
    if !last_segment.is_empty() {
        let parsed = parse_import_item_names(last_segment);
        if cursor_in_last {
            cursor_is_type = parsed.is_type;
        } else {
            existing_names.extend(parsed.names);
        }
    } else if cursor_in_last {
        // allow a trailing type keyword segment
        let prefix = statement[open_idx + 1..cursor_in_stmt.min(close_idx)].trim();
        if prefix == "type" {
            cursor_is_type = true;
        }
    }

    // return parsed names and cursor flags
    (existing_names, cursor_is_type, true)
}

/// Parse names from the import prefix segment.
fn parse_import_prefix_names(prefix: &str) -> Vec<String> {
    // normalize the prefix text
    let prefix = prefix.trim();
    let prefix = prefix.strip_prefix("import").unwrap_or(prefix).trim();
    let prefix = prefix.strip_prefix("type").unwrap_or(prefix).trim();
    if prefix.is_empty() {
        return Vec::new();
    }

    // parse comma separated items
    let mut names = Vec::new();
    for segment in prefix.split(',') {
        let segment = segment.trim();
        if segment.is_empty() {
            continue;
        }

        let parsed = parse_import_item_names(segment);
        names.extend(parsed.names);
    }

    // return the collected names
    names
}

/// Parsed names and flags for an import item segment.
struct ParsedImportItem {
    /// The parsed names for the item.
    names: Vec<String>,
    /// Whether the item is marked as type only.
    is_type: bool,
}

/// Parse a clause segment into names and type flag.
fn parse_import_item_names(segment: &str) -> ParsedImportItem {
    // tokenize and clean the segment
    let cleaned = segment
        .split_whitespace()
        .map(clean_import_token)
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();
    let mut tokens = cleaned.iter().map(String::as_str);

    // read the first token
    let Some(first) = tokens.next() else {
        return ParsedImportItem {
            names: Vec::new(),
            is_type: false,
        };
    };

    // handle optional leading type keyword
    let mut is_type = false;
    let mut names = Vec::new();
    let mut current = first;

    if current == "type" {
        is_type = true;
        let Some(next) = tokens.next() else {
            return ParsedImportItem { names, is_type };
        };
        current = next;
    }

    // handle namespace imports
    if current == "*" {
        if let Some(alias) = parse_alias(tokens) {
            names.push(alias);
        }
        return ParsedImportItem { names, is_type };
    }

    // collect the primary name and alias
    names.push(current.to_string());
    if let Some(alias) = parse_alias(tokens) {
        names.push(alias);
    }

    // return parsed names and flags
    ParsedImportItem { names, is_type }
}

/// Parse an alias token sequence from remaining tokens.
fn parse_alias<'a>(mut tokens: impl Iterator<Item = &'a str>) -> Option<String> {
    // scan for an as keyword
    while let Some(token) = tokens.next() {
        if token == "as" {
            return tokens.next().map(str::to_string);
        }
    }

    // return none when no alias is present
    None
}

/// Clean a raw token by trimming punctuation.
fn clean_import_token(token: &str) -> String {
    // strip punctuation characters from both sides
    token
        .trim_matches(|c: char| matches!(c, '{' | '}' | ',' | ';'))
        .to_string()
}
