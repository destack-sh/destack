use std::collections::HashMap;

use destack_dir as dir;
use destack_source::Span;

use crate::ModuleQueryContext;

/// One parsed documentation parameter entry.
#[derive(Debug, Clone)]
struct ParameterDoc {
    /// The documented parameter name.
    name: String,
    /// The accumulated parameter description.
    description: String,
}

/// One source comment viewed as documentation.
struct DocComment {
    /// The DIR comment entry.
    comment: dir::Comment,
}

impl ParameterDoc {
    /// Parse one `@param` documentation line.
    fn parse(line: &str) -> Option<Self> {
        // require a parameter tag
        let rest = line.strip_prefix("@param")?;
        let rest = parameter_tag_payload(rest.trim());

        // split the name from the description
        let mut parts = rest.splitn(2, |character: char| {
            character.is_whitespace() || character == '-'
        });
        let name = parts.next()?.trim();
        if name.is_empty() {
            return None;
        }

        // normalize the optional description text
        let description = parts
            .next()
            .unwrap_or("")
            .trim()
            .trim_start_matches('-')
            .trim()
            .to_string();

        Some(Self {
            name: name.to_string(),
            description,
        })
    }

    /// Append one continuation line.
    fn append_line(&mut self, line: &str) {
        // separate continuation text from the existing description
        if !self.description.is_empty() {
            self.description.push(' ');
        }

        self.description.push_str(line);
    }

    /// Insert this parameter into a documentation map.
    fn insert(self, docs: &mut HashMap<String, String>) {
        docs.insert(self.name, self.description.trim().to_string());
    }
}

impl DocComment {
    /// Create one documentation comment view.
    fn new(comment: dir::Comment) -> Self {
        Self { comment }
    }

    /// Return whether this comment is attached to one source span.
    fn is_attached_to(&self, span: Span) -> bool {
        self.comment.is_leading()
            && self.comment.span.file == span.file
            && self.comment.attached_to == span.start
    }

    /// Extract this normalized documentation string from source text.
    fn string(&self, source: &str) -> Option<String> {
        // convert checked source offsets into host indexes
        let start = usize::try_from(self.comment.span.start).unwrap_or_else(|_| {
            panic!(
                "comment start offset does not fit usize: {:?}",
                self.comment.span
            )
        });
        let end = usize::try_from(self.comment.span.end).unwrap_or_else(|_| {
            panic!(
                "comment end offset does not fit usize: {:?}",
                self.comment.span
            )
        });

        // read the comment source text
        let raw_comment = source
            .get(start..end)
            .unwrap_or_else(|| panic!("invalid comment source range {start}..{end}"));
        if !raw_comment.starts_with("///") && !raw_comment.starts_with("/**") {
            return None;
        }

        Some(dir::normalize_comment_payload(raw_comment).into_owned())
    }
}

impl ModuleQueryContext<'_> {
    /// Collect documentation strings attached to a source node.
    pub(crate) fn node_doc_strings(&self, source: &str, node_id: u32) -> Vec<String> {
        let node_span = self
            .source_index()
            .get_main(node_id)
            .unwrap_or_else(|| panic!("missing main source span for documented node {node_id}"));

        let mut doc_strings = Vec::new();

        // collect leading comments attached to this source node
        for comment in self.tree().comments().iter().copied() {
            let comment = DocComment::new(comment);
            if !comment.is_attached_to(node_span) {
                continue;
            }

            if let Some(doc_string) = comment.string(source) {
                doc_strings.push(doc_string);
            }
        }

        doc_strings
    }

    /// Collect documentation strings attached to a node or adjacent line docs.
    pub(crate) fn node_or_line_doc_strings(&self, source: &str, node_id: u32) -> Vec<String> {
        let mut doc_strings = self.node_doc_strings(source, node_id);

        // use adjacent line docs when no source docs are attached
        if doc_strings.is_empty() {
            let span = self.source_index().get_main(node_id).unwrap_or_else(|| {
                panic!("missing main source span for documented node {node_id}")
            });
            doc_strings = line_doc_strings_before_span(source, span.start);
        }

        doc_strings
    }

    /// Collect documentation strings from a node or enclosing nodes.
    pub(crate) fn node_or_enclosing_doc_strings(&self, source: &str, node_id: u32) -> Vec<String> {
        let mut doc_strings = self.node_doc_strings(source, node_id);

        // walk enclosing spans from innermost to outermost when the node has no docs
        if doc_strings.is_empty() {
            let span = self.source_index().get_main(node_id).unwrap_or_else(|| {
                panic!("missing main source span for documented node {node_id}")
            });
            let mut enclosing = self.source_index().get_enclosing_spans(
                self.file_id(),
                span.start,
                span.end.saturating_sub(1),
            );

            enclosing.sort_by_key(|entry| entry.length);
            for entry in enclosing {
                if entry.source_id == node_id {
                    continue;
                }

                doc_strings = self.node_doc_strings(source, entry.source_id);
                if !doc_strings.is_empty() {
                    break;
                }
            }
        }

        // use adjacent line docs when no source docs are attached
        if doc_strings.is_empty() {
            let span = self.source_index().get_main(node_id).unwrap_or_else(|| {
                panic!("missing main source span for documented node {node_id}")
            });
            doc_strings = line_doc_strings_before_span(source, span.start);
        }

        doc_strings
    }

    /// Join documentation strings for a symbol declaration or enclosing declaration nodes.
    pub(crate) fn symbol_doc_text(&self, symbol_id: dir::GlobalSymbolId) -> Option<String> {
        let symbol_module = self.module_context(symbol_id.module_id);

        let declaration = {
            let symbols = symbol_module.symbols();
            let symbol = symbols.get_symbol(symbol_id.local_id);
            symbol.declaration?
        };

        let view = symbol_module.view();
        if let Some(documentation) = view.get_documentation_any(declaration.local_id) {
            return Some(symbol_module.strings().get(documentation.text).to_string());
        }

        let source_node_id = view.get_source_any(declaration.local_id);
        let source_file = symbol_module.source_file();
        let source = source_file.text();
        let doc_strings = symbol_module.node_or_enclosing_doc_strings(source, source_node_id);
        if doc_strings.is_empty() {
            return None;
        }

        Some(doc_strings.join("\n\n"))
    }

    /// Join documentation strings with tag lines removed.
    pub(crate) fn node_doc_text_without_tags(
        &self,
        source: &str,
        node_id: u32,
        tags: &[&str],
    ) -> Option<String> {
        let doc_strings = self.node_or_line_doc_strings(source, node_id);

        doc_text_without_tags(doc_strings, tags)
    }
}

