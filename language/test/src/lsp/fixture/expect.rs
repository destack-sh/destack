use std::collections::BTreeMap;

use crate::lsp::{
    ExpectedCodeAction, ExpectedCompletionItem, ExpectedDocumentSymbol, ExpectedWorkspaceSymbol,
    NormalizedCallHierarchyCall, NormalizedCodeLens, NormalizedDiagnostic, NormalizedDocumentLink,
    NormalizedFoldingRange, NormalizedHierarchyItem, NormalizedInlayHint, NormalizedLocation,
    NormalizedResolvedCompletionItem, NormalizedSelectionRange, NormalizedSemanticToken,
};

/// One parsed snapshot record with repeated key support.
#[derive(Debug, Clone, PartialEq, Eq)]
struct SnapshotRecord {
    /// The parsed key-value pairs for this record.
    fields: BTreeMap<String, Vec<String>>,
}

impl SnapshotRecord {
    /// Build one empty snapshot record.
    fn new() -> Self {
        Self {
            fields: BTreeMap::new(),
        }
    }

    /// Push one key-value pair into this record.
    fn push(&mut self, key: &str, value: &str) {
        self.fields
            .entry(key.to_string())
            .or_default()
            .push(value.to_string());
    }

    /// Take one required scalar field.
    fn take_required(&mut self, context: &str, key: &str) -> Result<String, String> {
        let Some(values) = self.fields.remove(key) else {
            return Err(format!("{context} snapshot is missing {key}= field"));
        };
        if values.len() != 1 {
            return Err(format!(
                "{context} snapshot expected exactly one {key}= field, got {}",
                values.len()
            ));
        }

        Ok(values.into_iter().next().unwrap_or_default())
    }

    /// Take one optional scalar field.
    fn take_optional(&mut self, context: &str, key: &str) -> Result<Option<String>, String> {
        let Some(values) = self.fields.remove(key) else {
            return Ok(None);
        };
        if values.len() != 1 {
            return Err(format!(
                "{context} snapshot expected at most one {key}= field, got {}",
                values.len()
            ));
        }

        Ok(values.into_iter().next())
    }

    /// Take one repeated field list.
    fn take_repeated(&mut self, key: &str) -> Vec<String> {
        self.fields.remove(key).unwrap_or_default()
    }

    /// Reject unknown fields after a record is consumed.
    fn finish(self, context: &str) -> Result<(), String> {
        if self.fields.is_empty() {
            return Ok(());
        }

        let keys = self.fields.keys().cloned().collect::<Vec<_>>().join(", ");
        Err(format!("{context} snapshot has unknown fields: {keys}"))
    }
}

/// Parse one exact diagnostic snapshot into normalized diagnostics.
pub fn parse_expected_diagnostics_snapshot(
    snapshot: &str,
) -> Result<Vec<NormalizedDiagnostic>, String> {
    let mut diagnostics = Vec::new();

    // parse one exact diagnostic per record
    for mut record in parse_snapshot_records(snapshot)? {
        let file_path = record.take_required("diagnostic", "file")?;
        let range = record.take_required("diagnostic", "range")?;
        let severity = record.take_required("diagnostic", "severity")?;
        let code = record.take_required("diagnostic", "code")?;
        let source = record.take_required("diagnostic", "source")?;
        let message = record.take_required("diagnostic", "message")?;
        record.finish("diagnostic")?;

        let (start_line, start_character, end_line, end_character) = parse_snapshot_range(&range)?;
        diagnostics.push(NormalizedDiagnostic {
            file_path,
            start_line,
            start_character,
            end_line,
            end_character,
            severity: parse_snapshot_severity(&severity)?,
            code: Some(code),
            source: Some(source),
            message,
        });
    }

    Ok(diagnostics)
}

