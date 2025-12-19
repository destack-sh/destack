use std::collections::HashMap;

use destack_dir::Expression;
use destack_source::{BatchEdit, Edit, FileEdit, FileId, Span};

use crate::Session;
use crate::query::common::{
    find_symbol_at_offset, get_canonical_symbol, get_dir_node_span, get_symbol_definition_span,
};

/// Result of a prepare rename query.
#[derive(Debug, Clone)]
pub struct PrepareRenameResult {
    /// The range of the symbol to rename.
    pub range: Span,
    /// The current name (placeholder for rename dialog).
    pub placeholder: String,
}

/// Result of a rename query.
#[derive(Debug, Clone)]
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

/// Check if the symbol at the given position can be renamed.
///
/// Returns the range and current name if renameable.
pub fn prepare_rename(session: &Session, file: FileId, offset: u32) -> Option<PrepareRenameResult> {
    // 1. find the symbol at offset
    let symbol_at = find_symbol_at_offset(session, file, offset)?;

    // 2. get canonical symbol and check if it's in our workspace
    let canonical_id = get_canonical_symbol(session, symbol_at.symbol_id);

    // get the symbol to check if it has a name
    let module = session.modules.get(canonical_id.module_id);
    let module = module.read();
    let (Some(ast), Some(dir)) = (&module.ast, &module.dir) else {
        return None;
    };
    let symbols = dir.symbols.read();
    let symbol = symbols.get_symbol(canonical_id.local_id);

    // get the symbol name
    let name_string_id = symbol.name()?;
    let name = ast.strings.get(name_string_id).to_string();

    // 3. return the range and current name
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
    // 1. find the symbol at offset
    let symbol_at = find_symbol_at_offset(session, file, offset)?;

    // 2. validate new_name is a valid identifier (basic check)
    if new_name.is_empty() || !is_valid_identifier(new_name) {
        return None;
    }

    // 3. get canonical symbol
    let canonical_id = get_canonical_symbol(session, symbol_at.symbol_id);

    // 4. collect all spans to rename, grouped by file
    let mut edits_by_file: HashMap<FileId, Vec<Span>> = HashMap::new();

    // add the definition
    if let Some(definition_span) = get_symbol_definition_span(session, canonical_id) {
        edits_by_file
            .entry(definition_span.file)
            .or_default()
            .push(definition_span);
    }

    // 5. find all references across all modules
    for module in session.modules.iter() {
        let module = module.read();
        let (Some(ast), Some(dir)) = (&module.ast, &module.dir) else {
            continue;
        };

        // collect matching expression ids first to avoid borrow issues
        let matching_expression_ids: Vec<_> = {
            let dir_tree = dir.tree.read();
            dir_tree
                .iter_nodes_of_type::<Expression>()
                .filter_map(|(expression_id, expression)| {
                    if let Some(target) = expression.target_symbol() {
                        let target_canonical = get_canonical_symbol(session, target);
                        if target_canonical == canonical_id {
                            return Some(expression_id);
                        }
                    }
                    None
                })
                .collect()
        };

        // get spans for each matching expression
        for expression_id in matching_expression_ids {
            if let Some(span) = get_dir_node_span(ast, dir, expression_id.into()) {
                edits_by_file.entry(span.file).or_default().push(span);
            }
        }
    }

    // 6. create BatchEdit from collected spans
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