/// Collect line doc strings that immediately precede a declaration span.
pub(crate) fn line_doc_strings_before_span(source: &str, span_start: u32) -> Vec<String> {
    let mut lines = Vec::new();
    let bytes = source.as_bytes();
    let mut cursor = span_start.min(source.len() as u32) as usize;

    // find the start of the declaration line
    while cursor > 0 && bytes.get(cursor.saturating_sub(1)) != Some(&b'\n') {
        cursor = cursor.saturating_sub(1);
    }

    // walk contiguous line docs above the declaration
    while cursor > 0 {
        // locate the previous line range
        let line_end = cursor.saturating_sub(1);
        let mut line_start = line_end;
        while line_start > 0 && bytes.get(line_start.saturating_sub(1)) != Some(&b'\n') {
            line_start = line_start.saturating_sub(1);
        }

        // read and trim the previous line
        let line = source
            .get(line_start..line_end)
            .unwrap_or_else(|| panic!("invalid line doc range {line_start}..{line_end}"));
        let trimmed = line.trim();

        // allow leading blank lines before the first doc line only
        if trimmed.is_empty() {
            if lines.is_empty() {
                cursor = line_start;
                continue;
            }

            break;
        }

        // stop at the first non-doc line
        let Some(rest) = trimmed.strip_prefix("///") else {
            break;
        };

        // collect the doc payload
        lines.push(rest.trim().to_string());
        cursor = line_start;
    }

    // return no docs when no contiguous doc block was found
    if lines.is_empty() {
        return Vec::new();
    }

    // restore source order and group adjacent lines
    lines.reverse();
    vec![lines.join("\n")]
}

/// Parse `@param` tags from documentation text.
pub(crate) fn parse_parameter_docs(doc: &str) -> HashMap<String, String> {
    let mut docs = HashMap::new();
    let mut parameter: Option<ParameterDoc> = None;

    // scan documentation lines for param tags and continuations
    for raw_line in doc.lines() {
        let line = normalized_doc_line(raw_line);
        if let Some(next_parameter) = ParameterDoc::parse(line) {
            if let Some(parameter) = parameter.take() {
                parameter.insert(&mut docs);
            }

            parameter = Some(next_parameter);

            continue;
        }

        // close the current parameter when a different tag begins
        if line.starts_with('@') {
            if let Some(parameter) = parameter.take() {
                parameter.insert(&mut docs);
            }

            continue;
        }

        // append plain text as a continuation line
        if !line.is_empty() {
            if let Some(parameter) = parameter.as_mut() {
                parameter.append_line(line);
            }
        }
    }

    // flush the trailing parameter
    if let Some(parameter) = parameter {
        parameter.insert(&mut docs);
    }

    docs
}

/// Return the description payload after an optional typed parameter prefix.
fn parameter_tag_payload(rest: &str) -> &str {
    // strip jsdoc style type braces
    if let Some(after_brace) = rest.strip_prefix('{') {
        after_brace
            .find('}')
            .map(|index| after_brace[index + 1..].trim())
            .unwrap_or(rest)
    } else {
        rest
    }
}

/// Strip tag lines from a list of doc strings.
pub(crate) fn doc_text_without_tags(doc_strings: Vec<String>, tags: &[&str]) -> Option<String> {
    if doc_strings.is_empty() {
        return None;
    }

    let mut lines = Vec::new();

    // collect non-tag documentation lines
    for doc in doc_strings {
        for raw_line in doc.lines() {
            let line = normalized_doc_line(raw_line);
            if line.is_empty() || tags.iter().any(|tag| line.starts_with(tag)) {
                continue;
            }

            lines.push(line.to_string());
        }
    }

    // return nothing when every line was filtered out
    if lines.is_empty() {
        return None;
    }

    Some(lines.join("\n"))
}

/// Normalize a doc comment line for parsing.
fn normalized_doc_line(line: &str) -> &str {
    let mut line = line.trim();

    // strip block doc prefixes
    if let Some(stripped) = line.strip_prefix('*') {
        line = stripped.trim();
    }

    // strip line doc prefixes
    if let Some(stripped) = line.strip_prefix("///") {
        line = stripped.trim();
    }

    line
}
