use std::collections::HashMap;

use destack_source::{BatchEdit, Edit, FileEdit, FileId, Span, Uri};
use serde::{Deserialize, Serialize};

use crate::Session;
use crate::query::common::{
    ReferenceCollectionOptions, collect_symbol_references_in_context, find_symbol_at_offset,
    get_canonical_symbol, get_symbol_definition_span, sort_and_dedup_spans, token_at_offset,
};

/// Result of a prepare rename query.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrepareRenameResult {
    /// The range of the symbol to rename.
    pub range: Span,
    /// The current name (placeholder for rename dialog).
    pub placeholder: String,
}

/// Result of a rename query.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RenameResult {
    /// All edits to apply.
    pub edits: BatchEdit,
}

impl RenameResult {
    /// Create an empty rename result.
    pub fn empty() -> Self {
        Self {
            edits: BatchEdit::new(),
        }
    }

    /// Create a rename result from a batch edit.
    pub fn from_edits(edits: BatchEdit) -> Self {
        Self { edits }
    }

    /// Whether there are any edits.
    pub fn is_empty(&self) -> bool {
        self.edits.is_empty()
    }

    /// Total number of edits.
    pub fn edit_count(&self) -> usize {
        self.edits.total_edits()
    }

    /// Number of files affected.
    pub fn file_count(&self) -> usize {
        self.edits.file_count()
    }
}

/// Request prepare rename at a cursor position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrepareRenameRequest {
    /// The document URI.
    pub uri: Uri,
    /// The byte offset in the document.
    pub offset: u32,
}

/// Response payload for prepare rename queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrepareRenameResponse {
    /// Prepare rename result, if available.
    pub result: Option<PrepareRenameResult>,
}

/// Request rename edits at a cursor position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RenameRequest {
    /// The document URI.
    pub uri: Uri,
    /// The byte offset in the document.
    pub offset: u32,
    /// The new name for the symbol.
    pub new_name: String,
}

/// Response payload for rename queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RenameResponse {
    /// Rename result, if available.
    pub result: Option<RenameResult>,
}

/// Check if the symbol at the given position can be renamed.
///
/// Returns the range and current name if renameable.
pub fn prepare_rename(session: &Session, file: FileId, offset: u32) -> Option<PrepareRenameResult> {
    // find the symbol at offset
    let symbol_at = find_symbol_at_offset(session, file, offset)?;

    // reject non modifier keywords at the cursor
    let token = token_at_offset(session, file, offset);
    if token.is_some_and(|token| is_keyword(&token) && !is_modifier_keyword(&token)) {
        return None;
    }

    // get canonical symbol and check if it's in our workspace
    let canonical_id = get_canonical_symbol(session, symbol_at.symbol_id);

    // get the symbol to check if it has a name
    let module = session.modules.get(canonical_id.module_id);
    let module = module.read();
    let ctx = session.query_context(&module)?;

    let symbols = ctx.symbols();
    let symbol = symbols.get_symbol(canonical_id.local_id);

    let name = if let Some(name_string_id) = symbol.name() {
        ctx.ast.strings.get(name_string_id).to_string()
    } else {
        let file = session.files.get(symbol_at.span.file);
        let content = match &file.content {
            destack_source::FileContent::Text { content } => content.as_str(),
            destack_source::FileContent::Json { content, .. } => content.as_str(),
            _ => return None,
        };
        let start = symbol_at.span.start as usize;
        let end = symbol_at.span.end as usize;
        let slice = content.get(start..end)?;
        slice.to_string()
    };

    // return the range and current name
    Some(PrepareRenameResult {
        range: symbol_at.span,
        placeholder: name,
    })
}

