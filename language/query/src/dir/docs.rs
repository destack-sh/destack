use std::collections::HashMap;

use destack_ast::normalize_comment_payload;
use destack_dir as dir;
use destack_workspace::{Repository, Revision};

use crate::core::{AstQueryContext, query_context};

/// Collect documentation strings attached to a node.
pub(crate) fn doc_strings_for_node(
    ast: AstQueryContext<'_>,
    source: &str,
    node_id: u32,
) -> Vec<String> {
    let node_span = ast.source_map().get_main_or_enclosing(node_id);

    ast.tree()
        .comments()
        .iter()
        .copied()
        .filter(|comment| comment.is_leading())
        .filter(|comment| comment.span.file == node_span.file)
        .filter(|comment| comment.attached_to == node_span.start)
        .filter_map(|comment| {
            let start = usize::try_from(comment.span.start).ok()?;
            let end = usize::try_from(comment.span.end).ok()?;
            let raw_comment = source.get(start..end)?;
            if !is_doc_comment(raw_comment) {
                return None;
            }

            Some(normalize_comment_payload(raw_comment).into_owned())
        })
        .collect()
}

/// Collect documentation strings attached to a node or immediate line docs.
pub(crate) fn doc_strings_for_node_with_fallback(
    ast: AstQueryContext<'_>,
    source: &str,
    node_id: u32,
) -> Vec<String> {
    // collect doc strings from AST
    let mut doc_strings = doc_strings_for_node(ast, source, node_id);

    // fall back to line docs when AST docs are missing
    if doc_strings.is_empty() {
        let span = ast.source_map().get_main_or_enclosing(node_id);
        doc_strings = line_doc_strings_before_span(source, span.start);
    }

    // return the collected docs
    doc_strings
}

/// Collect documentation strings from a node or enclosing nodes.
pub(crate) fn doc_strings_for_node_or_enclosing(
    ast: AstQueryContext<'_>,
    source: &str,
    node_id: u32,
) -> Vec<String> {
    // gather docs on the node or enclosing nodes
    let mut doc_strings = doc_strings_for_node(ast, source, node_id);

    // fall back to enclosing nodes when no docs are attached
    if doc_strings.is_empty() {
        let span = ast.source_map().get_main_or_enclosing(node_id);
        let mut enclosing = ast
            .source_map()
            .get_enclosing_spans(span.start, span.end.saturating_sub(1));

        // check innermost nodes first
        enclosing.sort_by_key(|entry| entry.length);
        for entry in enclosing {
            if entry.idx == node_id {
                continue;
            }
            doc_strings = doc_strings_for_node(ast, source, entry.idx);
            if !doc_strings.is_empty() {
                break;
            }
        }
    }

    // fall back to line docs from source when AST docs are missing
    if doc_strings.is_empty() {
        let span = ast.source_map().get_main_or_enclosing(node_id);
        doc_strings = line_doc_strings_before_span(source, span.start);
    }

    // return the collected docs
    doc_strings
}

/// Return whether one raw comment is documentation shaped.
fn is_doc_comment(raw_comment: &str) -> bool {
    raw_comment.starts_with("///") || raw_comment.starts_with("/**")
}

/// Join documentation strings for a symbol declaration or enclosing declaration nodes.
pub(crate) fn doc_text_for_symbol(
    repository: &Repository,
    revision: Revision,
    symbol_id: dir::GlobalSymbolId,
) -> Option<String> {
    // resolve the module query context
    let ctx = query_context(repository, revision, symbol_id.module_id)?;

    // resolve the symbol declaration
    let declaration = {
        let symbols = ctx.dir().symbols();
        let symbol = symbols.get_symbol(symbol_id.local_id);
        symbol.primary_declaration?
    };

    // prefer semantic documentation attached to the dir declaration
    let dir_tree = ctx.dir().tree();
    if let Some(documentation) = dir_tree.get_documentation(declaration.local_id.id) {
        return Some(ctx.dir().strings().get(documentation.text).to_string());
    }

    // resolve the source node for the declaration
    let ast_node_id = dir_tree.get_source(declaration.local_id.id);
    let source_file = repository.file(revision, ctx.file_id()).ok().flatten()?;
    let source = source_file.text();

    // collect docs from the declaration or its enclosing wrapper nodes
    let doc_strings = doc_strings_for_node_or_enclosing(ctx.ast(), source, ast_node_id);
    if doc_strings.is_empty() {
        return None;
    }

    Some(doc_strings.join("\n\n"))
}