/// Parse one exact selection-range snapshot.
pub fn parse_expected_selection_ranges_snapshot(
    snapshot: &str,
) -> Result<Vec<NormalizedSelectionRange>, String> {
    let mut ranges = Vec::new();

    // parse one exact selection chain per record
    for mut record in parse_snapshot_records(snapshot)? {
        let file_path = record.take_required("selection range", "file")?;
        let range = record.take_required("selection range", "range")?;
        let parents = record.take_repeated("parent");
        record.finish("selection range")?;

        let mut chain = vec![parse_zero_based_relative_location(&file_path, &range)?];
        for parent in parents {
            chain.push(parse_zero_based_relative_location(&file_path, &parent)?);
        }

        ranges.push(NormalizedSelectionRange { chain });
    }

    ranges.sort();

    Ok(ranges)
}

/// Parse an expected call hierarchy snapshot into exact normalized calls.
pub fn parse_expected_call_hierarchy_calls(
    source: &str,
) -> Result<Vec<NormalizedCallHierarchyCall>, String> {
    let mut calls = Vec::new();

    // parse one exact hierarchy edge group per record
    for mut record in parse_snapshot_records(source)? {
        let range = parse_snapshot_location(&record.take_required("call hierarchy", "item")?)?;
        let name = record.take_required("call hierarchy", "name")?;
        let kind = record.take_required("call hierarchy", "kind")?;
        let selection =
            parse_snapshot_location(&record.take_required("call hierarchy", "selection")?)?;
        let call_ranges = record
            .take_repeated("call")
            .into_iter()
            .map(|call| parse_snapshot_location(&call))
            .collect::<Result<Vec<_>, _>>()?;
        record.finish("call hierarchy")?;

        calls.push(NormalizedCallHierarchyCall {
            item: NormalizedHierarchyItem {
                name,
                kind: expected_symbol_kind_name(&kind)?,
                detail: None,
                range,
                selection_range: selection,
            },
            call_ranges,
        });
    }

    calls.sort();

    Ok(calls)
}

/// Parse an expected type hierarchy snapshot into exact normalized items.
pub fn parse_expected_type_hierarchy_items(
    source: &str,
) -> Result<Vec<NormalizedHierarchyItem>, String> {
    let mut items = Vec::new();

    // parse one exact hierarchy item per record
    for mut record in parse_snapshot_records(source)? {
        let range = parse_snapshot_location(&record.take_required("type hierarchy", "item")?)?;
        let name = record.take_required("type hierarchy", "name")?;
        let kind = record.take_required("type hierarchy", "kind")?;
        let selection =
            parse_snapshot_location(&record.take_required("type hierarchy", "selection")?)?;
        let detail = record.take_optional("type hierarchy", "detail")?;
        record.finish("type hierarchy")?;

        items.push(NormalizedHierarchyItem {
            name,
            kind: expected_symbol_kind_name(&kind)?,
            detail,
            range,
            selection_range: selection,
        });
    }

    items.sort();

    Ok(items)
}

/// Parse an expected semantic token snapshot into exact token entries.
pub fn parse_expected_semantic_tokens(
    source: &str,
) -> Result<Vec<NormalizedSemanticToken>, String> {
    let mut tokens = Vec::new();

    // parse one exact semantic token per record
    for mut record in parse_snapshot_records(source)? {
        let range = record.take_required("semantic token", "range")?;
        let text = record.take_required("semantic token", "text")?;
        let kind = record.take_required("semantic token", "kind")?;
        let modifiers = record.take_repeated("modifier");
        record.finish("semantic token")?;

        let (start, end) = range
            .split_once('-')
            .ok_or_else(|| format!("semantic token range is missing - delimiter: {range}"))?;
        let (start_line, start_character) = parse_line_character(start)?;
        let (end_line, end_character) = parse_line_character(end)?;

        tokens.push(NormalizedSemanticToken {
            text,
            token_type: kind,
            modifiers,
            start_line,
            start_character,
            end_line,
            end_character,
        });
    }

    Ok(tokens)
}