/// Rename the symbol at the given position.
///
/// Returns edits for all files that need to be modified.
pub fn rename(
    session: &Session,
    file: FileId,
    offset: u32,
    new_name: &str,
) -> Option<RenameResult> {
    // find the symbol at offset
    let symbol_at = find_symbol_at_offset(session, file, offset)?;

    // reject non modifier keywords at the cursor
    let token = token_at_offset(session, file, offset);
    if token.is_some_and(|token| is_keyword(&token) && !is_modifier_keyword(&token)) {
        return None;
    }

    // validate new_name is a valid identifier
    if new_name.is_empty() || !is_valid_identifier(new_name) {
        return None;
    }

    // get canonical symbol
    let canonical_id = get_canonical_symbol(session, symbol_at.symbol_id);

    // resolve the existing symbol name for span targeting
    let old_name = symbol_name(session, canonical_id, symbol_at.span)?;

    // collect all spans to rename, grouped by file
    let mut edits_by_file: HashMap<FileId, Vec<Span>> = HashMap::new();

    // add the definition
    if let Some(definition_span) = get_symbol_definition_span(session, canonical_id) {
        edits_by_file
            .entry(definition_span.file)
            .or_default()
            .push(definition_span);
    }

    // configure reference collection for rename behavior
    let reference_options = ReferenceCollectionOptions {
        include_expressions: true,
        include_members: true,
        include_dependencies: true,
        include_namespace_members: true,
        skip_dependency_aliases: true,
        use_dependency_name_spans: true,
        target_name: Some(&old_name),
        limit_to_file: None,
    };

    // collect references across all modules
    for module in session.modules.iter() {
        let module = module.read();
        let Some(ctx) = session.query_context(&module) else {
            continue;
        };

        // collect spans for this module and group them by file
        let spans =
            collect_symbol_references_in_context(session, &ctx, canonical_id, reference_options);
        for span in spans {
            edits_by_file.entry(span.file).or_default().push(span);
        }
    }

    // normalize span ordering and remove duplicates per file
    for spans in edits_by_file.values_mut() {
        sort_and_dedup_spans(spans);
    }

    // create BatchEdit from collected spans
    let mut batch_edit = BatchEdit::new();
    for (file_id, spans) in edits_by_file {
        let edits: Vec<Edit> = spans
            .into_iter()
            .map(|span| Edit::replace(span, new_name.to_string()))
            .collect();
        batch_edit.push(FileEdit::with_edits(file_id, edits));
    }

    Some(RenameResult::from_edits(batch_edit))
}

/// Check if a string is a valid identifier.
fn is_valid_identifier(name: &str) -> bool {
    let mut chars = name.chars();

    // first character must be letter or underscore
    match chars.next() {
        Some(c) if c.is_alphabetic() || c == '_' => {}
        _ => return false,
    }

    // remaining characters must be alphanumeric or underscore
    chars.all(|c| c.is_alphanumeric() || c == '_')
}

/// Resolve the symbol name to target within edits.
fn symbol_name(
    session: &Session,
    symbol_id: destack_dir::GlobalSymbolId,
    span: Span,
) -> Option<String> {
    // extract an identifier from the source span when possible
    let file = session.files.get(span.file);
    let content = match &file.content {
        destack_source::FileContent::Text { content } => content.as_str(),
        destack_source::FileContent::Json { content, .. } => content.as_str(),
        _ => "",
    };

    let start = span.start as usize;
    let end = span.end as usize;
    if let Some(slice) = content.get(start..end) {
        // collect identifier like tokens from the slice
        let tokens = slice
            .split(|c: char| !(c.is_alphanumeric() || c == '_'))
            .filter(|token| !token.is_empty());

        // skip common keywords and return the first remaining token
        for token in tokens {
            if is_keyword(token) {
                continue;
            }

            return Some(token.to_string());
        }
    }

    // fall back to the symbol name from the defining module
    let module = session.modules.get(symbol_id.module_id);
    let module = module.read();
    let ctx = session.query_context(&module)?;
    let symbols = ctx.symbols();
    let symbol = symbols.get_symbol(symbol_id.local_id);
    let name_id = symbol.name()?;

    Some(ctx.ast.strings.get(name_id).to_string())
}

/// Check whether a token is a common declaration keyword.
fn is_keyword(token: &str) -> bool {
    matches!(
        token,
        "export"
            | "function"
            | "class"
            | "struct"
            | "enum"
            | "interface"
            | "type"
            | "extension"
            | "namespace"
            | "const"
            | "let"
            | "var"
            | "async"
            | "static"
            | "return"
            | "if"
            | "else"
            | "for"
            | "while"
            | "match"
            | "break"
            | "continue"
    )
}

/// Check whether a keyword is a declaration modifier.
fn is_modifier_keyword(token: &str) -> bool {
    matches!(
        token,
        "export" | "declare" | "abstract" | "async" | "static"
    )
}