/// Join documentation strings with tag lines removed.
pub(crate) fn doc_text_for_node_without_tags(
    ast: AstQueryContext<'_>,
    source: &str,
    node_id: u32,
    tags: &[&str],
) -> Option<String> {
    // collect doc strings with fallback handling
    let doc_strings = doc_strings_for_node_with_fallback(ast, source, node_id);

    // return the filtered doc text
    doc_text_without_tags(doc_strings, tags)
}

/// Collect line doc strings that immediately precede a declaration span.
pub(crate) fn line_doc_strings_before_span(source: &str, span_start: u32) -> Vec<String> {
    // initialize the line doc buffer
    let mut lines = Vec::new();

    // find the start of the line containing the span
    let bytes = source.as_bytes();
    let mut cursor = span_start.min(source.len() as u32) as usize;
    while cursor > 0 && bytes.get(cursor.saturating_sub(1)) != Some(&b'\n') {
        cursor = cursor.saturating_sub(1);
    }

    // walk backward over contiguous line docs
    while cursor > 0 {
        let line_end = cursor.saturating_sub(1);
        let mut line_start = line_end;
        while line_start > 0 && bytes.get(line_start.saturating_sub(1)) != Some(&b'\n') {
            line_start = line_start.saturating_sub(1);
        }
        // read and trim the current line
        let line = source.get(line_start..line_end).unwrap_or("");
        let trimmed = line.trim();

        if trimmed.is_empty() {
            if lines.is_empty() {
                cursor = line_start;
                continue;
            }
            break;
        }

        let Some(rest) = trimmed.strip_prefix("///") else {
            break;
        };

        lines.push(rest.trim().to_string());
        cursor = line_start;
    }

    if lines.is_empty() {
        return Vec::new();
    }

    lines.reverse();
    vec![lines.join("\n")]
}

/// Parse @param tags from documentation text.
pub(crate) fn parse_param_docs(doc: &str) -> HashMap<String, String> {
    // initialize parse state
    let mut result = HashMap::new();
    let mut current_param: Option<(String, String)> = None;

    // scan documentation lines for @param tags and continuations
    for raw_line in doc.lines() {
        // normalize leading doc markers
        let line = normalized_doc_line(raw_line);

        // start a new parameter entry when we see @param
        if let Some(rest) = line.strip_prefix("@param") {
            if let Some((name, desc)) = current_param.take() {
                result.insert(name, desc.trim().to_string());
            }
            let rest = rest.trim();

            // skip optional type annotations inside braces
            let rest = if let Some(after_brace) = rest.strip_prefix('{') {
                after_brace
                    .find('}')
                    .map(|i| after_brace[i + 1..].trim())
                    .unwrap_or(rest)
            } else {
                rest
            };

            // parse parameter name and initial description
            let mut parts = rest.splitn(2, |c: char| c.is_whitespace() || c == '-');
            if let Some(name) = parts.next() {
                let name = name.trim();
                if !name.is_empty() {
                    let desc = parts
                        .next()
                        .unwrap_or("")
                        .trim()
                        .trim_start_matches('-')
                        .trim();
                    current_param = Some((name.to_string(), desc.to_string()));
                }
            }

            continue;
        }

        // append continuation lines to the current parameter
        if let Some((name, desc)) = current_param.as_mut() {
            // append continuation lines for the current parameter
            if !line.is_empty() && !line.starts_with('@') {
                if !desc.is_empty() {
                    desc.push(' ');
                }
                desc.push_str(line);
            }
            // flush when a new tag begins
            else if line.starts_with('@') {
                result.insert(name.clone(), desc.trim().to_string());
                current_param = None;
            }
        }
    }

    // flush the last parameter entry
    if let Some((name, desc)) = current_param {
        result.insert(name, desc.trim().to_string());
    }

    result
}

/// Strip tag lines from a list of doc strings.
pub(crate) fn doc_text_without_tags(doc_strings: Vec<String>, tags: &[&str]) -> Option<String> {
    // return none when no docs are present
    if doc_strings.is_empty() {
        return None;
    }

    // filter out tag lines like @param and @returns
    let mut lines = Vec::new();
    for doc in doc_strings {
        for raw_line in doc.lines() {
            let line = normalized_doc_line(raw_line);

            if line.is_empty() || tags.iter().any(|tag| line.starts_with(tag)) {
                continue;
            }

            lines.push(line.to_string());
        }
    }

    // return none when no content remains
    if lines.is_empty() {
        return None;
    }

    // join the remaining lines
    Some(lines.join("\n"))
}

/// Normalize a doc comment line for parsing.
fn normalized_doc_line(line: &str) -> &str {
    let mut line = line.trim();
    if let Some(stripped) = line.strip_prefix('*') {
        line = stripped.trim();
    }
    if let Some(stripped) = line.strip_prefix("///") {
        line = stripped.trim();
    }
    line
}