/// Parse one repeated document symbol block body.
pub fn parse_expected_document_symbols(
    source: &str,
) -> Result<Vec<ExpectedDocumentSymbol>, String> {
    let mut symbols = Vec::new();

    // parse one marker and kind entry per non-empty line
    for entry in source
        .lines()
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
    {
        let mut parts = entry.split('|');
        let marker_name = parts.next().unwrap_or_default().trim();
        let kind = parts.next().unwrap_or_default().trim();

        if marker_name.is_empty() || kind.is_empty() || parts.next().is_some() {
            return Err("expected document_symbol line as marker|kind".to_string());
        }

        symbols.push(ExpectedDocumentSymbol {
            marker_name: marker_name.to_string(),
            kind: kind.to_string(),
        });
    }

    Ok(symbols)
}

/// Parse one repeated workspace symbol block body.
pub fn parse_expected_workspace_symbols(
    source: &str,
) -> Result<Vec<ExpectedWorkspaceSymbol>, String> {
    let mut symbols = Vec::new();

    // parse one marker, kind, and optional container entry per non-empty line
    for entry in source
        .lines()
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
    {
        let parts = entry.split('|').map(str::trim).collect::<Vec<_>>();
        if !(2..=3).contains(&parts.len()) || parts.iter().any(|part| part.is_empty()) {
            return Err("expected workspace_symbol line as marker|kind|container?".to_string());
        }

        symbols.push(ExpectedWorkspaceSymbol {
            marker_name: parts[0].to_string(),
            kind: parts[1].to_string(),
            container_name: parts.get(2).map(|part| (*part).to_string()),
        });
    }

    Ok(symbols)
}

/// Parse one repeated completion item block body.
pub fn parse_expected_completion_items(
    source: &str,
) -> Result<Vec<ExpectedCompletionItem>, String> {
    let mut items = Vec::new();

    // parse one label and kind entry per non-empty line
    for entry in source
        .lines()
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
    {
        let mut parts = entry.split('|');
        let label = parts.next().unwrap_or_default().trim();
        let kind = parts.next().unwrap_or_default().trim();

        if label.is_empty() || kind.is_empty() || parts.next().is_some() {
            return Err("expected completion_item line as label|kind".to_string());
        }

        items.push(ExpectedCompletionItem {
            label: label.to_string(),
            kind: kind.to_string(),
        });
    }

    Ok(items)
}

/// Parse one repeated code action block body.
pub fn parse_expected_code_actions(source: &str) -> Result<Vec<ExpectedCodeAction>, String> {
    let mut actions = Vec::new();

    // parse one title, kind, preferred, and optional edit flag entry per non-empty line
    for entry in source
        .lines()
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
    {
        let parts = entry.split('|').map(str::trim).collect::<Vec<_>>();
        if !(3..=4).contains(&parts.len()) || parts[..3].iter().any(|part| part.is_empty()) {
            return Err("expected code_action line as title|kind|preferred|hasEdit?".to_string());
        }

        actions.push(ExpectedCodeAction {
            title: parts[0].to_string(),
            kind: parts[1].to_string(),
            is_preferred: parse_bool(parts[2], "code action preferred")?,
            has_edit: parts
                .get(3)
                .map(|value| parse_bool(value, "code action hasEdit"))
                .transpose()?
                .unwrap_or(true),
        });
    }

    Ok(actions)
}

/// Parse one resolved completion item snapshot.
pub fn parse_expected_resolved_completion(
    source: &str,
) -> Result<NormalizedResolvedCompletionItem, String> {
    let mut records = parse_snapshot_records(source)?;
    if records.len() != 1 {
        return Err(format!(
            "expected exactly one completion resolve record, got {}",
            records.len()
        ));
    }

    let mut record = records.remove(0);
    let label = record.take_required("completion resolve", "label")?;
    let documentation = record.take_optional("completion resolve", "documentation")?;
    record.finish("completion resolve")?;

    Ok(NormalizedResolvedCompletionItem {
        label,
        documentation,
    })
}

/// Parse one exact document-link snapshot into normalized links.
pub fn parse_expected_document_links(source: &str) -> Result<Vec<NormalizedDocumentLink>, String> {
    let mut links = Vec::new();

    // parse one exact document link per record
    for mut record in parse_snapshot_records(source)? {
        let range = record.take_required("document link", "range")?;
        let target = record.take_optional("document link", "target")?;
        let tooltip = record.take_optional("document link", "tooltip")?;
        record.finish("document link")?;

        let (start_line, start_character, end_line, end_character) = parse_snapshot_range(&range)?;
        links.push(NormalizedDocumentLink {
            start_line,
            start_character,
            end_line,
            end_character,
            target,
            tooltip,
        });
    }

    links.sort();

    Ok(links)
}

/// Parse one exact folding-range snapshot into normalized ranges.
pub fn parse_expected_folding_ranges(source: &str) -> Result<Vec<NormalizedFoldingRange>, String> {
    let mut ranges = Vec::new();

    // parse one exact folding range per record
    for mut record in parse_snapshot_records(source)? {
        let range = record.take_required("folding range", "range")?;
        let kind = record.take_optional("folding range", "kind")?;
        let collapsed_text = record.take_optional("folding range", "collapsed_text")?;
        record.finish("folding range")?;

        let (start_line, start_character, end_line, end_character) = parse_snapshot_range(&range)?;
        ranges.push(NormalizedFoldingRange {
            start_line,
            start_character,
            end_line,
            end_character,
            kind,
            collapsed_text,
        });
    }

    ranges.sort();

    Ok(ranges)
}

/// Parse one exact inlay-hint snapshot into normalized hints.
pub fn parse_expected_inlay_hints(source: &str) -> Result<Vec<NormalizedInlayHint>, String> {
    let mut hints = Vec::new();

    // parse one exact inlay hint per record
    for mut record in parse_snapshot_records(source)? {
        let position = record.take_required("inlay hint", "position")?;
        let label = record.take_required("inlay hint", "label")?;
        let kind = record.take_optional("inlay hint", "kind")?;
        let padding_left = record
            .take_optional("inlay hint", "padding_left")?
            .map(|value| parse_bool(&value, "inlay hint padding_left"))
            .transpose()?
            .unwrap_or(false);
        let padding_right = record
            .take_optional("inlay hint", "padding_right")?
            .map(|value| parse_bool(&value, "inlay hint padding_right"))
            .transpose()?
            .unwrap_or(false);
        record.finish("inlay hint")?;

        let (line, character) = parse_snapshot_position(&position)?;
        hints.push(NormalizedInlayHint {
            line,
            character,
            label,
            kind,
            padding_left,
            padding_right,
        });
    }

    hints.sort();

    Ok(hints)
}

/// Parse one exact code-lens snapshot into normalized lenses.
pub fn parse_expected_code_lenses(source: &str) -> Result<Vec<NormalizedCodeLens>, String> {
    let mut lenses = Vec::new();

    // parse one exact code lens per record
    for mut record in parse_snapshot_records(source)? {
        let range = record.take_required("code lens", "range")?;
        let title = record.take_optional("code lens", "title")?;
        let command = record.take_optional("code lens", "command")?;
        let arguments = record.take_repeated("arg");
        record.finish("code lens")?;

        let (start_line, start_character, end_line, end_character) = parse_snapshot_range(&range)?;
        lenses.push(NormalizedCodeLens {
            start_line,
            start_character,
            end_line,
            end_character,
            title,
            command,
            arguments,
        });
    }

    lenses.sort();

    Ok(lenses)
}

/// Parse one `file:line:character-line:character` location snapshot.
pub fn parse_snapshot_location(text: &str) -> Result<NormalizedLocation, String> {
    let (file_path, coordinates) = text
        .split_once(':')
        .ok_or_else(|| format!("snapshot location is missing file separator: {text}"))?;
    let (start_text, end_text) = coordinates
        .split_once('-')
        .ok_or_else(|| format!("snapshot location is missing range separator: {text}"))?;
    let (start_line, start_character) = parse_line_character(start_text)?;
    let (end_line, end_character) = parse_line_character(end_text)?;

    Ok(NormalizedLocation {
        file_path: file_path.to_string(),
        start_line,
        start_character,
        end_line,
        end_character,
    })
}

/// Normalize one expected symbol kind string into the harness label space.
pub fn expected_symbol_kind_name(kind: &str) -> Result<&'static str, String> {
    match kind {
        "file" => Ok("file"),
        "module" => Ok("module"),
        "namespace" => Ok("namespace"),
        "package" => Ok("package"),
        "class" => Ok("class"),
        "method" => Ok("method"),
        "property" => Ok("property"),
        "field" => Ok("field"),
        "constructor" => Ok("constructor"),
        "enum" => Ok("enum"),
        "interface" => Ok("interface"),
        "function" => Ok("function"),
        "variable" => Ok("variable"),
        "constant" => Ok("constant"),
        "string" => Ok("string"),
        "number" => Ok("number"),
        "boolean" => Ok("boolean"),
        "array" => Ok("array"),
        "object" => Ok("object"),
        "key" => Ok("key"),
        "null" => Ok("null"),
        "enum_member" => Ok("enum_member"),
        "struct" => Ok("struct"),
        "event" => Ok("event"),
        "operator" => Ok("operator"),
        "type_parameter" => Ok("type_parameter"),
        "type" => Ok("type"),
        _ => Err(format!("unsupported symbol kind {kind:?}")),
    }
}

/// Parse one blank-line-separated record snapshot.
fn parse_snapshot_records(source: &str) -> Result<Vec<SnapshotRecord>, String> {
    let mut records = Vec::new();
    let mut record = SnapshotRecord::new();

    // split records on blank lines
    for line in source.lines() {
        let line = line.trim();

        // finish the current record on blank lines
        if line.is_empty() {
            if !record.fields.is_empty() {
                records.push(record);
                record = SnapshotRecord::new();
            }

            continue;
        }

        let (key, value) = line
            .split_once('=')
            .ok_or_else(|| format!("snapshot record line is missing = separator: {line}"))?;
        let key = key.trim();
        if key.is_empty() {
            return Err(format!("snapshot record line is missing a key: {line}"));
        }

        // keep values exact except for surrounding record whitespace
        record.push(key, value.trim());
    }

    // finish the trailing record when present
    if !record.fields.is_empty() {
        records.push(record);
    }

    Ok(records)
}

/// Parse one required boolean value.
fn parse_bool(value: &str, label: &str) -> Result<bool, String> {
    match value.trim() {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(format!("expected {label} to be true or false")),
    }
}

/// Parse one exact diagnostic range snapshot.
fn parse_snapshot_range(range: &str) -> Result<(usize, usize, usize, usize), String> {
    let (start, end) = range
        .split_once('-')
        .ok_or_else(|| format!("expected diagnostic range start-end, got {range:?}"))?;
    let (start_line, start_character) = parse_snapshot_position(start)?;
    let (end_line, end_character) = parse_snapshot_position(end)?;

    Ok((start_line, start_character, end_line, end_character))
}

/// Parse one exact line and character position snapshot.
fn parse_snapshot_position(position: &str) -> Result<(usize, usize), String> {
    let (line, character) = position
        .split_once(':')
        .ok_or_else(|| format!("expected line:character position, got {position:?}"))?;
    let line = line
        .parse::<usize>()
        .map_err(|error| format!("invalid line {line:?}: {error}"))?;
    let character = character
        .parse::<usize>()
        .map_err(|error| format!("invalid character {character:?}: {error}"))?;

    Ok((line, character))
}

/// Parse one exact severity label snapshot.
fn parse_snapshot_severity(severity: &str) -> Result<Option<&'static str>, String> {
    match severity {
        "error" => Ok(Some("error")),
        "warning" => Ok(Some("warning")),
        "information" => Ok(Some("information")),
        "hint" => Ok(Some("hint")),
        "" => Ok(None),
        _ => Err(format!("unsupported diagnostic severity {severity:?}")),
    }
}

/// Parse one zero-based relative location using one shared file path.
fn parse_zero_based_relative_location(
    file_path: &str,
    location: &str,
) -> Result<NormalizedLocation, String> {
    let (start_text, end_text) = location
        .split_once('-')
        .ok_or_else(|| format!("expected snapshot range start-end, got {location:?}"))?;
    let (start_line, start_character) = parse_snapshot_position(start_text)?;
    let (end_line, end_character) = parse_snapshot_position(end_text)?;

    Ok(NormalizedLocation {
        file_path: file_path.to_string(),
        start_line,
        start_character,
        end_line,
        end_character,
    })
}

/// Parse one 1-based `line:character` pair into zero-based coordinates.
fn parse_line_character(text: &str) -> Result<(usize, usize), String> {
    let (line, character) = text
        .split_once(':')
        .ok_or_else(|| format!("invalid line:character pair: {text}"))?;
    let line = line
        .parse::<usize>()
        .map_err(|error| format!("invalid line number {line}: {error}"))?;
    let character = character
        .parse::<usize>()
        .map_err(|error| format!("invalid character number {character}: {error}"))?;
    let line = line
        .checked_sub(1)
        .ok_or_else(|| "line numbers must be 1-based".to_string())?;
    let character = character
        .checked_sub(1)
        .ok_or_else(|| "character numbers must be 1-based".to_string())?;

    Ok((line, character))
}

#[cfg(test)]
mod tests {
    use super::{
        parse_expected_call_hierarchy_calls, parse_expected_diagnostics_snapshot,
        parse_expected_selection_ranges_snapshot, parse_expected_semantic_tokens,
        parse_expected_type_hierarchy_items,
    };

    #[test]
    fn test_parse_diagnostics_snapshot_records() {
        let diagnostics = parse_expected_diagnostics_snapshot(
            "\
file=main.ds
range=0:21-0:22
severity=error
code=EP001
source=destack
message=parse error
",
        )
        .unwrap();

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].file_path, "main.ds");
        assert_eq!(diagnostics[0].message, "parse error");
    }

    #[test]
    fn test_parse_selection_range_snapshot_records() {
        let ranges = parse_expected_selection_ranges_snapshot(
            "\
file=main.ds
range=1:11-1:12
parent=1:11-1:16
parent=1:4-1:16
",
        )
        .unwrap();

        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges[0].chain.len(), 3);
        assert_eq!(ranges[0].chain[0].file_path, "main.ds");
    }

    #[test]
    fn test_parse_call_hierarchy_snapshot_records() {
        let calls = parse_expected_call_hierarchy_calls(
            "\
item=main.ds:2:1-3:2
name=caller
kind=function
selection=main.ds:2:10-2:16
call=main.ds:3:5-3:9
",
        )
        .unwrap();

        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].item.name, "caller");
        assert_eq!(calls[0].call_ranges.len(), 1);
    }

    #[test]
    fn test_parse_type_hierarchy_snapshot_records() {
        let items = parse_expected_type_hierarchy_items(
            "\
item=main.ds:1:1-1:12
name=Base
kind=class
selection=main.ds:1:7-1:11
",
        )
        .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "Base");
    }

    #[test]
    fn test_parse_semantic_tokens_snapshot_records() {
        let tokens = parse_expected_semantic_tokens(
            "\
range=1:10-1:15
text=greet
kind=function
modifier=declaration
",
        )
        .unwrap();

        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].text, "greet");
        assert_eq!(tokens[0].modifiers, vec!["declaration"]);
    }
}
